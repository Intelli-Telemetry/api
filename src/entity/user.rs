use std::sync::Arc;

use chrono::{DateTime, Utc};
use deadpool_postgres::tokio_postgres::Row;
use postgres_derive::{FromSql, ToSql};
use serde::{Deserialize, Serialize};

pub type UserExtension = Arc<User>;

#[derive(Debug, Serialize, Deserialize, PartialEq, FromSql, ToSql)]
#[postgres(name = "user_provider")]
pub enum Provider {
    #[postgres(name = "Local")]
    Local,
    #[postgres(name = "Google")]
    Google,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, FromSql, ToSql)]
#[postgres(name = "user_role")]
pub enum Role {
    #[postgres(name = "Free")]
    Free,
    #[postgres(name = "Premium")]
    Premium,
    #[postgres(name = "Business")]
    Business,
    #[postgres(name = "Admin")]
    Admin,
}

#[derive(Debug, Serialize)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: Option<String>,
    pub avatar: String,
    #[serde(skip_serializing)]
    pub provider: Provider,
    pub role: Role,
    #[serde(skip_serializing)]
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn from_row(row: &Row) -> Arc<Self> {
        Arc::new(Self::from(row))
    }
}

impl From<&Row> for User {
    fn from(row: &Row) -> Self {
        User {
            id: row.get(0),
            email: row.get(1),
            username: row.get(2),
            password: row.get(3),
            avatar: row.get(4),
            provider: row.get(5),
            role: row.get(6),
            active: row.get(7),
            created_at: row.get(8),
            updated_at: row.get(9),
        }
    }
}
