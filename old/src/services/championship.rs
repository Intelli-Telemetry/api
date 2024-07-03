use chrono::{Duration, Utc};
use postgres_types::ToSql;
use tracing::info;

use crate::{
    cache::ServiceCache,
    config::Database,
    error::{AppResult, ChampionshipError, CommonError, UserError},
    repositories::{ChampionshipRepository, UserRepository},
    structs::{CreateChampionshipDto, UpdateChampionship},
    utils::{IdsGenerator, MachinePorts},
};

/// Manages championship-related operations, including creation, update, and user management.
///
/// This service integrates db and cache operations to handle championships efficiently.
/// It supports creating new championships, updating existing ones, adding or removing users,
/// and deleting championships, all while managing port assignments for each championship.
pub struct ChampionshipService {
    /// Database connection for persistent storage of championship data.
    db: &'static Database,
    /// Redis cache for temporarily storing championship-related data.
    cache: &'static ServiceCache,
    /// A shared, thread-safe set of available ports for championships.
    ports: MachinePorts,
    /// Repository for user-specific db operations.
    user_repo: &'static UserRepository,
    /// Repository for championship-specific db operations.
    championship_repo: &'static ChampionshipRepository,
    /// Ids generator for championship ids
    ids_generator: IdsGenerator,
}

//TODO: Create a common trait for the entities and implement the common methods there
impl ChampionshipService {
    /// Creates a new instance of `ChampionshipService`.
    ///
    /// Initializes the service with db and cache connections, and prepares the set
    /// of available ports for use by new championships.
    ///
    /// # Arguments
    /// - `db`: A reference to the db connection.
    /// - `cache`: A reference to the Redis cache.
    ///
    /// # Returns
    /// A new `ChampionshipService` instance.
    pub async fn new(
        db: &'static Database,
        cache: &'static ServiceCache,
        user_repo: &'static UserRepository,
        championship_repo: &'static ChampionshipRepository,
    ) -> AppResult<Self> {
        let machine_port = MachinePorts::new(championship_repo).await?;

        let ids_generator = {
            let used_ids = championship_repo.used_ids().await?;
            IdsGenerator::new(700000000..799999999, used_ids)
        };

        Ok(Self {
            db,
            cache,
            user_repo,
            championship_repo,
            ports: machine_port,
            ids_generator,
        })
    }

    /// Creates a new championship with the specified details.
    ///
    /// Allocates a port for the new championship, ensures the championship name is unique,
    /// and stores the championship data in the db.
    ///
    /// # Arguments
    /// - `payload`: The details of the new championship to create.
    /// - `user_id`: The ID of the user creating the championship.
    ///
    /// # Returns
    /// An empty result indicating success or an error if the operation fails.
    pub async fn create(&self, payload: CreateChampionshipDto, user_id: i32) -> AppResult<()> {
        if self
            .championship_repo
            .find_by_name(&payload.name)
            .await?
            .is_some()
        {
            Err(ChampionshipError::AlreadyExists)?
        };

        {
            let conn = self.db.pg.get().await?;

            let create_championship_stmt_fut = conn.prepare_cached(
                r#"
                    INSERT INTO championship (id, port, name, category, season, owner_id)
                    VALUES ($1,$2,$3,$4,$5,$6)
                "#,
            );

            let relate_user_with_championship_stmt_fut = conn.prepare_cached(
                r#"
                    INSERT INTO user_championships (user_id, championship_id)
                    VALUES ($1,$2)
                "#,
            );

            let (create_championship_stmt, relate_user_with_championship_stmt) = tokio::try_join!(
                create_championship_stmt_fut,
                relate_user_with_championship_stmt_fut
            )?;

            let id = self.ids_generator.next();

            let port = self
                .ports
                .next()
                .ok_or(ChampionshipError::NoPortsAvailable)?;

            let result = conn
                .execute(
                    &create_championship_stmt,
                    &[
                        &id,
                        &port,
                        &payload.name,
                        &payload.category,
                        &payload.season,
                        &user_id,
                    ],
                )
                .await;

            if let Err(e) = result {
                self.ports.return_port(port);
                return Err(e.into());
            }

            conn.execute(&relate_user_with_championship_stmt, &[&user_id, &id])
                .await?;
        }

        self.cache.championship.delete_by_user(user_id);
        Ok(())
    }

