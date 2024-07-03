use garde::Validate;
use ntex::web::{
    types::{Form, Path, State},
    HttpRequest, HttpResponse,
};

pub(crate) use admin::*;
pub(crate) use service::*;
pub(crate) use stream::*;

use crate::{
    entity::{Role, UserExtension},
    error::{AppResult, ChampionshipError, CommonError},
    states::AppState,
    structs::{
        AddUser, ChampionshipAndUserIdPath, ChampionshipIdPath, CreateChampionshipDto,
        UpdateChampionship,
    },
};

mod admin;
mod service;
mod stream;

#[inline(always)]
pub async fn create_championship(
    req: HttpRequest,
    state: State<AppState>,
    form: Form<CreateChampionshipDto>,
) -> AppResult<HttpResponse> {
    if form.validate().is_err() {
        return Err(CommonError::ValidationFailed)?;
    }

    let user = req
        .extensions()
        .get::<UserExtension>()
        .cloned()
        .ok_or(CommonError::InternalServerError)?;

    let championships_len = state.championship_repo.championship_len(user.id).await?;

    match user.role {
        Role::Free => {
            if championships_len >= 1 {
                Err(ChampionshipError::LimitReached)?
            }
        }

        Role::Premium => {
            if championships_len >= 3 {
                Err(ChampionshipError::LimitReached)?
            }
        }

        Role::Business => {
            if championships_len >= 14 {
                Err(ChampionshipError::LimitReached)?
            }
        }

        Role::Admin => {}
    }

    state
        .championship_svc
        .create(form.into_inner(), user.id)
        .await?;

    Ok(HttpResponse::Created().finish())
}

#[inline(always)]
pub async fn update(
    req: HttpRequest,
    state: State<AppState>,
    form: Form<UpdateChampionship>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<HttpResponse> {
    if form.validate().is_err() || path.validate().is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let user_id = req
        .extensions()
        .get::<UserExtension>()
        .ok_or(CommonError::InternalServerError)?
        .id;

    state
        .championship_svc
        .update(path.0, user_id, &form)
        .await?;

    Ok(HttpResponse::Ok().finish())
}

#[inline(always)]
pub async fn add_user(
    req: HttpRequest,
    state: State<AppState>,
    form: Form<AddUser>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<HttpResponse> {
    if form.validate().is_err() || path.validate().is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let user_id = req
        .extensions()
        .get::<UserExtension>()
        .ok_or(CommonError::InternalServerError)?
        .id;

    state
        .championship_svc
        .add_user(path.0, user_id, &form.email)
        .await?;

    Ok(HttpResponse::Ok().finish())
}

#[inline(always)]
pub async fn remove_user(
    req: HttpRequest,
    state: State<AppState>,
    path: Path<ChampionshipAndUserIdPath>,
) -> AppResult<HttpResponse> {
    if path.validate().is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let user_id = req
        .extensions()
        .get::<UserExtension>()
        .ok_or(CommonError::InternalServerError)?
        .id;

    state
        .championship_svc
        .remove_user(path.id, user_id, path.user_id)
        .await?;

    Ok(HttpResponse::Ok().finish())
}

#[inline(always)]
pub async fn get_championship(
    state: State<AppState>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<HttpResponse> {
    if path.validate().is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let Some(championship) = state.championship_repo.find(path.0).await? else {
        Err(ChampionshipError::NotFound)?
    };

    Ok(HttpResponse::Ok().json(&championship))
}

#[inline(always)]
pub async fn all_championships(
    req: HttpRequest,
    state: State<AppState>,
) -> AppResult<HttpResponse> {
    let user_id = req
        .extensions()
        .get::<UserExtension>()
        .ok_or(CommonError::InternalServerError)?
        .id;

    let championships = state.championship_repo.find_all(user_id).await?;

    Ok(HttpResponse::Ok().json(&championships))
}
