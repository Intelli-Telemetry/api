use crate::{
    entity::{Role, User},
    error::{AppResult, UserError},
};
use axum::{http::Request, middleware::Next, response::Response};
use tracing::info;

pub async fn admin_handler<T>(req: Request<T>, next: Next<T>) -> AppResult<Response> {
    let user = req
        .extensions()
        .get::<User>()
        .ok_or(UserError::Unauthorized)?;

    if user.role.ne(&Role::Admin) {
        info!(
            "User: {}, tried to access to admin area without having admin role",
            user.id
        );
        Err(UserError::Unauthorized)?
    }

    Ok(next.run(req).await)
}