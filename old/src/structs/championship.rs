use garde::Validate;
use serde::{Deserialize, Serialize};
use serde_trim::{option_string_trim, string_trim};

use crate::entity::Category;

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

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateChampionship {
    #[garde(ascii, length(min = 3, max = 20))]
    #[serde(default, deserialize_with = "option_string_trim")]
    pub name: Option<String>,
    #[garde(skip)]
    pub category: Option<Category>,
    #[garde(skip)]
    pub season: Option<i16>,
}

#[derive(Debug, Serialize)]
pub struct ServiceStatus {
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

#[derive(Deserialize, Validate)]
pub struct ChampionshipIdPath(#[garde(range(min = 700000000, max = 799999999))] pub i32);

#[derive(Deserialize, Validate)]
pub struct ChampionshipAndUserIdPath {
    #[garde(range(min = 700000000, max = 799999999))]
    pub id: i32,
    #[garde(range(min = 600000000, max = 699999999))]
    pub user_id: i32,
}
