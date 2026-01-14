use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::oapi::extract::*;
use salvo::prelude::*;

use crate::models::*;
use crate::permission::Accessible;
use crate::schema::*;
use crate::{AppResult, DepotExt, JsonResult, PagedData, PagedResult, StatusInfo, db};

#[endpoint(tags("account"))]
pub fn show(notification_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<Notification> {
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let notification = notifications::table
        .find(notification_id.into_inner())
        .first::<Notification>(conn)?
        .assign_to(cuser, "view", conn)?;
    Ok(Json(notification))
}
#[endpoint(tags("account"))]
pub fn delete(notification_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;

    let notification = notifications::table
        .find(notification_id.into_inner())
        .first::<Notification>(conn)?
        .assign_to(cuser, "delete", conn)?;
    db::delete_notification(notification.id, cuser.id, conn)?;
    Ok(StatusInfo::done())
}
#[endpoint(tags("account"))]
pub async fn bulk_delete(req: &mut Request, depot: &mut Depot) -> AppResult<StatusInfo> {
    let conn = &mut db::connect()?;
    let info = bulk_delete_records!(req, depot, res, notifications, Notification, db::delete_notification, conn);
    Ok(info)
}

#[endpoint(tags("account"))]
pub async fn list(req: &mut Request, depot: &mut Depot) -> PagedResult<Notification> {
    let cuser = depot.current_user()?;
    let mut query = notifications::table.filter(notifications::owner_id.eq(cuser.id)).into_boxed();
    if let Some(stream_id) = req.query::<i64>("stream_id") {
        query = query.filter(notifications::stream_id.eq(stream_id));
    }
    let conn = &mut db::connect()?;
    let data = query_pagation_data!(
        req,
        res,
        Notification,
        query,
        "sent_at desc",
        NOTIFICATION_FILTER_FIELDS.clone(),
        NOTIFICATION_JOINED_OPTIONS.clone(),
        NOTIFICATION_SEARCH_TMPL,
        conn
    );
    Ok(Json(data))
}

#[derive(Serialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct NotificationOutGroup {
    id: i64,
    realm_id: i64,
    owner_id: i64,
    kind: String,
    subject: String,
    read_count: i64,
    unread_count: i64,

    latest_id: i64,
    latest_sender_id: i64,
    latest_sent_at: DateTime<Utc>,
    latest_notification: Option<Notification>,
}
#[endpoint(tags("account"))]
pub fn list_groups(req: &mut Request, depot: &mut Depot) -> PagedResult<NotificationOutGroup> {
    let cuser = depot.current_user()?;
    let query = notification_groups::table.filter(notification_groups::owner_id.eq(cuser.id)).into_boxed();
    let conn = &mut db::connect()?;
    let mut data = query_pagation_data!(
        req,
        res,
        NotificationGroup,
        query,
        "latest_sent_at desc",
        NOTIFICATION_GROUP_FILTER_FIELDS.clone(),
        NOTIFICATION_GROUP_JOINED_OPTIONS.clone(),
        NOTIFICATION_GROUP_SEARCH_TMPL,
        conn
    );
    let mut records = Vec::with_capacity(data.records.len());
    for NotificationGroup {
        id,
        realm_id,
        owner_id,
        kind,
        subject,
        read_count,
        unread_count,
        latest_id,
        latest_sender_id,
        latest_sent_at,
    } in std::mem::take(&mut data.records)
    {
        records.push(NotificationOutGroup {
            latest_notification: notifications::table.find(latest_id).first(conn).ok(),
            id,
            realm_id,
            owner_id,
            kind,
            subject,
            read_count,
            unread_count,
            latest_id,
            latest_sender_id,
            latest_sent_at,
        });
    }
    Ok(Json(PagedData {
        records,
        limit: data.limit,
        offset: data.offset,
        total: data.total,
        sort: data.sort,
    }))
}

#[endpoint(tags("account"))]
pub fn mark_read(req: &mut Request, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?;
    let notification_id: i64 = req.query("id").or_else(|| req.query("notification_id")).unwrap_or(0);
    let stream_id: i64 = req.query("stream_id").unwrap_or(0);
    let thread_id: i64 = req.query("thread_id").unwrap_or(0);
    let conn = &mut db::connect()?;
    if notification_id > 0 {
        diesel::update(
            notifications::table
                .filter(notifications::id.eq(notification_id))
                .filter(notifications::owner_id.eq(cuser.id)),
        )
        .set((notifications::is_read.eq(true), notifications::read_at.eq(Utc::now())))
        .execute(conn)?;
    }
    if stream_id > 0 {
        diesel::update(
            notifications::table
                .filter(notifications::owner_id.eq(cuser.id))
                .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
                    "extra @> '{{\"stream_id\":{}}}'::jsonb",
                    stream_id
                ))),
        )
        .set((notifications::is_read.eq(true), notifications::read_at.eq(Utc::now())))
        .execute(conn)?;
    }
    if thread_id > 0 {
        diesel::update(
            notifications::table
                .filter(notifications::owner_id.eq(cuser.id))
                .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
                    "extra @> '{{\"thread_id\":{}}}'::jsonb",
                    thread_id
                ))),
        )
        .set((notifications::is_read.eq(true), notifications::read_at.eq(Utc::now())))
        .execute(conn)?;
    }
    Ok(StatusInfo::done())
}
#[endpoint(tags("account"))]
pub fn mark_all_read(_req: &mut Request, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    diesel::update(notifications::table.filter(notifications::owner_id.eq(cuser.id)))
        .filter(notifications::is_read.eq(false))
        .set((notifications::is_read.eq(true), notifications::read_at.eq(Utc::now())))
        .execute(conn)?;
    Ok(StatusInfo::done())
}
