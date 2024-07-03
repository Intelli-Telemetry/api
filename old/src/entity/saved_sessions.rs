use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SavedSession<'a> {
    pub id: i32,
    pub events: &'a [u8],
    pub session_data: &'a [u8],
    pub participants: &'a [u8],
    pub session_history: &'a [u8],
    pub final_classifications: &'a [u8],
    pub championship_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
