use garde::Validate;
use ntex::web::{
    types::{Form, State},
    HttpRequest, HttpResponse,
};

pub(crate) use admin::*;

use crate::{
    entity::UserExtension,
    error::{AppResult, CommonError},
    states::AppState,
    structs::{UpdateUser, UserData},
};

mod admin;

#[inline(always)]
pub(crate) async fn user_data(req: HttpRequest, state: State<AppState>) -> AppResult<HttpResponse> {
    let user = req
        .extensions()
        .get::<UserExtension>()
        .cloned()
        .ok_or(CommonError::InternalServerError)?;

    let championships = state.championship_repo.find_all(user.id).await?;

    Ok(HttpResponse::Ok().json(&UserData {
        user,
        championships,
    }))
}

#[inline(always)]
pub(crate) async fn update_user(
    req: HttpRequest,
    state: State<AppState>,
    form: Form<UpdateUser>,
) -> AppResult<HttpResponse> {
    if form.validate().is_err() {
        Err(CommonError::ValidationFailed)?
    };

    let user = req
        .extensions()
        .get::<UserExtension>()
        .cloned()
        .ok_or(CommonError::InternalServerError)?;

    state.user_svc.update(&user, &form).await?;
    Ok(HttpResponse::Ok().finish())
}
