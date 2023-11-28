use crate::entity::Category;
use garde::Validate;
use serde::{Deserialize, Serialize};
use serde_trim::string_trim;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateChampionshipDto {
    #[garde(ascii, length(min = 3, max = 20))]
    #[serde(deserialize_with = "string_trim")]
    pub name: String,
    #[garde(skip)]
    pub category: Category,
    #[garde(skip)]
    pub season: i16,
}

#[derive(Debug, Serialize)]
pub struct SocketStatus {
    pub active: bool,
    pub connections: usize,
}

#[allow(dead_code)]
pub struct ChampionshipCacheData {
    pub session_data: Vec<u8>,
    pub motion_data: Vec<u8>,
    pub participants_data: Vec<u8>,
    pub history_data: Option<Vec<Vec<u8>>>,
}
