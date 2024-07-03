use garde::Validate;
use ntex::web::{
    types::{Path, State},
    HttpRequest, HttpResponse,
};

use crate::{
    entity::UserExtension,
    error::{AppResult, CommonError, UserError},
    states::AppState,
    structs::UserIdPath,
};

#[inline(always)]
pub async fn delete_user(
    req: HttpRequest,
    state: State<AppState>,
    path: Path<UserIdPath>,
) -> AppResult<HttpResponse> {
    if path.validate().is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let user_id = req
        .extensions()
        .get::<UserExtension>()
        .ok_or(CommonError::InternalServerError)?
        .id;

    if state.user_repo.find(path.0).await?.is_none() {
        Err(UserError::NotFound)?
    };

    if path.0 == user_id {
        Err(UserError::AutoDelete)?
    }

    state.user_svc.delete(path.0).await?;
    Ok(HttpResponse::Ok().finish())
}

#[inline(always)]
pub async fn disable_user(
    req: HttpRequest,
    state: State<AppState>,
    path: Path<UserIdPath>,
) -> AppResult<HttpResponse> {
    if path.validate().is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let Some(path_user_active) = state.user_repo.status(path.0).await? else {
        Err(UserError::NotFound)?
    };

    if !path_user_active {
        Err(UserError::AlreadyInactive)?
    }

    let user_id = req
        .extensions()
        .get::<UserExtension>()
        .ok_or(CommonError::InternalServerError)?
        .id;

    if path.0 == user_id {
        Err(UserError::AutoDelete)?
    }

    state.user_svc.deactivate(path.0).await?;
    Ok(HttpResponse::Ok().finish())
}

#[inline(always)]
pub async fn enable_user(
    req: HttpRequest,
    state: State<AppState>,
    path: Path<UserIdPath>,
) -> AppResult<HttpResponse> {
    if path.validate().is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let Some(path_user_active) = state.user_repo.status(path.0).await? else {
        Err(UserError::NotFound)?
    };

    if path_user_active {
        Err(UserError::AlreadyActive)?
    }

    let user_id = req
        .extensions()
        .get::<UserExtension>()
        .ok_or(CommonError::InternalServerError)?
        .id;

    if path.0 == user_id {
        Err(UserError::AutoDelete)?
    }

    state.user_svc.activate(path.0).await?;
    Ok(HttpResponse::Ok().finish())
}
