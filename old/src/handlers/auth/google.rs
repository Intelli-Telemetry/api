use ntex::web::{
    types::{Query, State},
    HttpResponse,
};

use crate::{
    config::constants::*,
    entity::Provider,
    error::{AppResult, UserError},
    states::AppState,
    structs::{GoogleCallbackQuery, TokenType},
};

#[inline(always)]
pub async fn callback(
    state: State<AppState>,
    query: Query<GoogleCallbackQuery>,
) -> AppResult<HttpResponse> {
    let google_user = state.google_repo.account_info(&query.code).await?;
    let user = state.user_repo.find_by_email(&google_user.email).await?;

    let user = match user {
        Some(user) => {
            if user.provider != Provider::Google {
                Err(UserError::WrongProvider)?
            }

            user
        }

        None => {
            let id = state.user_svc.create(&google_user.into()).await?;

            match state.user_repo.find(id).await? {
                Some(user) => user,
                None => Err(UserError::NotFound)?,
            }
        }
    };

    let access_token = state.token_svc.generate_token(user.id, TokenType::Bearer)?;
    let refresh_token = state
        .token_svc
        .generate_refresh_token(user.id, String::from("google"))?;

    let redirect_url = format!(
        "{GOOGLE_REDIRECT}?access_token={}&refresh_token={}",
        access_token, refresh_token
    );

    Ok(HttpResponse::Found()
        .set_header("Location", redirect_url)
        .body("Redirecting..."))
}
