use axum::{
    extract::{Query, State},
    response::Response,
};

use crate::{
    config::constants::*,
    entity::Provider,
    error::{AppResult, UserError},
    repositories::UserRepositoryTrait,
    services::{TokenServiceTrait, UserServiceTrait},
    states::AppState,
    structs::{GoogleCallbackQuery, TokenType},
};

pub async fn callback(
    state: State<AppState>,
    query: Query<GoogleCallbackQuery>,
) -> AppResult<Response> {
    let google_user = state.google_repository.account_info(&query.code).await?;

    let user = state
        .user_repository
        .find_by_email(&google_user.email)
        .await?;

    let user = match user {
        Some(user) => {
            if user.provider != Provider::Google {
                Err(UserError::WrongProvider)?
            }

            user
        }

        None => {
            let id = state.user_service.create(&google_user.into()).await?;

            if let Some(user) = state.user_repository.find(&id).await? {
                user
            } else {
                Err(UserError::NotFound)?
            }
        }
    };

    let access_token_fut = state
        .token_service
        .generate_token(user.id, TokenType::Bearer);

    let refresh_token_fut = state
        .token_service
        .generate_refresh_token(&user.id, "google");

    let (access_token, refresh_token) = tokio::try_join!(access_token_fut, refresh_token_fut)?;

    let redirect_url = format!(
        "{GOOGLE_REDIRECT}?access_token={}&refresh_token={}",
        access_token, refresh_token
    );

    let resp = Response::builder()
        .header("Location", redirect_url)
        .status(302)
        .body("Redirecting...".into())
        .unwrap();

    Ok(resp)
}
