use garde::Validate;
use ntex::web::{
    types::{Form, Path, State},
    HttpRequest, HttpResponse, Responder,
};

pub(crate) use admin::*;
pub(crate) use socket::*;
pub(crate) use sockets::*;

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
mod socket;
mod sockets;

#[inline(always)]
pub async fn create_championship(
    req: HttpRequest,
    state: State<AppState>,
    form: Form<CreateChampionshipDto>,
) -> AppResult<impl Responder> {
    if form.validate(&()).is_err() {
        return Err(CommonError::ValidationFailed)?;
    }

    let user = req
        .extensions()
        .get::<UserExtension>()
        .cloned()
        .ok_or(CommonError::InternalServerError)?;

    let championships_len = state
        .championship_repository
        .championship_len(&user.id)
        .await?;

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
        .championship_service
        .create(form.into_inner(), &user.id)
        .await?;

    Ok(HttpResponse::Ok())
}

#[inline(always)]
pub async fn update(
    req: HttpRequest,
    state: State<AppState>,
    form: Form<UpdateChampionship>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<impl Responder> {
    if form.validate(&()).is_err() || path.validate(&()).is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let user_id = req
        .extensions()
        .get::<UserExtension>()
        .ok_or(CommonError::InternalServerError)?
        .id;

    state
        .championship_service
        .update(&path.id, &user_id, &form)
        .await?;

    Ok(HttpResponse::Ok())
}

#[inline(always)]
pub async fn add_user(
    req: HttpRequest,
    state: State<AppState>,
    form: Form<AddUser>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<impl Responder> {
    if form.validate(&()).is_err() || path.validate(&()).is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let user_id = req
        .extensions()
        .get::<UserExtension>()
        .ok_or(CommonError::InternalServerError)?
        .id;

    state
        .championship_service
        .add_user(&path.id, &user_id, &form.email)
        .await?;

    Ok(HttpResponse::Ok())
}

#[inline(always)]
pub async fn remove_user(
    req: HttpRequest,
    state: State<AppState>,
    path: Path<ChampionshipAndUserIdPath>,
) -> AppResult<impl Responder> {
    if path.validate(&()).is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let user_id = req
        .extensions()
        .get::<UserExtension>()
        .ok_or(CommonError::InternalServerError)?
        .id;

    state
        .championship_service
        .remove_user(&path.id, &user_id, &path.user_id)
        .await?;

    Ok(HttpResponse::Ok())
}

#[inline(always)]
pub async fn get_championship(
    state: State<AppState>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<impl Responder> {
    if path.validate(&()).is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let Some(championship) = state.championship_repository.find(&path.id).await? else {
        Err(ChampionshipError::NotFound)?
    };

    Ok(HttpResponse::Ok().json(&championship))
}

#[inline(always)]
pub async fn all_championships(
    req: HttpRequest,
    state: State<AppState>,
) -> AppResult<impl Responder> {
    let user_id = req
        .extensions()
        .get::<UserExtension>()
        .ok_or(CommonError::InternalServerError)?
        .id;

    let championships = state.championship_repository.find_all(&user_id).await?;

    Ok(HttpResponse::Ok().json(&championships))
}
