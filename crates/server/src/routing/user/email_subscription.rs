use diesel::prelude::*;

use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::{self};
use crate::models::*;
use crate::db::schema::*;
use crate::{AppError, DepotExt, JsonResult, PagedResult};

#[endpoint(tags("user"))]
pub async fn list(user_id: PathParam<i64>, req: &mut Request, depot: &mut Depot) -> PagedResult<EmailSubscription> {
    let user_id = user_id.into_inner();
    let _cuser = depot.current_user()?.must_in_kernel()?;
    let conn = &mut db::conn()?;
    let query = email_subscriptions::table.filter(email_subscriptions::user_id.eq(user_id));
    let data = query_pagation_data!(
        req,
        res,
        EmailSubscription,
        query,
        "created_at desc",
        EMAIL_SUBSCRIPTION_FILTER_FIELDS.clone(),
        EMAIL_SUBSCRIPTION_JOINED_OPTIONS.clone(),
        ID_SEARCH_TMPL,
        conn
    );
    Ok(Json(data))
}

/// 如果对应的记录不存在，认为是默认接收提醒邮件的。
#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct UpsertInItem {
    topic: String,
    value: bool,
}
#[endpoint(tags("user"))]
pub async fn upsert(user_id: PathParam<i64>, pdata: JsonBody<Vec<UpsertInItem>>, depot: &mut Depot) -> JsonResult<Vec<EmailSubscription>> {
    let pdata = pdata.into_inner();
    let user_id = user_id.into_inner();

    let cuser = depot.current_user()?.must_in_kernel()?;
    let mut conn = db::conn()?;
    let subscriptions = conn.transaction::<Vec<EmailSubscription>, AppError, _>(|conn| {
        diesel::delete(email_subscriptions::table.filter(email_subscriptions::user_id.eq(user_id))).execute(conn)?;

        let new_subscriptions = pdata
            .iter()
            .map(|item| NewEmailSubscription {
                user_id,
                realm_id: None,
                topic: &item.topic,
                value: item.value,
                updated_by: Some(cuser.id),
                created_by: Some(cuser.id),
            })
            .collect::<Vec<_>>();
        let subscriptions = diesel::insert_into(email_subscriptions::table)
            .values(&new_subscriptions)
            .get_results::<EmailSubscription>(conn)?;
        Ok(subscriptions)
    })?;
    drop(conn);

    Ok(Json(subscriptions))
}
