use ntex::web::{types::State, HttpResponse};

use crate::{error::AppResult, states::AppState};

#[inline(always)]
pub async fn pool_status(state: State<AppState>) -> AppResult<HttpResponse> {
    let active_pools = &state.server_repo.active_pools();
    Ok(HttpResponse::Ok().json(active_pools))
}
