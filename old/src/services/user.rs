use chrono::{Duration, Utc};
use postgres_types::ToSql;
use tracing::{error, info};

use crate::{
    cache::{EntityCache, ServiceCache},
    config::Database,
    entity::{Provider, UserExtension},
    error::{AppResult, TokenError, UserError},
    repositories::UserRepository,
    structs::{RegisterUserDto, TokenType, UpdateUser},
    utils::IdsGenerator,
};

use super::TokenService;

/// A service for managing user accounts.
///
/// This service provides functionality to create, update, delete, and manage the state of user
/// accounts. It integrates db operations with caching for improved performance and
/// efficiency, leveraging both a direct db connection and a Redis cache.
pub struct UserService {
    /// Redis cache for temporarily storing user data.
    cache: &'static ServiceCache,
    /// Database connection for persistent storage.
    db: &'static Database,
    /// Repository for user-specific db operations.
    user_repo: &'static UserRepository,
    /// Service for managing authentication tokens.
    token_svc: &'static TokenService,
    /// Service for generating unique IDs.
    ids_generator: IdsGenerator,
}

impl UserService {
    /// Constructs a new instance of the user service.
    ///
    /// # Arguments
    /// - `db`: A reference to the db connection.
    /// - `cache`: A reference to the Redis cache.
    ///
    /// # Returns
    /// A new `UserService` instance.
    pub async fn new(
        db: &'static Database,
        cache: &'static ServiceCache,
        user_repo: &'static UserRepository,
        token_svc: &'static TokenService,
    ) -> Self {
        let ids_generator = {
            let used_ids = user_repo.used_ids().await.unwrap();
            IdsGenerator::new(600000000..699999999, used_ids)
        };

        Self {
            cache,
            db,
            token_svc,
            user_repo,
            ids_generator,
        }
    }

    /// Creates a new user account.
    ///
    /// # Arguments
    /// - `register`: Data transfer object containing user registration details.
    ///
    /// # Returns
    /// The ID of the newly created user if successful.
    pub async fn create(&self, register: &RegisterUserDto) -> AppResult<i32> {
        let user_exists = self.user_repo.user_exists(&register.email).await?;

        if user_exists {
            Err(UserError::AlreadyExists)?
        }

        let id = self.ids_generator.next();
        let conn = self.db.pg.get().await?;

        match &register.provider {
            Some(provider) if provider == &Provider::Google => {
                let create_user_stmt = conn
                    .prepare_cached(
                        r#"
                            INSERT INTO users (id, email, username, avatar, provider, active)
                            VALUES ($1,$2,$3,$4,$5, true)
                        "#,
                    )
                    .await?;

                conn.execute(
                    &create_user_stmt,
                    &[
                        &id,
                        &register.email,
                        &register.username,
                        &register.avatar,
                        provider,
                    ],
                )
                .await?;
            }

            None => {
                let hashed_password = self
                    .user_repo
                    .hash_password(register.password.clone().unwrap())
                    .await?;

                let create_google_user_stmt = conn
                    .prepare_cached(
                        r#"
                            INSERT INTO users (id, email, username, password, avatar, active)
                            VALUES ($1,$2,$3,$4,$5, false)
                        "#,
                    )
                    .await?;

                conn.execute(
                    &create_google_user_stmt,
                    &[
                        &id,
                        &register.email,
                        &register.username,
                        &hashed_password,
                        &format!("https://ui-avatars.com/api/?name={}", &register.username),
                    ],
                )
                .await?;
            }

            _ => Err(UserError::InvalidProvider)?,
        }

        info!("User created: {}", register.username);
        Ok(id)
    }

    /// Updates an existing user account.
    ///
    /// # Arguments
    /// - `user`: Current state of the user to be updated.
    /// - `form`: Data transfer object containing updated user details.
    ///
    /// # Returns
    /// An empty result indicating success or failure.
    pub async fn update(&self, user: &UserExtension, form: &UpdateUser) -> AppResult<()> {
        if Utc::now().signed_duration_since(user.updated_at) <= Duration::try_days(7).unwrap() {
            Err(UserError::UpdateLimitExceeded)?
        }

        // Todo: Check security in building this query on runtime & Improve performance
        let (query, params) = {
            let mut params_counter = 1u8;
            let mut clauses = Vec::with_capacity(2);
            let mut params: Vec<&(dyn ToSql + Sync)> = Vec::with_capacity(3);

            if let Some(username) = &form.username {
                clauses.push(format!("name = ${}", params_counter));
                params.push(username);
                params_counter += 1;
            }

            if let Some(avatar) = &form.avatar {
                clauses.push(format!("avatar = ${}", params_counter));
                params.push(avatar);
                params_counter += 1;
            }

            if clauses.is_empty() {
                Err(UserError::InvalidUpdate)?
            }

            let clause = clauses.join(", ");
            let query = format!("UPDATE users {} WHERE id = ${}", clause, params_counter);
            params.push(&user.id);

            (query, params)
        };

        let conn = self.db.pg.get().await?;
        conn.execute(&query, &params[..]).await?;
        self.cache.user.delete(user.id);

        Ok(())
    }

