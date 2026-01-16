use salvo::prelude::*;

use super::{consume_code, create_code, login, me, register, update_me};
use crate::hoops::require_auth;

pub fn router() -> Router {
    Router::new()
        .push(
            Router::with_path("auth")
                .push(
                    Router::with_path("code")
                        .hoop(require_auth)
                        .post(create_code),
                )
                .push(Router::with_path("consume").post(consume_code)),
        )
        // .push(
        //     Router::with_path("oauth")
        //         .push(Router::with_path("login").post(oauth_login))
        //         .push(Router::with_path("bind").post(oauth_bind))
        //         .push(Router::with_path("skip").post(oauth_skip)),
        // )
        .push(Router::with_path("register").post(register))
        .push(Router::with_path("login").post(login))
        .push(
            Router::with_path("me")
                .hoop(require_auth)
                .get(me)
                .put(update_me)
                .push(
                    Router::with_path("records")
                        .get(list_records)
                        .post(create_record),
                ),
        )
}
