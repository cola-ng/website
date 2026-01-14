use salvo::prelude::*;

use crate::models::wallet::*;
use crate::models::*;
use crate::db::schema::*;
use crate::{DepotExt, PagedResult, db};

#[endpoint(tags("wallet"))]
pub async fn list(req: &mut Request, depot: &mut Depot) -> PagedResult<UsedCoupon> {
    let _cuser = depot.current_user()?.must_in_kernel()?;
    let conn = &mut db::conn()?;
    let data = query_pagation_data!(
        req,
        res,
        UsedCoupon,
        wallet_used_coupons::table, //.permit(cuser, "view")?,
        "used_at desc",
        OBTAINED_COUPON_FILTER_FIELDS.clone(),
        OBTAINED_COUPON_JOINED_OPTIONS.clone(),
        ID_NAME_SEARCH_TMPL,
        conn
    );
    Ok(Json(data))
}
