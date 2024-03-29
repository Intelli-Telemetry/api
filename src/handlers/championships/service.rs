use garde::Validate;
use ntex::web::{
    types::{Path, State},
    HttpResponse, Responder,
};

use crate::{
    error::{AppResult, ChampionshipError, CommonError},
    states::AppState,
    structs::{ChampionshipIdPath, ServiceStatus},
};

// use super::counter::get;

#[inline(always)]
pub async fn active_services(state: State<AppState>) -> AppResult<impl Responder> {
    let services = state.f123_svc.active_services().await;
    Ok(HttpResponse::Ok().json(&services))
}

#[inline(always)]
pub async fn start_service(
    state: State<AppState>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<impl Responder> {
    if path.validate(&()).is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let Some(championship) = state.championship_repo.find(path.0).await? else {
        Err(ChampionshipError::NotFound)?
    };

    state
        .f123_svc
        .start_service(championship.port, championship.id)
        .await?;

    Ok(HttpResponse::Created())
}

#[inline(always)]
pub async fn service_status(
    state: State<AppState>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<impl Responder> {
    if path.validate(&()).is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let Some(championship) = state.championship_repo.find(path.0).await? else {
        Err(ChampionshipError::NotFound)?
    };

    let num_connections = 0;
    let service_active = state.f123_svc.service_active(championship.id).await;

    // if service_active {
    // if let Some(count) = get(&path.id) {
    //     num_connections = count;
    // };
    // }

    let service_status = ServiceStatus {
        active: service_active,
        connections: num_connections,
    };

    Ok(HttpResponse::Ok().json(&service_status))
}

#[inline(always)]
pub async fn stop_service(
    state: State<AppState>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<impl Responder> {
    if path.validate(&()).is_err() {
        Err(CommonError::ValidationFailed)?
    }

    state.f123_svc.stop_service(path.0).await?;

    Ok(HttpResponse::Ok())
}
