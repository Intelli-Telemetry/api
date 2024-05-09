use std::{ops::DerefMut, str::FromStr};

use deadpool_postgres::{
    tokio_postgres::{Config, NoTls},
    Manager, Pool,
};
use dotenvy::var;
use refinery::embed_migrations;
use tracing::info;

embed_migrations!("migrations");

/// Represents the application's database connections, encapsulating both Redis and Postgres pools.
pub struct Database {
    /// The PostgreSQL connection pool for database operations.
    pub pg: Pool,
}

impl Database {
    /// Initializes and returns a new `Database` instance by setting up connection to Postgres databases.
    ///
    /// It reads database URL from the environment, creates connection pools for POstgres,
    /// and performs database migrations.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The environment variable `DATABASE_URL` is not set.
    /// - It fails to create the connection pools.
    /// - It fails to run database migrations.
    ///
    /// # Examples
    /// ```
    /// let database = Database::new().await;
    /// ```
    pub async fn new() -> Self {
        info!("Connecting Databases...");

        // Postgres connection
        let pg = {
            let config =
                Config::from_str(&var("DATABASE_URL").expect("Environment DATABASE_URL not found"))
                    .unwrap();

            let manager = Manager::new(config, NoTls);
            Pool::builder(manager).max_size(200).build().unwrap()
        };

        Self::migrations(&pg).await;

        Self { pg }
    }

    /// Executes database migrations for the Postgres database.
    ///
    /// This is an internal function called during the initialization of the `Database` struct.
    ///
    /// # Panics
    /// This function will panic if it fails to acquire a connection from the pool or to run the migrations.
    #[inline]
    async fn migrations(pg: &Pool) {
        let mut conn = pg.get().await.unwrap();
        let client = conn.deref_mut().deref_mut();
        migrations::runner().run_async(client).await.unwrap();
    }
}
