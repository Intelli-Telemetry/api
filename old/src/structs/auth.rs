use garde::Validate;
use serde::{Deserialize, Serialize};
use serde_trim::{option_string_trim, string_trim};

use crate::entity::Provider;

#[derive(Deserialize)]
pub struct GoogleCallbackQuery {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct GoogleTokenRequest<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub code: &'a str,
    pub grant_type: &'a str,
    pub redirect_uri: &'a str,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct GoogleAuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: String,
    pub scope: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: String,
    pub locale: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
}

#[derive(Deserialize, Debug, Validate)]
pub struct RegisterUserDto {
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

impl From<GoogleUserInfo> for RegisterUserDto {
    fn from(value: GoogleUserInfo) -> Self {
        Self {
            username: value.name,
            email: value.email,
            password: None,
            avatar: Some(value.picture),
            provider: Some(Provider::Google),
        }
    }
}

#[derive(Deserialize, Validate)]
pub struct LoginUserDto {
    #[garde(email)]
    #[serde(deserialize_with = "string_trim")]
    pub email: String,
    #[garde(length(min = 8, max = 40))]
    #[serde(deserialize_with = "string_trim")]
    pub password: String,
}

#[derive(Deserialize, Validate)]
pub struct ForgotPasswordDto {
    #[garde(email)]
    #[serde(deserialize_with = "string_trim")]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordDto {
    #[garde(length(min = 8, max = 40))]
    #[serde(deserialize_with = "string_trim")]
    pub password: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordQuery {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailParams {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenQuery {
    pub fingerprint: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct FingerprintQuery {
    pub fingerprint: String,
}
