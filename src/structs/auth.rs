use garde::Validate;
use serde::{Deserialize, Serialize};
use serde_trim::{option_string_trim, string_trim};

use crate::entity::Provider;

// Authentication Structures
#[derive(Deserialize, Validate)]
pub struct LoginCredentials {
    #[garde(email)]
    #[serde(deserialize_with = "string_trim")]
    pub email: String,
    #[garde(length(min = 8, max = 40))]
    #[serde(deserialize_with = "string_trim")]
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct NewAccessToken {
    pub access_token: String,
}

// User Registration and Management
#[derive(Deserialize, Debug, Validate)]
pub struct UserRegistrationData {
    #[garde(ascii, length(min = 3, max = 20))]
    #[serde(deserialize_with = "string_trim")]
    pub username: String,
    #[garde(email)]
    #[serde(deserialize_with = "string_trim")]
    pub email: String,
    #[serde(default, deserialize_with = "option_string_trim")]
    #[garde(length(min = 8, max = 40))]
    pub password: Option<String>,
    #[garde(inner(length(min = 10, max = 100)))]
    pub avatar: Option<String>,
    #[garde(skip)]
    pub provider: Option<Provider>,
}

impl UserRegistrationData {
    pub fn from_google_user_info(google_info: GoogleUserInfo) -> Self {
        Self {
            username: google_info.name,
            email: google_info.email,
            password: None,
            avatar: Some(google_info.picture),
            provider: Some(Provider::Google),
        }
    }
}

// Password Management
#[derive(Deserialize, Validate)]
pub struct PasswordResetRequest {
    #[garde(email)]
    #[serde(deserialize_with = "string_trim")]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PasswordUpdateData {
    #[garde(length(min = 8, max = 40))]
    #[serde(deserialize_with = "string_trim")]
    pub password: String,
}

// Token and Security
#[derive(Debug, Deserialize)]
pub struct TokenVerification {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub fingerprint: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientFingerprint {
    pub fingerprint: String,
}

// Google OAuth Structures
#[derive(Deserialize)]
pub struct GoogleAuthorizationCode {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct GoogleTokenExchangeRequest<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub code: &'a str,
    pub grant_type: &'a str,
    pub redirect_uri: &'a str,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct GoogleAuthTokens {
    pub access_token: String,
    pub expires_in: i64,
    pub id_token: String,
    pub scope: String,
    pub token_type: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub email: String,
    pub family_name: Option<String>,
    pub given_name: Option<String>,
    pub id: String,
    pub name: String,
    pub picture: String,
    pub verified_email: bool,
}
