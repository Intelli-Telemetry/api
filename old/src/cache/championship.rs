use std::sync::Arc;

use quick_cache::sync::Cache;

use crate::entity::Championship;

use super::EntityCache;

pub struct ChampionshipCache {
    cache: Cache<i32, Arc<Championship>>,
    name_to_id: Cache<String, i32>,
    user_championships: Cache<i32, Vec<Arc<Championship>>>,
}

impl ChampionshipCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::new(10_000),
            name_to_id: Cache::new(10_000),
            user_championships: Cache::new(10_000),
        }
    }

    pub fn get_user_championships(&self, user_id: i32) -> Option<Vec<Arc<Championship>>> {
        self.user_championships.get(&user_id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<Arc<Championship>> {
        if let Some(id) = self.name_to_id.get(name) {
            return self.get(id);
        }

        None
    }

    pub fn set_user_championships(&self, user_id: i32, championships: Vec<Arc<Championship>>) {
        self.user_championships.insert(user_id, championships);
    }

    #[inline]
    pub fn delete_by_user(&self, user_id: i32) {
        self.user_championships.remove(&user_id);
    }

    pub fn prune(&self, id: i32, users: Vec<i32>) {
        for user in users {
            self.delete_by_user(user);
        }

        self.delete(id);
    }
}

impl EntityCache for ChampionshipCache {
    type Entity = Championship;

    fn get(&self, id: i32) -> Option<Arc<Self::Entity>> {
        self.cache.get(&id)
    }

    fn set(&self, entity: Arc<Self::Entity>) {
        self.cache.insert(entity.id, entity.clone());
        self.name_to_id.insert(entity.name.clone(), entity.id);
    }

    fn delete(&self, id: i32) {
        if let Some((_, championship)) = self.cache.remove(&id) {
            self.name_to_id.remove(&championship.name);
        }
    }
}
