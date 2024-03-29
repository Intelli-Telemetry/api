use chrono::{Duration, Utc};
use postgres_types::ToSql;
use tracing::info;

use crate::{
    cache::RedisCache,
    config::Database,
    error::{AppResult, ChampionshipError, CommonError, UserError},
    repositories::{ChampionshipRepository, UserRepository},
    structs::{CreateChampionshipDto, UpdateChampionship},
    utils::{write, IdsGenerator, MachinePorts},
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
    cache: &'static RedisCache,
    /// A shared, thread-safe set of available ports for championships.
    ports: MachinePorts,
    /// Repository for user-specific db operations.
    user_repo: &'static UserRepository,
    /// Repository for championship-specific db operations.
    championship_repo: &'static ChampionshipRepository,
    /// Id generator for championship ids
    ids_generator: IdsGenerator<&'static ChampionshipRepository>,
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
        cache: &'static RedisCache,
        user_repo: &'static UserRepository,
        championship_repo: &'static ChampionshipRepository,
    ) -> AppResult<Self> {
        let machine_port = MachinePorts::new(championship_repo).await?;
        let ids_generator = IdsGenerator::new(700000000..799999999, championship_repo).await;

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

            let id = self.ids_generator.next().await;

            let port = self
                .ports
                .get()
                .ok_or(ChampionshipError::NoPortsAvailable)?;

            conn.execute(
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
            .await?;

            conn.execute(&relate_user_with_championship_stmt, &[&user_id, &id])
                .await?;
        }

        self.cache.championship.delete_by_user_id(user_id).await?;

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

            if Utc::now().signed_duration_since(championship.updated_at)
                <= Duration::try_days(7).unwrap()
            {
                Err(ChampionshipError::IntervalNotReached)?
            };

            if championship.owner_id != user_id {
                Err(ChampionshipError::NotOwner)?
            }
        }

        let (query, params) = {
            let mut counter = 1u8;
            let mut params: Vec<&(dyn ToSql + Sync)> = Vec::with_capacity(5);
            let mut query = String::from("UPDATE championship SET ");

            if let Some(name) = &form.name {
                write(&mut query, &mut counter, "name");
                params.push(name);
            }

            if let Some(category) = &form.category {
                write(&mut query, &mut counter, "category");
                params.push(category);
            }

            if let Some(season) = &form.season {
                write(&mut query, &mut counter, "season");
                params.push(season);
            }

            if counter == 1 {
                Err(CommonError::NotValidUpdate)?
            }

            write(&mut query, &mut counter, "WHERE id");
            params.push(&id);

            // Check if owner_id is the same as user_id
            write(&mut query, &mut counter, "AND owner_id");
            params.push(&user_id);

            (query, params)
        };

        // Scope to update championship
        {
            let conn = self.db.pg.get().await?;
            let update_championship_stmt = conn.prepare_cached(&query).await?;
            conn.execute(&update_championship_stmt, &params).await?;
        }

        let users = self.championship_repo.users(id).await?;
        self.cache.championship.delete_all(id, users).await?;

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

        let add_user_fut = async {
            let bindings: [&(dyn ToSql + Sync); 2] = [&new_user_id, &id];
            conn.execute(&add_user_stmt, &bindings).await?;
            Ok(())
        };

        let delete_user_cache_fut = self.cache.championship.delete_by_user_id(new_user_id);
        tokio::try_join!(add_user_fut, delete_user_cache_fut)?;
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

        let remove_user_fut = async {
            let bindings: [&(dyn ToSql + Sync); 2] = [&remove_user_id, &id];
            conn.execute(&remove_user_stmt, &bindings).await?;
            Ok(())
        };
        let remove_user_cache_fut = self.cache.championship.delete_by_user_id(remove_user_id);

        tokio::try_join!(remove_user_fut, remove_user_cache_fut)?;
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

        let bindings: [&(dyn ToSql + Sync); 1] = [&id];
        let (delete_championship_relations_stmt, delete_championship_stmt) = tokio::try_join!(
            delete_championship_relations_stmt_fut,
            delete_championship_stmt_fut
        )?;

        let users = self.championship_repo.users(id).await?;
        let remove_championship_users_fut = async {
            conn.execute(&delete_championship_relations_stmt, &bindings)
                .await?;
            Ok(())
        };
        let delete_champ_cache_fut = self.cache.championship.delete_all(id, users);

        tokio::try_join!(remove_championship_users_fut, delete_champ_cache_fut)?;
        conn.execute(&delete_championship_stmt, &bindings).await?;
        info!("Championship deleted with success: {id}");

        Ok(())
    }
}