    /// Updates an existing championship with the given details.
    ///
    /// Validates user ownership and update interval before applying changes to the championship.
    ///
    /// # Arguments
    /// - `id`: The ID of the championship to update.
    /// - `user_id`: The ID of the user requesting the update.
    /// - `form`: The new details to apply to the championship.
    ///
    /// # Returns
    /// An empty result indicating success or an error if the operation fails.
    pub async fn update(&self, id: i32, user_id: i32, form: &UpdateChampionship) -> AppResult<()> {
        // Scope to check if championship exists and if user is owner
        {
            let Some(championship) = self.championship_repo.find(id).await? else {
                Err(ChampionshipError::NotFound)?
            };

            if championship.owner_id != user_id {
                Err(ChampionshipError::NotOwner)?
            }

            if Utc::now().signed_duration_since(championship.updated_at)
                <= Duration::try_days(7).unwrap()
            {
                Err(ChampionshipError::IntervalNotReached)?
            };
        }

        // Todo: Check security in building this query on runtime & Improve performance
        let (query, params) = {
            let mut params_counter = 1u8;
            let mut clauses = Vec::with_capacity(3);
            let mut params: Vec<&(dyn ToSql + Sync)> = Vec::with_capacity(5);

            if let Some(name) = &form.name {
                clauses.push(format!("name = ${}", params_counter));
                params.push(name);
                params_counter += 1;
            }

            if let Some(category) = &form.category {
                clauses.push(format!("category = ${}", params_counter));
                params.push(category);
                params_counter += 1;
            }

            if let Some(season) = &form.season {
                clauses.push(format!("season = ${}", params_counter));
                params.push(season);
                params_counter += 1;
            }

            if clauses.is_empty() {
                Err(CommonError::NotValidUpdate)?
            }

            let clause = clauses.join(", ");
            let query = format!(
                "UPDATE championship SET {} WHERE id = ${} AND owner_id = ${}",
                clause,
                params_counter,
                params_counter + 1
            );

            params.push(&id);
            params.push(&user_id);

            (query, params)
        };

        // Scope to update championship
        {
            let conn = self.db.pg.get().await?;
            conn.execute(&query, &params).await?;
        }

        let users = self.championship_repo.users(id).await?;
        self.cache.championship.prune(id, users);

        Ok(())
    }

    /// Adds a user to a championship, allowing them to participate or manage it.
    ///
    /// Validates championship existence and user ownership before adding the specified user.
    ///
    /// # Arguments
    /// - `id`: The ID of the championship.
    /// - `user_id`: The ID of the user performing the operation.
    /// - `bind_user_email`: The email of the user to add to the championship.
    ///
    /// # Returns
    /// An empty result indicating success or an error if the operation fails.
    pub async fn add_user(&self, id: i32, user_id: i32, bind_user_email: &str) -> AppResult<()> {
        // Scope to check if championship exists and if user is owner
        {
            let Some(championship) = self.championship_repo.find(id).await? else {
                Err(ChampionshipError::NotFound)?
            };

            if championship.owner_id != user_id {
                Err(ChampionshipError::NotOwner)?
            }
        }

        let new_user_id = {
            let Some(bind_user) = self.user_repo.find_by_email(bind_user_email).await? else {
                Err(UserError::NotFound)?
            };

            bind_user.id
        };

        let conn = self.db.pg.get().await?;

        let add_user_stmt = conn
            .prepare_cached(
                r#"
                    INSERT INTO user_championships (user_id, championship_id)
                    VALUES ($1,$2)
                "#,
            )
            .await?;

        conn.execute(&add_user_stmt, &[&new_user_id, &id]).await?;
        self.cache.championship.delete_by_user(new_user_id);
        Ok(())
    }

    /// Removes a user from a championship.
    ///
    /// Validates championship existence, user ownership, and that the user to remove is not the owner.
    ///
    /// # Arguments
    /// - `id`: The ID of the championship.
    /// - `user_id`: The ID of the user performing the operation.
    /// - `remove_user_id`: The ID of the user to remove from the championship.
    ///
    /// # Returns
    /// An empty result indicating success or an error if the operation fails.
    pub async fn remove_user(&self, id: i32, user_id: i32, remove_user_id: i32) -> AppResult<()> {
        // Scope to check if championship exists and if user is owner
        {
            let Some(championship) = self.championship_repo.find(id).await? else {
                Err(ChampionshipError::NotFound)?
            };

            if championship.owner_id != user_id {
                Err(ChampionshipError::NotOwner)?
            }

            if championship.owner_id == remove_user_id {
                Err(ChampionshipError::CannotRemoveOwner)?
            }
        }

        if self.user_repo.find(remove_user_id).await?.is_none() {
            Err(UserError::NotFound)?
        };

        let conn = self.db.pg.get().await?;

        let remove_user_stmt = conn
            .prepare_cached(
                r#"
                    DELETE FROM user_championships WHERE user_id = $1 AND championship_id = $2
                "#,
            )
            .await?;

        conn.execute(&remove_user_stmt, &[&remove_user_id, &id])
            .await?;
        self.cache.championship.delete_by_user(remove_user_id);

        Ok(())
    }

    /// Deletes a championship and all related user associations.
    ///
    /// Validates championship existence before deleting it from the db and cache.
    ///
    /// # Arguments
    /// - `id`: The ID of the championship to delete.
    ///
    /// # Returns
    /// An empty result indicating success or an error if the operation fails.
    pub async fn delete(&self, id: i32) -> AppResult<()> {
        let conn = self.db.pg.get().await?;

        let delete_championship_relations_stmt_fut = conn.prepare_cached(
            r#"
                DELETE FROM user_championships WHERE championship_id = $1
            "#,
        );

        let delete_championship_stmt_fut = conn.prepare_cached(
            r#"
                DELETE FROM championship WHERE id = $1
            "#,
        );

        let (delete_championship_relations_stmt, delete_championship_stmt) = tokio::try_join!(
            delete_championship_relations_stmt_fut,
            delete_championship_stmt_fut
        )?;

        let users = self.championship_repo.users(id).await?;
        conn.execute(&delete_championship_relations_stmt, &[&id])
            .await?;
        self.cache.championship.prune(id, users);

        conn.execute(&delete_championship_stmt, &[&id]).await?;
        info!("Championship deleted with success: {id}");

        Ok(())
    }
}
