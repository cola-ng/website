use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use salvo::oapi::extract::*;
use salvo::prelude::*;

use crate::models::trade::*;
use crate::models::wallet::*;
use crate::models::*;
use crate::permission::Accessible;
use crate::db::schema::*;
use crate::{AppError, AppResult, DepotExt, ErrorItem, JsonResult, PagedResult, StatusInfo, db};

#[endpoint(tags("wallet"))]
pub async fn list(req: &mut Request, depot: &mut Depot) -> PagedResult<ObtainedCoupon> {
    let _cuser = depot.current_user()?.must_in_kernel()?;
    let conn = &mut db::conn()?;
    let data = query_pagation_data!(
        req,
        res,
        ObtainedCoupon,
        wallet_obtained_coupons::table, //.permit(cuser, "view")?,
        "updated_at desc",
        OBTAINED_COUPON_FILTER_FIELDS.clone(),
        OBTAINED_COUPON_JOINED_OPTIONS.clone(),
        ID_NAME_SEARCH_TMPL,
        conn
    );
    Ok(Json(data))
}

#[endpoint(tags("wallet"))]
pub async fn show(obtained_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<ObtainedCoupon> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let obtained_coupon = wallet_obtained_coupons::table
        .find(obtained_id.into_inner())
        .first::<ObtainedCoupon>(conn)?;
    realms::table
        .find(obtained_coupon.realm_id)
        .get_result::<Realm>(conn)?
        .assign_to(cuser, "wallet.view", conn)?;
    Ok(Json(obtained_coupon))
}
#[endpoint(tags("wallet"))]
pub async fn delete(obtained_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let obtained_coupon = wallet_obtained_coupons::table
        .find(obtained_id.into_inner())
        .first::<ObtainedCoupon>(conn)?;
    realms::table
        .find(obtained_coupon.realm_id)
        .get_result::<Realm>(conn)?
        .assign_to(cuser, "wallet.edit", conn)?;
    db::delete_wallet_obtained_coupon(obtained_coupon.id, cuser.id, conn)?;
    Ok(StatusInfo::done())
}
#[endpoint(tags("wallet"))]
pub async fn bulk_delete(req: &mut Request, depot: &mut Depot) -> AppResult<StatusInfo> {
    let conn = &mut db::conn()?;
    let info = bulk_delete_records!(
        req,
        depot,
        res,
        wallet_obtained_coupons,
        ObtainedCoupon,
        db::delete_wallet_obtained_coupon,
        realms,
        Realm,
        realm_id,
        "wallet.edit",
        conn
    );
    Ok(info)
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct ObtainInData {
    realm_id: i64,
    coupon_id: i64,
    #[serde(default = "crate::default_as_false")]
    send_notification: bool,
    description: Option<String>,
}
#[endpoint(tags("wallet"))]
pub async fn obtain(pdata: JsonBody<ObtainInData>, depot: &mut Depot) -> JsonResult<ObtainedCoupon> {
    let ObtainInData {
        realm_id,
        coupon_id,
        send_notification,
        description,
    } = pdata.into_inner();
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let realm = realms::table.find(realm_id).first::<Realm>(conn)?.assign_to(cuser, "wallet.edit", conn)?;
    if !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }
    let coupon = trade_coupons::table
        .filter(trade_coupons::id.eq(coupon_id))
        .filter(trade_coupons::obtained_mode.eq("realm"))
        .get_result::<Coupon>(conn)?;
    if coupon.is_delisted {
        return Err(StatusError::bad_request().brief("coupon is delisted").into());
    }

    let obtained_coupon = realm.obtain_coupon(&coupon, Some(cuser.id), None, None, description.as_deref(), conn)?;
    if send_notification {
        realm.send_assigned_coupons_notification(&[coupon], conn).ok();
    }
    Ok(Json(obtained_coupon))
}

#[derive(AsChangeset, Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
#[diesel(table_name = wallet_obtained_coupons)]
struct UpdateInData {
    is_used: Option<bool>,
    expires_at: Option<DateTime<Utc>>,
    description: Option<String>,
    #[serde(default)]
    updated_by: Option<i64>,
    updated_at: Option<DateTime<Utc>>,
}
#[endpoint(tags("wallet"))]
pub async fn update(obtained_id: PathParam<i64>, pdata: JsonBody<UpdateInData>, depot: &mut Depot) -> JsonResult<ObtainedCoupon> {
    let mut pdata = pdata.into_inner();
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let obtained_coupon = wallet_obtained_coupons::table
        .find(obtained_id.into_inner())
        .first::<ObtainedCoupon>(conn)?;
    realms::table
        .find(obtained_coupon.realm_id)
        .get_result::<Realm>(conn)?
        .assign_to(cuser, "wallet.edit", conn)?;
    if !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }
    pdata.updated_by = Some(cuser.id);
    pdata.updated_at = Some(Utc::now());
    let obtained_coupon = conn.transaction::<ObtainedCoupon, AppError, _>(|conn| {
        let obtained_coupon = diesel::update(&obtained_coupon).set(&pdata).get_result::<ObtainedCoupon>(conn)?;
        Ok(obtained_coupon)
    })?;
    Ok(Json(obtained_coupon))
}
