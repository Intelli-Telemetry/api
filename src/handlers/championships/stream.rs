use garde::Validate;
use ntex::web::{
    types::{Path, State},
    HttpResponse, Responder,
};
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    error::{AppResult, CommonError, F123ServiceError},
    states::AppState,
    structs::ChampionshipIdPath,
};

pub async fn handle_stream(
    state: State<AppState>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<impl Responder> {
    if path.validate(&()).is_err() {
        Err(CommonError::ValidationFailed)?
    }

    if !state.f123_svc.service_active(path.0).await {
        Err(F123ServiceError::NotActive)?
    }

    let Some(rx) = state.f123_svc.subscribe(path.0).await else {
        Err(F123ServiceError::NotActive)?
    };

    let stream = BroadcastStream::new(rx);

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .streaming(stream))
}
