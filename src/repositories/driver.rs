use std::sync::Arc;

use crate::{config::Database, entity::Driver, error::AppResult};

pub struct DriverRepository {
    db: &'static Database,
}

impl DriverRepository {
    pub fn new(db: &'static Database) -> Self {
        DriverRepository { db }
    }

    pub async fn find(&self, steam_name: &str) -> AppResult<Option<Arc<Driver>>> {
        if let Some(driver) = self.db.cache.driver.get(steam_name) {
            return Ok(Some(driver));
        }

        let row = {
            let conn = self.db.pg.get().await?;

            let find_driver_stmt = conn
                .prepare_cached(
                    r#"
                        SELECT * FROM drivers
                        WHERE steam_name = $1
                    "#,
                )
                .await?;

            conn.query_opt(&find_driver_stmt, &[&steam_name]).await?
        };

        match row {
            Some(ref row) => {
                let driver = Driver::from_row_arc(row);
                self.db.cache.driver.set(driver.clone());
                Ok(Some(driver))
            }

            None => Ok(None),
        }
    }
}
