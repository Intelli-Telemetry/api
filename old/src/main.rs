#![feature(portable_simd)]
mod cache;
mod config;
mod entity;
mod error;
mod handlers;
mod middlewares;
mod protos;
mod repositories;
mod routes;
mod services;
mod states;
mod structs;
mod utils;

use std::net::IpAddr;

use cache::ServiceCache;
use config::{initialize_tracing_subscriber, Database};
use dashmap::DashMap;
use dotenvy::{dotenv, var};
use middlewares::VisitorData;
use ntex::{http::header, web};
use ntex_cors::Cors;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use states::AppState;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    initialize_tracing_subscriber();
    let app_state = {
        let db = Box::leak(Box::from(Database::new().await));
        let service_cache = Box::leak(Box::new(ServiceCache::new()));
        AppState::new(db, service_cache).await.unwrap()
    };

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

    builder
        .set_private_key_file("certs/key.pem", SslFiletype::PEM)
        .unwrap();

    builder
        .set_certificate_chain_file("certs/cert.pem")
        .unwrap();

    // Todo - Make an recycle function to delete all unused data
    let login_limit_visitors: &'static DashMap<IpAddr, VisitorData> =
        Box::leak(Box::new(DashMap::with_capacity(100_000)));

    web::server(move || {
        web::App::new()
            .configure(|svc| routes::api_routes(svc, login_limit_visitors))
            .configure(routes::admin_routes)
            .state(app_state.clone())
            .wrap(
                Cors::new()
                    .allowed_origin("https://intellitelemetry.live")
                    .allowed_origin("http://localhost:5173")
                    .allowed_methods(["GET", "POST", "DELETE"])
                    .allowed_headers([
                        header::ACCEPT,
                        header::CONTENT_TYPE,
                        header::AUTHORIZATION,
                        header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                    ])
                    .max_age(3600)
                    .finish(),
            )
    })
    .bind_openssl(var("HOST").unwrap(), builder)?
    .run()
    .await
}
