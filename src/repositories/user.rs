use std::sync::Arc;

use ahash::AHashSet;

use crate::{
    cache::{EntityCache, ServiceCache},
    config::Database,
    entity::User,
    error::AppResult,
    utils::{PasswordHasher, UsedIds},
};

/// A repository for managing user data within a db and cache.
///
/// This struct provides an interface to interact with user records, offering capabilities
/// to find, verify, and manage user information. It integrates both a db connection
/// and a caching layer to optimize data retrieval and reduce db load.
pub struct UserRepository {
    db: &'static Database,
    cache: &'static ServiceCache,
    hasher: PasswordHasher,
}

impl UsedIds for &'static UserRepository {
    async fn used_ids(&self) -> AppResult<AHashSet<i32>> {
        let conn = self.db.pg.get().await?;

        let user_ids_stmt = conn
            .prepare_cached(
                r#"
                    SELECT id FROM users
                "#,
            )
            .await?;

        let rows = conn.query(&user_ids_stmt, &[]).await?;
        let mut user_ids = AHashSet::with_capacity(rows.len());

        for row in rows {
            let id: i32 = row.get(0);
            user_ids.insert(id);
        }

        Ok(user_ids)
    }
}

impl UserRepository {
    /// Creates a new `UserRepository` instance.
    ///
    /// # Arguments
    /// - `db`: A reference to a `Database` connection.
    /// - `cache`: A reference to a `ServiceCache`.
    ///
    /// # Returns
    /// A new instance of `UserRepository`.
    pub fn new(db: &'static Database, cache: &'static ServiceCache) -> Self {
        let hasher = PasswordHasher::new(11);
        Self { cache, db, hasher }
    }

    /// Finds a user by ID.
    ///
    /// # Arguments
    /// - `id`: The user's ID.
    ///
    /// # Returns
    /// An `AppResult` containing the user if found, or `None`.
    pub async fn find(&self, id: i32) -> AppResult<Option<Arc<User>>> {
        if let Some(user) = self.cache.user.get(id) {
            return Ok(Some(user));
        };

        let row = {
            let conn = self.db.pg.get().await?;

            let find_user_stmt = conn
                .prepare_cached(
                    r#"
                        SELECT * FROM users
                        WHERE id = $1
                    "#,
                )
                .await?;

            conn.query_opt(&find_user_stmt, &[&id]).await?
        };

        match row {
            Some(ref row) => {
                let user = User::from_row(row);
                self.cache.user.set(user.clone());
                Ok(Some(user))
            }

            None => Ok(None),
        }
    }

    /// Finds a user by their email address.
    ///
    /// # Arguments
    /// - `email`: The email address of the user.
    ///
    /// # Returns
    /// An `AppResult` containing the user if found, or `None`.
    pub async fn find_by_email(&self, email: &str) -> AppResult<Option<Arc<User>>> {
        if let Some(user) = self.cache.user.get_by_email(email) {
            return Ok(Some(user));
        };

        let row = {
            let conn = self.db.pg.get().await?;

            let find_by_email_stmt = conn
                .prepare_cached(
                    r#"
                        SELECT * FROM users
                        WHERE email = $1
                    "#,
                )
                .await?;

            conn.query_opt(&find_by_email_stmt, &[&email]).await?
        };

        match row {
            Some(ref row) => {
                let user = User::from_row(row);
                self.cache.user.set(user.clone());
                Ok(Some(user))
            }

            None => Ok(None),
        }
    }

    /// Checks if a user exists by their email.
    ///
    /// # Arguments
    /// - `email`: The email address to check.
    ///
    /// # Returns
    /// `true` if the user exists, otherwise `false`.
    pub async fn user_exists(&self, email: &str) -> AppResult<bool> {
        if self.find_by_email(email).await?.is_some() {
            return Ok(true);
        };

        let row = {
            let conn = self.db.pg.get().await?;

            let user_exists_stmt = conn
                .prepare_cached(
                    r#"
                        SELECT EMAIL FROM users
                        WHERE email = $1
                    "#,
                )
                .await?;

            conn.query_opt(&user_exists_stmt, &[&email]).await?
        };

        Ok(row.is_some())
    }

    /// Retrieves the active status of a user.
    ///
    /// # Arguments
    /// - `id`: The user's ID.
    ///
    /// # Returns
    /// An optional boolean indicating the user's active status, or `None` if not found.
    pub async fn status(&self, id: i32) -> AppResult<Option<bool>> {
        let row = {
            let conn = self.db.pg.get().await?;

            let user_status_stmt = conn
                .prepare_cached(
                    r#"
                        SELECT active FROM users
                        WHERE id = $1
                    "#,
                )
                .await?;

            conn.query_opt(&user_status_stmt, &[&id]).await?
        };

        if let Some(row) = row {
            return Ok(Some(row.get(0)));
        }

        Ok(None)
    }

    #[inline]
    pub async fn hash_password(&self, password: String) -> AppResult<String> {
        self.hasher.hash_password(password).await
    }

    /// Validates a user's password against a stored hash.
    ///
    /// # Arguments
    /// - `password`: The password to validate.
    /// - `hash`: The hash to validate against.
    ///
    /// # Returns
    /// `true` if the password matches the hash, otherwise `false`.
    #[inline]
    pub async fn validate_password(&self, pwd: String, hash: String) -> AppResult<bool> {
        self.hasher.verify_password(hash, pwd).await
    }
}
