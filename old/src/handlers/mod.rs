use ntex::web::HttpResponse;

pub(crate) mod admin;
pub(crate) mod auth;
pub(crate) mod championships;
pub(crate) mod user;

pub(crate) async fn heartbeat() -> HttpResponse {
    HttpResponse::Ok().finish()
}
