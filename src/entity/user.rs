use std::sync::Arc;

use chrono::{DateTime, Utc};
use deadpool_postgres::tokio_postgres::Row;
use postgres_derive::{FromSql, ToSql};
use rkyv::{Archive, Deserialize as RDeserialize, Serialize as RSerialize};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub type UserExtension = Arc<User>;

#[derive(
    Debug, Archive, RDeserialize, RSerialize, Serialize, Deserialize, PartialEq, Eq, FromSql, ToSql,
)]
#[postgres(name = "user_provider")]
pub enum Provider {
    #[postgres(name = "Local")]
    Local,
    #[postgres(name = "Google")]
    Google,
}

#[derive(Debug, Archive, RDeserialize, RSerialize, Serialize, PartialEq, Eq, FromSql, ToSql)]
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

#[derive(Debug, Serialize, Archive, RDeserialize, RSerialize)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: Option<String>,
    #[serde(skip_serializing)]
    pub provider: Provider,
    pub avatar: String,
    pub role: Role,
    #[serde(skip_serializing)]
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<&Row> for User {
    type Error = AppError;

    #[inline]
    fn try_from(row: &Row) -> AppResult<User> {
        Ok(User {
            id: row.try_get("id")?,
            email: row.try_get("email")?,
            username: row.try_get("username")?,
            password: row.try_get("password")?,
            provider: row.try_get("provider")?,
            avatar: row.try_get("avatar")?,
            role: row.try_get("role")?,
            active: row.try_get("active")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}
