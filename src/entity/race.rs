use chrono::{DateTime, Utc};
use deadpool_postgres::tokio_postgres::Row;

/// Represents a race in a championship
#[allow(unused)]
pub struct Race {
    id: i32,
    championship_id: i32,
    track_id: i16,
    date: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Race {
    /// Creates a Race from a database row
    #[inline]
    #[allow(unused)]
    pub fn from_row(row: &Row) -> Self {
        Race {
            id: row.get(0),
            championship_id: row.get(1),
            track_id: row.get(2),
            date: row.get(3),
            created_at: row.get(4),
            updated_at: row.get(5),
        }
    }
}
