use garde::Validate;
use ntex::web::{
    types::{Path, State},
    HttpResponse,
};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

use crate::{
    error::{AppResult, CommonError, F1ServiceError},
    states::AppState,
    structs::ChampionshipIdPath,
};

#[inline(always)]
pub async fn handle_stream(
    state: State<AppState>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<HttpResponse> {
    if path.validate().is_err() {
        Err(CommonError::ValidationFailed)?
    }

    if !state.f1_svc.service_active(path.0).await {
        Err(F1ServiceError::NotActive)?
    }

    let cached_data = state.f1_svc.service_cache(path.0).await?;

    let Some(rx) = state.f1_svc.subscribe(path.0).await else {
        Err(F1ServiceError::NotActive)?
    };

    let stream = BroadcastStream::new(rx);

    match cached_data {
        None => Ok(HttpResponse::Ok()
            .content_type("application/octet-stream")
            .streaming(stream)),

        Some(data) => {
            let cache_steam = tokio_stream::once(Ok(data));
            let combined_stream = cache_steam.chain(stream);

            Ok(HttpResponse::Ok()
                .content_type("application/octet-stream")
                .streaming(combined_stream))
        }
    }
}
