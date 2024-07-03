use garde::Validate;
use ntex::web::{
    types::{Path, State},
    HttpResponse,
};

use crate::{
    error::{AppResult, ChampionshipError, CommonError},
    states::AppState,
    structs::{ChampionshipIdPath, UserIdPath},
};

#[inline(always)]
pub async fn user_championships(
    state: State<AppState>,
    path: Path<UserIdPath>,
) -> AppResult<HttpResponse> {
    if path.validate().is_err() {
        Err(CommonError::ValidationFailed)?
    }

    let championships = state.championship_repo.find_all(path.0).await?;
    Ok(HttpResponse::Ok().json(&championships))
}

#[inline(always)]
pub async fn delete_championship(
    state: State<AppState>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<HttpResponse> {
    let Some(championship) = state.championship_repo.find(path.0).await? else {
        Err(ChampionshipError::NotFound)?
    };

    state.championship_svc.delete(championship.id).await?;
    Ok(HttpResponse::Ok().finish())
}
