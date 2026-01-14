use diesel::prelude::*;
use salvo::oapi::extract::*;
use salvo::prelude::*;

use crate::models::*;
use crate::permission::Accessible;
use crate::schema::*;
use crate::{AppResult, DepotExt, PagedResult, StatusInfo, db};

#[endpoint(tags("role"))]
pub fn list(role_id: PathParam<i64>, req: &mut Request, depot: &mut Depot) -> PagedResult<User> {
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let role = roles::table
        .find(role_id.into_inner())
        .first::<Role>(conn)?
        .assign_to(cuser, "users.view", conn)?;

    let query = users::table
        .filter(users::is_disabled.eq(false))
        .filter(users::id.eq_any(role_users::table.filter(role_users::role_id.eq(role.id)).select(role_users::user_id)));
    // let query = db_query_apply_permission_filter!(query, cuser, "user", "view");//TODO: permission
    let data = query_pagation_data!(
        req,
        res,
        User,
        query,
        "created_at desc",
        USER_FILTER_FIELDS.clone(),
        USER_JOINED_OPTIONS.clone(),
        USER_SEARCH_TMPL,
        conn
    );
    Ok(Json(data))
}

#[endpoint(tags("user"))]
pub async fn add(role_id: PathParam<i64>, req: &mut Request, depot: &mut Depot) -> AppResult<StatusInfo> {
    let ids = crate::parse_ids_from_request(req, "id", "ids").await;
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let role = roles::table
        .find(role_id.into_inner())
        .first::<Role>(conn)?
        .assign_to(cuser, "users.add", conn)?;
    if crate::is_kernel_realm_id(role.realm_id) && !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }

    let mut rusers = vec![];
    let user_ids = if crate::is_kernel_realm_id(role.realm_id) {
        users::table
            .filter(users::id.eq_any(ids))
            .filter(users::in_kernel.eq(true))
            .filter(users::is_disabled.eq(false))
            .select(users::id)
            .get_results::<i64>(conn)?
    } else {
        users::table
            .filter(users::id.eq_any(ids))
            .filter(users::is_disabled.eq(false))
            .select(users::id)
            .get_results::<i64>(conn)?
    };
    for id in &user_ids {
        let ruser = crate::models::RoleUser {
            realm_id: role.realm_id,
            role_id: role.id,
            user_id: *id,
        };
        rusers.push(ruser);
    }
    if !rusers.is_empty() {
        diesel::insert_into(role_users::table)
            .values(&rusers)
            .on_conflict_do_nothing()
            .execute(conn)?;
    }
    Ok(StatusInfo::done())
}
#[endpoint(tags("user"))]
pub async fn remove(role_id: PathParam<i64>, req: &mut Request, depot: &mut Depot) -> AppResult<StatusInfo> {
    let ids = crate::parse_ids_from_request(req, "id", "ids").await;
    let conn = &mut db::connect()?;
    let cuser = depot.current_user()?.must_in_kernel()?;
    let role = roles::table
        .find(role_id.into_inner())
        .first::<Role>(conn)?
        .assign_to(cuser, "users.remove", conn)?;
    if crate::is_kernel_realm_id(role.realm_id) && !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }
    diesel::delete(
        role_users::table
            .filter(role_users::role_id.eq(role.id))
            .filter(role_users::user_id.eq_any(ids)),
    )
    .execute(conn)?;
    Ok(StatusInfo::done())
}
