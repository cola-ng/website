use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::models::*;
use crate::permission::Accessible;
use crate::db::schema::*;
use crate::utils::validator;
use crate::{AppError, AppResult, DepotExt, JsonResult, PagedResult, StatusInfo, db};

pub mod permission;
pub mod user;

pub fn authed_root(path: impl Into<String>) -> Router {
    Router::with_path(path).get(list).post(create).delete(bulk_delete).push(
        Router::with_path("{role_id:u64}")
            .get(show)
            .patch(update)
            .delete(delete)
            .push(Router::with_path("permissions").get(permission::list).patch(permission::update))
            .push(Router::with_path("users").get(user::list))
            .push(Router::with_path("add_users").post(user::add))
            .push(Router::with_path("remove_users").post(user::remove)),
    )
}

#[endpoint(tags("role"))]
pub fn show(role_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<Role> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let role = roles::table
        .find(role_id.into_inner())
        .first::<Role>(conn)?
        .assign_to(cuser, "view", conn)?;
    Ok(Json(role))
}
#[endpoint(tags("role"))]
pub fn list(req: &mut Request, depot: &mut Depot) -> PagedResult<Role> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let data = query_pagation_data!(
        req,
        res,
        Role,
        roles::table.permit(cuser, "view", conn)?,
        "updated_at desc",
        ROLE_FILTER_FIELDS.clone(),
        ROLE_JOINED_OPTIONS.clone(),
        ID_NAME_SEARCH_TMPL,
        conn
    );
    Ok(Json(data))
}
#[endpoint(tags("role"))]
pub fn delete(role_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;

    let role = roles::table
        .find(role_id.into_inner())
        .first::<Role>(conn)?
        .assign_to(cuser, "delete", conn)?;
    db::delete_role(role.id, cuser.id, conn)?;
    Ok(StatusInfo::done())
}
#[endpoint(tags("role"))]
pub async fn bulk_delete(req: &mut Request, depot: &mut Depot) -> AppResult<StatusInfo> {
    let mut conn = db::conn()?;
    let info = bulk_delete_records!(req, depot, res, roles, Role, db::delete_role, &mut conn);
    Ok(info)
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct CreateInData {
    name: String,
    realm_id: i64,
    description: Option<String>,
}
#[endpoint(tags("role"))]
pub fn create(pdata: JsonBody<CreateInData>, depot: &mut Depot) -> JsonResult<Role> {
    let pdata = pdata.into_inner();
    if let Err(msg) = validator::validate_generic_name(&pdata.name) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let realm = realms::table.find(pdata.realm_id).first::<Realm>(conn)?;
    require_all_permitted_in_realm!(res, cuser, &realm, "create", crate::permission::roles, conn);

    let role = conn.transaction::<_, AppError, _>(|conn| {
        let query = roles::table
            .filter(roles::name.eq(&pdata.name))
            .filter(roles::realm_id.eq(&pdata.realm_id));
        if diesel_exists!(query, conn) {
            return Err(StatusError::conflict().brief("This name is already taken, please try another. ").into());
        }
        let new_role = NewRole {
            code: None,
            name: &pdata.name,
            kind: Some("custom"),
            realm_id: pdata.realm_id,
            description: pdata.description.as_deref(),
            owner_id: cuser.id,
            updated_by: Some(cuser.id),
            created_by: Some(cuser.id),
        };
        let role = diesel::insert_into(roles::table).values(&new_role).get_result::<Role>(conn)?;
        Ok(role)
    })?;
    Ok(Json(role))
}

#[derive(AsChangeset, Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
#[diesel(table_name = roles)]
struct UpdateInData {
    name: Option<String>,
    description: Option<String>,
    updated_by: Option<i64>,
    updated_at: Option<DateTime<Utc>>,
}
#[endpoint(tags("role"))]
pub fn update(role_id: PathParam<i64>, pdata: JsonBody<UpdateInData>, depot: &mut Depot) -> JsonResult<Role> {
    let mut pdata = pdata.into_inner();
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let exist_role = roles::table
        .find(role_id.into_inner())
        .first::<Role>(conn)?
        .assign_to(cuser, "edit", conn)?;

    pdata.updated_by = Some(cuser.id);
    pdata.updated_at = Some(Utc::now());

    let role = conn.transaction::<_, AppError, _>(|conn| {
        if let Some(name) = &pdata.name {
            let query = roles::table
                .filter(roles::realm_id.eq(exist_role.realm_id))
                .filter(roles::id.ne(exist_role.id))
                .filter(roles::name.eq(name));
            if diesel_exists!(query, conn) {
                return Err(StatusError::conflict().brief("This name is already taken, please try another.").into());
            }
        }
        let role = diesel::update(&exist_role).set(&pdata).get_result::<Role>(conn)?;
        Ok(role)
    })?;
    Ok(Json(role))
}
