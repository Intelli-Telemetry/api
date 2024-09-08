use chrono::{Duration, Utc};
use postgres_types::ToSql;
use tracing::info;

use crate::{
    cache::ServiceCache,
    config::Database,
    error::{AppResult, ChampionshipError, CommonError, UserError},
    repositories::{ChampionshipRepository, UserRepository},
    structs::{ChampionshipCreationData, ChampionshipUpdateData, ChampionshipUserAddForm},
    utils::{IdsGenerator, MachinePorts},
};

/// Manages championship-related operations.
pub struct ChampionshipService {
    db: &'static Database,
    cache: &'static ServiceCache,
    machine_ports: MachinePorts,
    user_repo: &'static UserRepository,
    championship_repo: &'static ChampionshipRepository,
    ids_generator: IdsGenerator,
}

impl ChampionshipService {
    /// Creates a new ChampionshipService instance.
    ///
    /// # Arguments
    /// - `db`: Database connection.
    /// - `cache`: Redis cache.
    /// - `user_repo`: User repository for database operations.
    /// - `championship_repo`: Championship repository for database operations.
    ///
    /// # Returns
    /// A new ChampionshipService instance or an error.
    pub async fn new(
        db: &'static Database,
        cache: &'static ServiceCache,
        user_repo: &'static UserRepository,
        championship_repo: &'static ChampionshipRepository,
    ) -> AppResult<Self> {
        let machine_ports = {
            let used_ports = championship_repo.ports_in_use().await?;
            MachinePorts::new(used_ports).await?
        };

        let ids_generator = {
            let used_ids = championship_repo.used_ids().await?;
            IdsGenerator::new(700000000..799999999, used_ids)
        };

        Ok(Self {
            db,
            cache,
            user_repo,
            championship_repo,
            machine_ports,
            ids_generator,
        })
    }

    /// Creates a new championship.
    ///
    /// # Arguments
    /// - `payload`: Championship creation data.
    /// - `user_id`: ID of the user creating the championship.
    ///
    /// # Returns
    /// Result indicating success or failure.
    pub async fn create(&self, payload: ChampionshipCreationData, user_id: i32) -> AppResult<()> {
        if self
            .championship_repo
            .find_by_name(&payload.name)
            .await?
            .is_some()
        {
            Err(ChampionshipError::AlreadyExists)?
        };

        let conn = self.db.pg.get().await?;

        let create_championship_stmt_fut = conn.prepare_cached(
            r#"
                    INSERT INTO championships (id, port, name, category, owner_id)
                    VALUES ($1,$2,$3,$4,$5)
                "#,
        );

        let relate_user_with_championship_stmt_fut = conn.prepare_cached(
            r#"
                    INSERT INTO championship_users (user_id, championship_id, role)
                    VALUES ($1,$2, 'Admin')
                "#,
        );

        let (create_championship_stmt, relate_user_with_championship_stmt) = tokio::try_join!(
            create_championship_stmt_fut,
            relate_user_with_championship_stmt_fut
        )?;

        let id = self.ids_generator.next();

        let port = self
            .machine_ports
            .next()
            .ok_or(ChampionshipError::NoPortsAvailable)?;

        let result = conn
            .execute(
                &create_championship_stmt,
                &[&id, &port, &payload.name, &payload.category, &user_id],
            )
            .await;

        if let Err(e) = result {
            self.machine_ports.return_port(port);
            return Err(e)?;
        }

        conn.execute_raw(&relate_user_with_championship_stmt, &[&user_id, &id])
            .await?;

        self.cache.championship.delete_by_user(user_id);
        Ok(())
    }

