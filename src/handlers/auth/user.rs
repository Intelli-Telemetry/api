use chrono::Utc;
use garde::Validate;
use ntex::web::{
    types::{Form, Query, State},
    HttpRequest, HttpResponse, Responder,
};

use crate::{
    config::constants::PASSWORD_UPDATE_INTERVAL,
    entity::{Provider, UserExtension},
    error::{AppResult, CommonError, UserError},
    states::AppState,
    structs::{
        AuthResponse, EmailUser, FingerprintQuery, ForgotPasswordDto, LoginUserDto,
        PasswordChanged, RefreshResponse, RefreshTokenQuery, RegisterUserDto, ResetPassword,
        ResetPasswordDto, ResetPasswordQuery, TokenType, VerifyEmail,
    },
};

#[inline(always)]
pub(crate) async fn register(
    state: State<AppState>,
    form: Form<RegisterUserDto>,
) -> AppResult<impl Responder> {
    if form.validate(&()).is_err() {
        return Err(CommonError::ValidationFailed)?;
    }

    let user_id = state.user_svc.create(&form).await?;

    let token = state
        .token_svc
        .generate_token(user_id, TokenType::Email)
        .await?;

    state.token_svc.save_email_token(&token).await?;

    let template = VerifyEmail {
        verification_link: &format!(
            "https://intellitelemetry.live/auth/verify-email?token={}",
            token
        ),
    };

    state
        .email_svc
        .send_mail((&*form).into(), "Verify Email", template)?;

    Ok(HttpResponse::Created())
}

// Todo: This function call is expensive in terms of performance. Because dehashing the password is a blocking operation
// Todo: Every user should have a limit to login attempts and should be locked out for a certain period of time
#[inline(always)]
pub(crate) async fn login(
    state: State<AppState>,
    query: Query<FingerprintQuery>,
    form: Form<LoginUserDto>,
) -> AppResult<impl Responder> {
    if form.validate(&()).is_err() {
        return Err(CommonError::ValidationFailed)?;
    }

    let Some(user) = state.user_repo.find_by_email(&form.email).await? else {
        return Err(UserError::NotFound)?;
    };

    if !user.active {
        return Err(UserError::NotVerified)?;
    }

    if user.provider != Provider::Local {
        return Err(UserError::GoogleLogin)?;
    }

    if !state
        .user_repo
        .validate_password(&form.password, &user.password.unwrap())?
    {
        return Err(UserError::InvalidCredentials)?;
    }

    let access_token_fut = state.token_svc.generate_token(user.id, TokenType::Bearer);

    let refresh_token_fut = state
        .token_svc
        .generate_refresh_token(user.id, &query.fingerprint);

    let (access_token, refresh_token) = tokio::try_join!(access_token_fut, refresh_token_fut)?;

    let auth_response = &AuthResponse {
        access_token,
        refresh_token,
    };

    Ok(HttpResponse::Ok().json(auth_response))
}

#[inline(always)]
pub(crate) async fn refresh_token(
    state: State<AppState>,
    query: Query<RefreshTokenQuery>,
) -> AppResult<impl Responder> {
    let access_token = state
        .token_svc
        .refresh_access_token(&query.refresh_token, &query.fingerprint)
        .await?;

    let refresh_response = &RefreshResponse { access_token };

    Ok(HttpResponse::Ok().json(refresh_response))
}

#[inline(always)]
pub(crate) async fn logout(
    req: HttpRequest,
    state: State<AppState>,
    query: Query<FingerprintQuery>,
) -> AppResult<impl Responder> {
    let user_id = req
        .extensions()
        .get::<UserExtension>()
        .ok_or(CommonError::InternalServerError)?
        .id;

    state
        .token_svc
        .remove_refresh_token(user_id, &query.fingerprint)
        .await?;

    Ok(HttpResponse::Ok())
}

#[inline(always)]
pub(crate) async fn forgot_password(
    state: State<AppState>,
    form: Form<ForgotPasswordDto>,
) -> AppResult<impl Responder> {
    if form.validate(&()).is_err() {
        return Err(CommonError::ValidationFailed)?;
    }

    let Some(user) = state.user_repo.find_by_email(&form.email).await? else {
        return Err(UserError::NotFound)?;
    };

    // Todo: Duration::hours(1) should be a constant and Utc::now() should be saved in a variable for a cache of 1 minute
    if Utc::now().signed_duration_since(user.updated_at) > *PASSWORD_UPDATE_INTERVAL {
        return Err(UserError::UpdateLimitExceeded)?;
    }

    let token = state
        .token_svc
        .generate_token(user.id, TokenType::ResetPassword)
        .await?;

    let template = ResetPassword {
        reset_password_link: &format!(
            "https://intellitelemetry.live/auth/reset-password?token={}",
            token
        ),
    };

    state.token_svc.save_reset_password_token(&token).await?;

    state.email_svc.send_mail(
        EmailUser {
            username: &user.username,
            email: &user.email,
        },
        "Reset Password",
        template,
    )?;

    Ok(HttpResponse::Ok())
}

#[inline(always)]
pub async fn reset_password(
    query: Query<ResetPasswordQuery>,
    state: State<AppState>,
    form: Form<ResetPasswordDto>,
) -> AppResult<impl Responder> {
    if form.validate(&()).is_err() {
        return Err(CommonError::ValidationFailed)?;
    }

    let user_id = state
        .user_svc
        .reset_password_with_token(&query.token, &form.password)
        .await?;

    let Some(user) = state.user_repo.find(user_id).await? else {
        Err(UserError::NotFound)?
    };

    let template = PasswordChanged {};

    state.email_svc.send_mail(
        EmailUser {
            username: &user.username,
            email: &user.email,
        },
        "Password Changed",
        template,
    )?;

    Ok(HttpResponse::Ok())
}
