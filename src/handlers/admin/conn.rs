use crate::{error::AppResult, repositories::UserRepositoryTrait, states::AppState};
use ntex::web;

#[inline(always)]
pub async fn pool_status(state: web::types::State<AppState>) -> AppResult<impl web::Responder> {
    let active_pools = state.user_repository.active_pools();

    Ok(web::HttpResponse::Ok().json(&active_pools))
}