    /// Updates an existing championship.
    ///
    /// # Arguments
    /// - `id`: Championship ID.
    /// - `user_id`: ID of the user updating the championship.
    /// - `form`: Updated championship data.
    ///
    /// # Returns
    /// Result indicating success or failure.
    pub async fn update(
        &self,
        id: i32,
        user_id: i32,
        form: &ChampionshipUpdateData,
    ) -> AppResult<()> {
        // Scope to check if championship exists and if user is owner
        {
            let Some(championship) = self.championship_repo.find(id).await? else {
                Err(ChampionshipError::NotFound)?
            };

            if championship.owner_id != user_id {
                Err(ChampionshipError::NotOwner)?
            }

            if let Some(last_update) = championship.updated_at {
                if Utc::now().signed_duration_since(last_update) <= Duration::days(7) {
                    Err(ChampionshipError::IntervalNotReached)?
                };
            }
        }

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

            if clauses.is_empty() {
                Err(CommonError::NotValidUpdate)?
            }

            clauses.push("updated_at = CURRENT_TIMESTAMP".to_owned());

            let clause = clauses.join(", ");
            let query = format!(
                "UPDATE championships SET {} WHERE id = ${} AND owner_id = ${}",
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

    /// Adds a user to a championship.
    ///
    /// # Arguments
    /// - `id`: Championship ID.
    /// - `user_id`: ID of the user performing the operation.
    /// - `form`: Form containing the email and role of the user to add.
    ///
    /// # Returns
    /// Result indicating success or failure.
    pub async fn add_user(
        &self,
        id: i32,
        user_id: i32,
        form: ChampionshipUserAddForm,
    ) -> AppResult<()> {
        // Scope to check if championship exists and if user is owner
        {
            let Some(championship) = self.championship_repo.find(id).await? else {
                Err(ChampionshipError::NotFound)?
            };

            if championship.owner_id != user_id {
                Err(ChampionshipError::NotOwner)?
            }
        }

        let bind_user_id = {
            let Some(bind_user) = self.user_repo.find_by_email(&form.email).await? else {
                Err(UserError::NotFound)?
            };

            bind_user.id
        };

        let conn = self.db.pg.get().await?;

        let add_user_stmt = conn
            .prepare_cached(
                r#"
                    INSERT INTO championship_users (user_id, championship_id, role, team_id)
                    VALUES ($1,$2,$3,$4)
                "#,
            )
            .await?;

        let formatted_team_id = form.team_id.map(|valid_team_id| valid_team_id as i16);

        conn.execute(
            &add_user_stmt,
            &[&bind_user_id, &id, &form.role, &formatted_team_id],
        )
        .await?;
        self.cache.championship.delete_by_user(bind_user_id);
        Ok(())
    }

    /// Removes a user from a championship.
    ///
    /// # Arguments
    /// - `id`: Championship ID.
    /// - `user_id`: ID of the user performing the operation.
    /// - `remove_user_id`: ID of the user to remove.
    ///
    /// # Returns
    /// Result indicating success or failure.
    pub async fn remove_user(&self, id: i32, user_id: i32, remove_user_id: i32) -> AppResult<()> {
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
                    DELETE FROM championship_users WHERE user_id = $1 AND championship_id = $2
                "#,
            )
            .await?;

        conn.execute_raw(&remove_user_stmt, &[&remove_user_id, &id])
            .await?;
        self.cache.championship.delete_by_user(remove_user_id);

        Ok(())
    }

    /// Deletes a championship and all related user associations.
    ///
    /// # Arguments
    /// - `id`: Championship ID to delete.
    ///
    /// # Returns
    /// Result indicating success or failure.
    pub async fn delete(&self, id: i32) -> AppResult<()> {
        let conn = self.db.pg.get().await?;

        let delete_championship_relations_stmt_fut = conn.prepare_cached(
            r#"
                DELETE FROM championship_users WHERE championship_id = $1
            "#,
        );

        let delete_championship_stmt_fut = conn.prepare_cached(
            r#"
                DELETE FROM championships WHERE id = $1
            "#,
        );

        let (delete_championship_relations_stmt, delete_championship_stmt) = tokio::try_join!(
            delete_championship_relations_stmt_fut,
            delete_championship_stmt_fut
        )?;

        let users = self.championship_repo.users(id).await?;
        conn.execute_raw(&delete_championship_relations_stmt, &[&id])
            .await?;
        self.cache.championship.prune(id, users);

        conn.execute_raw(&delete_championship_stmt, &[&id]).await?;
        info!("Championship deleted with success: {id}");

        Ok(())
    }
}
