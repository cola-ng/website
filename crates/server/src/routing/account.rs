use salvo::prelude::*;

use super::{
    auth_required, consume_code, create_desktop_code, create_record, list_records, login,
    me, oauth_bind, oauth_login, oauth_skip, register, update_me,
};

pub fn router() -> Router {
    Router::new()
        .push(
            Router::with_path("auth")
                .push(
                    Router::with_path("code")
                        .hoop(auth_required)
                        .post(create_desktop_code),
                )
                .push(Router::with_path("consume").post(consume_code)),
        )
        .push(
            Router::with_path("oauth")
                .push(Router::with_path("login").post(oauth_login))
                .push(Router::with_path("bind").post(oauth_bind))
                .push(Router::with_path("skip").post(oauth_skip)),
        )
        .push(Router::with_path("register").post(register))
        .push(Router::with_path("login").post(login))
        .push(
            Router::with_path("me")
                .hoop(auth_required)
                .get(me)
                .put(update_me)
                .push(
                    Router::with_path("records")
                        .get(list_records)
                        .post(create_record),
                ),
        )
}
