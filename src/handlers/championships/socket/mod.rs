pub(super) mod counter;

use self::counter::{decrement, increment};
use crate::{
    entity::Championship,
    error::{AppResult, ChampionshipError, SocketError},
    states::AppState,
};
use ntex::{
    chain,
    channel::oneshot,
    fn_service, rt,
    service::{fn_factory_with_config, fn_shutdown, map_config},
    util::{select, Bytes, Either},
    web,
    ws::{self, Message},
    Service,
};
use std::{future::ready, io};
use tokio::sync::broadcast::Receiver;

#[inline(always)]
pub async fn session_socket(
    req: web::HttpRequest,
    state: web::types::State<AppState>,
    championship_id: web::types::Path<i32>,
) -> AppResult<web::HttpResponse> {
    let Some(championship) = state.championship_repository.find(&championship_id).await? else {
        Err(ChampionshipError::NotFound)?
    };

    let socket_active = state
        .f123_service
        .is_championship_socket_active(&championship.id)
        .await;

    if !socket_active {
        Err(SocketError::NotActive)?
    }

    web::ws::start(
        req,
        map_config(fn_factory_with_config(web_socket), move |cfg| {
            (cfg, state.clone(), championship.clone())
        }),
    )
    .await
}

#[inline(always)]
async fn web_socket(
    (sink, state, championship): (web::ws::WsSink, web::types::State<AppState>, Championship),
) -> AppResult<impl Service<ws::Frame, Response = Option<Message>, Error = io::Error>> {
    let (tx, close_rx) = oneshot::channel();

    {
        let cache = state
            .f123_repository
            .get_cache_data(&championship.id)
            .await?;

        if let Some(data) = cache {
            if sink.send(Message::Binary(Bytes::from(data))).await.is_err() {
                return Err(SocketError::FailedToSendMessage.into());
            };
        }
    }

    let Some(rx) = state
        .f123_service
        .subscribe_to_championship_events(&championship.id)
        .await
    else {
        return Err(SocketError::NotFound.into());
    };

    increment(championship.id);
    rt::spawn(send_data(sink, rx, close_rx));

    let service = fn_service(move |_| ready(Ok(None)));

    let on_shutdown = fn_shutdown(move || {
        decrement(championship.id);
        let _ = tx.send(());
    });

    Ok(chain(service).and_then(on_shutdown))
}

#[inline(always)]
async fn send_data(
    sink: web::ws::WsSink,
    mut rx: Receiver<Bytes>,
    mut close_rx: oneshot::Receiver<()>,
) {
    while let Either::Left(Ok(data)) = select(rx.recv(), &mut close_rx).await {
        if sink.send(Message::Binary(data)).await.is_err() {
            break;
        }
    }
}
