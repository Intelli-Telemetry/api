use ntex::http::StatusCode;

use super::AppError;

#[derive(Debug)]
pub enum F1ServiceError {
    AlreadyExists,
    NotActive,
    InvalidPacketType,
    CastingError,
    Shutdown,
    UnsupportedFormat,
}

impl F1ServiceError {
    pub const fn status_code(&self) -> StatusCode {
        match self {
            F1ServiceError::AlreadyExists => StatusCode::CONFLICT,
            F1ServiceError::NotActive => StatusCode::INTERNAL_SERVER_ERROR,
            F1ServiceError::InvalidPacketType => StatusCode::INTERNAL_SERVER_ERROR,
            F1ServiceError::Shutdown => StatusCode::INTERNAL_SERVER_ERROR,
            F1ServiceError::CastingError => StatusCode::INTERNAL_SERVER_ERROR,
            F1ServiceError::UnsupportedFormat => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub const fn error_message(&self) -> &'static str {
        match self {
            F1ServiceError::AlreadyExists => "Already Exists",
            F1ServiceError::NotActive => "Service not active",
            F1ServiceError::InvalidPacketType => "Invalid packet type",
            F1ServiceError::Shutdown => "Error shutting down service",
            F1ServiceError::CastingError => "Error casting data",
            F1ServiceError::UnsupportedFormat => "Unsupported Format",
        }
    }
}

impl std::error::Error for F1ServiceError {}

impl std::fmt::Display for F1ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error_message())
    }
}

impl From<F1ServiceError> for AppError {
    #[inline]
    fn from(e: F1ServiceError) -> Self {
        AppError::F1(e)
    }
}
