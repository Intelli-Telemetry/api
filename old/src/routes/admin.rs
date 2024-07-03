use ntex::web::{delete, get, post, scope, ServiceConfig};

use crate::{
    handlers::{
        admin::pool_status,
        championships::{active_services, delete_championship, user_championships},
        user::{delete_user, disable_user, enable_user},
    },
    middlewares::{Admin, Authentication},
};

pub(crate) fn admin_routes(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/admin")
            .service(
                scope("/users")
                    .route("/{id}", delete().to(delete_user))
                    .route("/{id}/enable", post().to(enable_user))
                    .route("/{id}/disable", post().to(disable_user)),
            )
            .service(
                scope("/championships")
                    .route("/{id}", get().to(user_championships)) // id = user_id
                    .route("/{id}", delete().to(delete_championship)),
            )
            .service(scope("/services").route("/services", get().to(active_services)))
            .route("/pools", get().to(pool_status))
            .wrap(Admin)
            .wrap(Authentication),
    );
}
