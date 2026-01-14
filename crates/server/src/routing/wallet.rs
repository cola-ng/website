mod balance;
mod history;
mod obtained_coupon;
mod used_coupon;

use crate::routers::full_routed;
use salvo::prelude::*;

pub fn authed_root(path: impl Into<String>) -> Router {
    Router::with_path(path)
        .push(Router::with_path("balances").then(|router| {
            if full_routed() {
                router
                    .post(balance::create)
                    .push(Router::with_path("{balance_id}").patch(balance::update))
            } else {
                router
            }
        }))
        .push(Router::with_path("histories").then(|router| if full_routed() { router.post(history::list) } else { router }))
        .push(
            Router::with_path("obtained_coupons")
                .push(Router::with_path("{obtained_id:u64}").get(obtained_coupon::show))
                .then(|router| {
                    if full_routed() {
                        router
                            .get(obtained_coupon::list)
                            .post(obtained_coupon::obtain)
                            .delete(obtained_coupon::bulk_delete)
                            .push(
                                Router::with_path("{obtained_id:u64}")
                                    .patch(obtained_coupon::update)
                                    .delete(obtained_coupon::delete),
                            )
                    } else {
                        router
                    }
                }),
        )
        .then(|router| {
            if full_routed() {
                router.push(Router::with_path("used_coupons").get(used_coupon::list))
            } else {
                router
            }
        })
}
