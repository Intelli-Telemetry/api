use std::fmt::Display;

use ntex::{
    http::StatusCode,
    web::{error::WebResponseError, HttpRequest, HttpResponse},
};

use super::AppError;

#[derive(Debug)]
pub enum ChampionshipError {
    AlreadyExists,
    NotFound,
    LimitReached,
    NotOwner,
    CannotRemoveOwner,
    IntervalNotReached,
    NoPortsAvailable,
}

impl std::error::Error for ChampionshipError {}

impl Display for ChampionshipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error_message())
    }
}

impl ChampionshipError {
    pub const fn status_code(&self) -> StatusCode {
        match self {
            ChampionshipError::AlreadyExists => StatusCode::CONFLICT,
            ChampionshipError::NotFound => StatusCode::NOT_FOUND,
            ChampionshipError::LimitReached => StatusCode::BAD_REQUEST,
            ChampionshipError::NotOwner => StatusCode::UNAUTHORIZED,
            ChampionshipError::CannotRemoveOwner => StatusCode::BAD_REQUEST,
            ChampionshipError::IntervalNotReached => StatusCode::BAD_REQUEST,
            ChampionshipError::NoPortsAvailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    pub const fn error_message(&self) -> &'static str {
        match self {
            ChampionshipError::AlreadyExists => "Championship already exists",
            ChampionshipError::NotFound => "Championship not found",
            ChampionshipError::LimitReached => "Championship limit reached",
            ChampionshipError::NotOwner => "Not Owner of Championship",
            ChampionshipError::CannotRemoveOwner => "Cannot remove owner of Championship",
            ChampionshipError::IntervalNotReached => "Interval update time not reached",
            ChampionshipError::NoPortsAvailable => "No ports available",
        }
    }
}

impl WebResponseError for ChampionshipError {
    fn error_response(&self, _: &HttpRequest) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .set_header("content-type", "text/plain; charset=utf-8")
            .body(self.error_message())
    }
}

impl From<ChampionshipError> for AppError {
    fn from(error: ChampionshipError) -> Self {
        AppError::Championship(error)
    }
}
