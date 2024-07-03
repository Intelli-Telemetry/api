use crate::{config::Database, structs::DatabasesStatus};

/// Manages access to server resources, including db connections.
///
/// This repository provides an interface for querying the status of various db pools
/// within the server, such as Redis and PostgreSQL. It's designed to facilitate the monitoring
/// and management of db connections and their operational status.
#[derive(Clone)]
pub struct ServerRepository {
    /// The db connection(s) managed by this repository.
    db: &'static Database,
}

impl ServerRepository {
    /// Constructs a new `ServerRepository` instance.
    ///
    /// Initializes the repository with a given db connection. This allows the repository
    /// to perform operations related to the db, such as checking the status of connection pools.
    ///
    /// # Arguments
    ///
    /// * `db` - A reference to an established `Database` connection.
    ///
    /// # Returns
    ///
    /// A new instance of `ServerRepository`.
    pub fn new(db: &'static Database) -> Self {
        Self { db }
    }

    /// Retrieves the status of active db pools.
    ///
    /// Queries the status of both Redis and PostgreSQL connection pools managed by the server.
    /// This method can be used to monitor the health and availability of the db resources.
    ///
    /// # Returns
    ///
    /// A `DatabasesStatus` struct containing the status information for the Redis and PostgreSQL pools.
    pub fn active_pools(&self) -> DatabasesStatus {
        let pg_pool = self.db.pg.status();
        DatabasesStatus { pg: pg_pool.into() }
    }
}
