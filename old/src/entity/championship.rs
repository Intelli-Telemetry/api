use std::sync::Arc;

use chrono::{DateTime, Utc};
use deadpool_postgres::tokio_postgres::Row;
use postgres_derive::{FromSql, ToSql};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "championship_category")]
pub enum Category {
    #[postgres(name = "F1")]
    F1,
    #[postgres(name = "F2")]
    F2,
}

#[derive(Debug, Serialize)]
pub struct Championship {
    pub id: i32,
    pub port: i32,
    pub name: String,
    pub category: Category,
    pub season: i16,
    pub driver_count: i16,
    pub owner_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Championship {
    pub fn from_row(row: &Row) -> Arc<Championship> {
        Arc::new(Championship::from(row))
    }

    pub fn from_rows(rows: &Vec<Row>) -> Vec<Arc<Championship>> {
        let mut championships = Vec::with_capacity(rows.len());

        for row in rows {
            championships.push(Championship::from_row(row));
        }

        championships
    }
}

impl From<&Row> for Championship {
    fn from(value: &Row) -> Self {
        Self {
            id: value.get(0),
            port: value.get(1),
            name: value.get(2),
            category: value.get(3),
            season: value.get(4),
            driver_count: value.get(5),
            owner_id: value.get(6),
            created_at: value.get(7),
            updated_at: value.get(8),
        }
    }
}
