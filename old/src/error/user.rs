use ntex::{
    http::{
        header::{HeaderValue, CONTENT_TYPE},
        StatusCode,
    },
    web::{error::WebResponseError, HttpRequest, HttpResponse},
};

use super::AppError;

#[derive(Debug)]
pub enum UserError {
    AlreadyExists,
    NotFound,
    InvalidCredentials,
    NotVerified,
    GoogleLogin,
    Unauthorized,
    AutoDelete,
    AlreadyActive,
    AlreadyInactive,
    InvalidProvider,
    WrongProvider,
    InvalidUpdate,
    UpdateLimitExceeded,
}

impl UserError {
    pub const fn status_code(&self) -> StatusCode {
        match self {
            UserError::AlreadyExists => StatusCode::CONFLICT,
            UserError::NotFound => StatusCode::NOT_FOUND,
            UserError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            UserError::NotVerified => StatusCode::UNAUTHORIZED,
            UserError::GoogleLogin => StatusCode::BAD_REQUEST,
            UserError::Unauthorized => StatusCode::UNAUTHORIZED,
            UserError::AutoDelete => StatusCode::BAD_REQUEST,
            UserError::AlreadyActive => StatusCode::BAD_REQUEST,
            UserError::AlreadyInactive => StatusCode::BAD_REQUEST,
            UserError::InvalidProvider => StatusCode::BAD_REQUEST,
            UserError::WrongProvider => StatusCode::BAD_REQUEST,
            UserError::InvalidUpdate => StatusCode::BAD_REQUEST,
            UserError::UpdateLimitExceeded => StatusCode::UNAUTHORIZED,
        }
    }

    pub const fn error_message(&self) -> &'static str {
        match self {
            UserError::AlreadyExists => "User already exists",
            UserError::NotFound => "User not found",
            UserError::InvalidCredentials => "Invalid credentials",
            UserError::NotVerified => "Not verified user",
            UserError::GoogleLogin => "Use google to login",
            UserError::Unauthorized => "Unauthorized user",
            UserError::AutoDelete => "Cannot Delete Yourself",
            UserError::AlreadyActive => "User Already Active",
            UserError::AlreadyInactive => "User is not active",
            UserError::InvalidProvider => "Invalid Provider",
            UserError::WrongProvider => "Using wrong provider",
            UserError::InvalidUpdate => "Invalid Update",
            UserError::UpdateLimitExceeded => "Update Limit Exceeded",
        }
    }
}

impl std::error::Error for UserError {}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error_message())
    }
}

impl From<UserError> for AppError {
    fn from(e: UserError) -> Self {
        AppError::User(e)
    }
}

// Added for middlewares
impl WebResponseError for UserError {
    fn error_response(&self, _: &HttpRequest) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .header(
                CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            )
            .body(self.error_message())
    }
}
