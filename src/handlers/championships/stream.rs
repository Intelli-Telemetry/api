use std::pin::Pin;
use std::task::{Context, Poll};

use garde::Validate;
use ntex::web::{
    types::{Path, State},
    HttpResponse,
};
use reqwest::header::HeaderValue;
use tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt};

use crate::{
    error::{AppResult, CommonError, F1ServiceError},
    states::AppState,
    structs::ChampionshipIdPath,
};

struct CleanupStream<S> {
    inner: S,
    app_state: State<AppState>,
    championship_id: i32,
}

impl<S: Stream + Unpin> Stream for CleanupStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

impl<S> Drop for CleanupStream<S> {
    fn drop(&mut self) {
        self.app_state.f1_svc.unsubscribe(&self.championship_id);
    }
}

#[inline(always)]
pub async fn handle_stream(
    state: State<AppState>,
    path: Path<ChampionshipIdPath>,
) -> AppResult<HttpResponse> {
    if path.validate().is_err() {
        Err(CommonError::ValidationFailed)?
    }

    if !state.f1_svc.service(&path.0) {
        Err(F1ServiceError::NotActive)?
    }

    let cached_data = state.f1_svc.cache(&path.0).await?;

    let Some(rx) = state.f1_svc.subscribe(&path.0) else {
        Err(F1ServiceError::NotActive)?
    };

    let stream = BroadcastStream::new(rx);
    let cleanup_stream = CleanupStream {
        inner: stream,
        app_state: state.clone(),
        championship_id: path.0,
    };

    let mut response = HttpResponse::Ok();
    response.content_type(HeaderValue::from_static("application/octet-stream"));

    match cached_data {
        None => Ok(response.streaming(cleanup_stream)),

        Some(data) => {
            let cache_steam = tokio_stream::once(Ok(data));
            let combined_stream = cache_steam.chain(cleanup_stream);

            Ok(response.streaming(combined_stream))
        }
    }
}
