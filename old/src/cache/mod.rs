use std::sync::Arc;

use self::{championship::ChampionshipCache, token::TokenCache, user::UserCache};

mod championship;
mod token;
mod user;

/// Represents a caching layer for various entities using Redis.
///
/// This struct provides separate caches for users, championships, and tokens,
/// allowing for efficient data retrieval and storage.
///
/// # Examples
///
/// ```
/// let db = Database::connect(...); // Assume `Database::connect` is a method that returns a database connection.
/// let cache = ServiceCache::new(&db);
/// ```
pub struct ServiceCache {
    /// Cache for user-related data.
    pub user: UserCache,
    /// Cache for championship-related data.
    pub championship: ChampionshipCache,
    /// Cache for token-related data.
    pub token: TokenCache,
}

impl ServiceCache {
    /// Creates a new `ServiceCache` instance with caches initialized for users,
    /// championships, and tokens.
    ///
    /// # Arguments
    ///
    /// * `db` - A reference to a `Database` instance to be used for cache initialization.
    pub fn new() -> Self {
        Self {
            user: UserCache::new(),
            championship: ChampionshipCache::new(),
            token: TokenCache::new(),
        }
    }
}

/// A trait for caching entities, providing generic methods for getting, setting,
/// and deleting entities in the cache.
///
/// Implementors must specify the entity type and can override the default cache expiration.
pub trait EntityCache {
    /// The type of entity being cached.
    type Entity;

    // Retrieves an entity by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the entity to retrieve.
    ///
    /// # Returns
    ///
    /// A result containing an option with the entity if found, or a none if not.
    fn get(&self, id: i32) -> Option<Arc<Self::Entity>>;

    /// Stores an entity in the cache.
    ///
    /// # Arguments
    ///
    /// * `entity` - A reference to the entity to store.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    fn set(&self, entity: Arc<Self::Entity>);

    /// Removes an entity from the cache by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the entity to remove.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    fn delete(&self, id: i32);
}