    /// Deletes a user account by ID.
    ///
    /// # Arguments
    /// - `id`: The ID of the user to delete.
    ///
    /// # Returns
    /// An empty result indicating success or failure.
    // Todo: Create a column "deleted" in users table and update it instead of delete
    pub async fn delete(&self, id: i32) -> AppResult<()> {
        let conn = self.db.pg.get().await?;

        let delete_users_relations_stmt_fut = conn.prepare_cached(
            r#"
                DELETE FROM user_championships
                WHERE user_id = $1
            "#,
        );

        let delete_user_stmt_fut = conn.prepare_cached(
            r#"
                DELETE FROM users
                WHERE id = $1
            "#,
        );

        let (delete_users_relations_stmt, delete_user_stmt) =
            tokio::try_join!(delete_users_relations_stmt_fut, delete_user_stmt_fut)?;

        let binding: [&(dyn ToSql + Sync); 1] = [&id];
        conn.execute(&delete_users_relations_stmt, &binding).await?;

        conn.execute(&delete_user_stmt, &binding).await?;
        self.cache.user.delete(id);

        info!("User deleted with success: {}", id);

        Ok(())
    }

    /// Resets the password for a user account using a token.
    ///
    /// # Arguments
    /// - `token`: A token validating the reset request.
    /// - `password`: The new password.
    ///
    /// # Returns
    /// The ID of the user whose password was reset if successful.
    pub async fn reset_password_with_token(
        &self,
        token: String,
        password: String,
    ) -> AppResult<i32> {
        self.cache
            .token
            .get_token(token.clone(), TokenType::ResetPassword);

        let user_id = {
            let token_data = self.token_svc.validate(&token)?;
            if token_data.claims.token_type != TokenType::ResetPassword {
                error!("Token type is not ResetPassword");
                Err(TokenError::InvalidToken)?
            }

            token_data.claims.sub
        };

        self.reset_password(user_id, password).await?;
        self.cache
            .token
            .remove_token(token, TokenType::ResetPassword);

        Ok(user_id)
    }

    /// Activates a user account.
    ///
    /// # Arguments
    /// - `id`: The ID of the user to activate.
    ///
    /// # Returns
    /// An empty result indicating success or failure.
    pub async fn activate(&self, id: i32) -> AppResult<()> {
        let conn = self.db.pg.get().await?;

        let activate_user_stmt = conn
            .prepare_cached(
                r#"
                    UPDATE users
                    SET active = true
                    WHERE id = $1
                "#,
            )
            .await?;

        conn.execute(&activate_user_stmt, &[&id]).await?;
        self.cache.user.delete(id);

        info!("User activated with success: {}", id);

        Ok(())
    }

    /// Activates a user account using a token.
    ///
    /// # Arguments
    /// - `token`: A token validating the activation request.
    ///
    /// # Returns
    /// The ID of the user activated if successful.
    pub async fn activate_with_token(&self, token: String) -> AppResult<i32> {
        self.cache.token.get_token(token.clone(), TokenType::Email);

        let user_id = {
            let token_data = self.token_svc.validate(&token)?;
            if token_data.claims.token_type != TokenType::Email {
                Err(TokenError::InvalidToken)?
            }

            token_data.claims.sub
        };

        self.activate(user_id).await?;
        self.cache.token.remove_token(token, TokenType::Email);

        Ok(user_id)
    }

    /// Deactivates a user account.
    ///
    /// # Arguments
    /// - `id`: The ID of the user to deactivate.
    ///
    /// # Returns
    /// An empty result indicating success or failure.
    pub async fn deactivate(&self, id: i32) -> AppResult<()> {
        let conn = self.db.pg.get().await?;
        let deactivate_user_stmt = conn
            .prepare_cached(
                r#"
                    UPDATE users
                    SET active = false
                    WHERE id = $1
                "#,
            )
            .await?;

        conn.execute(&deactivate_user_stmt, &[&id]).await?;
        self.cache.user.delete(id);

        info!("User activated with success: {}", id);
        Ok(())
    }

    /// Resets the password for a user account.
    ///
    /// # Arguments
    /// - `id`: The ID of the user whose password is to be reset.
    /// - `password`: The new password.
    ///
    /// # Returns
    /// An empty result indicating success or failure.
    async fn reset_password(&self, id: i32, password: String) -> AppResult<()> {
        let Some(user) = self.user_repo.find(id).await? else {
            Err(UserError::NotFound)?
        };

        if Utc::now().signed_duration_since(user.updated_at) <= Duration::try_minutes(15).unwrap() {
            Err(UserError::UpdateLimitExceeded)?
        }

        let conn = self.db.pg.get().await?;
        let reset_password_stmt = conn
            .prepare_cached(
                r#"
                    UPDATE users
                    SET password = $1
                    WHERE id = $2
                "#,
            )
            .await?;

        let hashed_password = self.user_repo.hash_password(password).await?;
        conn.execute(&reset_password_stmt, &[&hashed_password, &id])
            .await?;

        self.cache.user.delete(id);

        info!("User password reseated with success: {}", id);
        Ok(())
    }
}
