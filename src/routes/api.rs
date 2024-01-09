use ntex::web::{delete, get, post, put, resource, scope, ServiceConfig};

use crate::{
    handlers::{
        auth::{
            callback, forgot_password, login, logout, refresh_token, register, reset_password,
            verify_email,
        },
        championships::{
            add_user, all_championships, create_championship, get_championship, remove_user,
            session_socket, socket_status, start_socket, stop_socket, update,
        },
        heartbeat,
        intelli_app::latest_release,
        user::{update_user, user_data},
    },
    middlewares::Authentication,
};

#[inline(always)]
pub(crate) fn api_routes(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/auth")
            .route("/google/callback", get().to(callback))
            .route("/register", post().to(register))
            .route("/login", post().to(login))
            .route("/refresh", get().to(refresh_token))
            .route("/verify/email", get().to(verify_email))
            .route("/forgot-password", post().to(forgot_password))
            .route("/reset-password", post().to(reset_password)),
    );

    cfg.service(
        resource("/logout")
            .route(get().to(logout))
            .wrap(Authentication),
    );

    cfg.service(
        scope("/user")
            .route("", put().to(update_user))
            .route("/data", get().to(user_data))
            .wrap(Authentication),
    );

    cfg.service(scope("/intelli-app").route("/releases/latest", get().to(latest_release)));

    cfg.service(
        scope("/championships")
            .route("", post().to(create_championship))
            .route("/all", get().to(all_championships))
            .route("/{id}", get().to(get_championship))
            .route("/{id}", put().to(update))
            .route("/{id}/user/add", put().to(add_user))
            .route("/{id}/user/{user_id}", delete().to(remove_user))
            .route("/{id}/socket/start", get().to(start_socket))
            .route("/{id}/socket/status", get().to(socket_status))
            .route("/{id}/socket/stop", get().to(stop_socket))
            .wrap(Authentication),
    );

    cfg.route("/heartbeat", get().to(heartbeat));

    cfg.route("/web_socket/championship/{id}", get().to(session_socket));
}
