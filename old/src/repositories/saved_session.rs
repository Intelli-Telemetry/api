use crate::{cache::ServiceCache, config::Database, error::AppResult};

#[derive(Clone)]
pub struct SavedSessionRepository {
    db: &'static Database,
    #[allow(unused)]
    cache: &'static ServiceCache,
}

impl SavedSessionRepository {
    pub fn new(db: &'static Database, cache: &'static ServiceCache) -> Self {
        Self { db, cache }
    }

    /// This method should only be called once
    pub async fn used_ids(&self) -> AppResult<Vec<i32>> {
        let conn = self.db.pg.get().await?;

        let saved_session_ids_stmt = conn
            .prepare_cached(
                r#"
                    SELECT id FROM saved_sessions
                "#,
            )
            .await?;

        let rows = conn.query(&saved_session_ids_stmt, &[]).await?;
        let mut saved_session_ids = Vec::with_capacity(rows.len());

        for row in rows {
            let id: i32 = row.get(0);
            saved_session_ids.push(id);
        }

        Ok(saved_session_ids)
    }
}
