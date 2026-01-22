mod access_token;
mod avatar;
mod email;
mod email_subscription;
mod permission;
mod phone;
mod share_link;

use chrono::Utc;
use diesel::prelude::*;
use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db;
use crate::models::*;
use crate::permission::Accessible;
use crate::routers::full_routed;
use crate::db::schema::*;
use crate::things::realm::create_user_realm;
use crate::utils::{password, validator};
use crate::{AppError, AppResult, DepotExt, JsonResult, PagedResult, StatusInfo, things};

pub fn authed_root(path: impl Into<String>) -> Router {
    Router::with_path(path)
        .get(list)
        .then(|root| if full_routed() { root.post(create).delete(bulk_delete) } else { root })
        .push(Router::with_path("invite").post(invite))
        .push(
            Router::with_path("{user_id:u64}")
                .get(show)
                .patch(update)
                .delete(delete)
                .then(|router| {
                    if full_routed() {
                        router
                            .push(Router::with_path("avatars/upload").post(avatar::upload).delete(avatar::delete))
                            .push(Router::with_path("reject").post(reject).delete(reject))
                            .push(Router::with_path("set_limited").post(set_limited))
                            .push(Router::with_path("set_locked").post(set_locked))
                            .push(Router::with_path("set_disabled").post(set_disabled))
                            .push(Router::with_path("set_points").post(set_points))
                            .push(
                                Router::with_path("email_subscriptions")
                                    .get(email_subscription::list)
                                    .post(email_subscription::upsert),
                            )
                    } else {
                        router
                    }
                })
                .push(Router::with_path(r#"avatars/{width:u64}x{height:u64}.{ext}"#).get(avatar::show))
                .push(Router::with_path("has_partial_permissions").post(permission::has_partial_permissions))
                .push(Router::with_path("has_record_permissions").post(permission::has_record_permissions))
                .push(Router::with_path("roles").get(list_roles))
                .push(Router::with_path("emails").get(email::list).post(email::create))
                .push(Router::with_path("emails/{email_id:u64}").delete(email::delete))
                .push(Router::with_path("phones").get(phone::list).post(phone::create))
                .push(Router::with_path("phones/{phone_id:u64}").delete(phone::delete)),
        )
}

pub fn public_root(path: impl Into<String>) -> Router {
    Router::with_path(path).push(Router::with_path("is_other_taken").get(is_other_taken))
}

#[endpoint(tags("user"))]
pub fn show(user_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<User> {
    // 防止在模拟用户形式下无法拿到自己原本的用户信息。前端界面需要。
    let cuser = depot.jwt_user()?;
    // let jwt_user = depot.jwt_user();
    let conn = &mut db::conn()?;
    let user = if cuser.in_kernel {
        users::table
            .find(user_id.into_inner())
            .first::<User>(conn)?
            .assign_to(cuser, "view", conn)?
    } else {
        users::table
            .filter(users::id.eq(user_id.into_inner()))
            .filter(users::is_disabled.eq(false))
            .first::<User>(conn)?
            .assign_to(cuser, "view", conn)?
    };
    Ok(Json(user))
}
#[endpoint(tags("user"))]
pub async fn list(req: &mut Request, depot: &mut Depot) -> PagedResult<User> {
    // let cuser = depot.current_user()?;
    // 防止在模拟用户形式下无法拿到自己原本的用户信息。前端界面需要。
    let cuser = depot.jwt_user()?;
    // let jwt_user = depot.jwt_user();
    let conn = &mut db::conn()?;
    let users = if !cuser.in_kernel {
        let query = users::table
            .permit(cuser, "view", conn)?
            .filter(users::is_root.eq(false))
            .filter(users::is_disabled.eq(false));
        query_pagation_data!(
            req,
            res,
            User,
            query,
            "updated_at desc",
            USER_FILTER_FIELDS.clone(),
            USER_JOINED_OPTIONS.clone(),
            USER_SEARCH_TMPL,
            conn
        )
    } else {
        let query = users::table.permit(cuser, "view", conn)?.filter(users::is_root.eq(false));
        query_pagation_data!(
            req,
            res,
            User,
            query,
            "updated_at desc",
            USER_FILTER_FIELDS.clone(),
            USER_JOINED_OPTIONS.clone(),
            USER_SEARCH_TMPL,
            conn
        )
    };
    Ok(Json(users))
}
#[endpoint(tags("user"))]
pub fn reject(user_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let conn = &mut db::conn()?;
    let user = users::table.find(user_id.into_inner()).first::<User>(conn)?;
    let cuser = depot.current_user()?;
    if !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }
    db::delete_user(user.id, cuser.id, conn)?;

    Ok(StatusInfo::done())
}
#[endpoint(tags("user"))]
pub fn delete(user_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;

    let user = users::table
        .find(user_id.into_inner())
        .first::<User>(conn)?
        .assign_to(cuser, "delete", conn)?;
    db::delete_user(user.id, cuser.id, conn)?;
    Ok(StatusInfo::done())
}
#[endpoint(tags("user"))]
pub async fn bulk_delete(req: &mut Request, depot: &mut Depot) -> AppResult<StatusInfo> {
    let conn = &mut db::conn()?;
    let info = bulk_delete_records!(req, depot, res, users, User, db::delete_user, conn);
    Ok(info)
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct OauthBindData {
    platform: String,
    access_token: String,
}
#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct CreateInData {
    ident_name: String,
    display_name: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    email: PostedEmail,
    #[serde(default)]
    phone: PostedPhone,
    oauth_bind: Option<OauthBindData>,
    #[serde(default)]
    #[salvo(schema(value_type = Option<Object>))]
    profile: Value,
}
#[endpoint(tags("user"))]
pub fn create(pdata: JsonBody<CreateInData>, depot: &mut Depot) -> JsonResult<User> {
    let pdata = pdata.into_inner();
    if let Err(msg) = validator::validate_email(&pdata.email.value) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    let cuser = depot.current_user()?;
    if !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }
    let conn = &mut db::conn()?;

    if let Err(msg) = validator::validate_password(&pdata.password) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    let pwd = password::hash(&pdata.password);
    if pwd.is_err() {
        return Err(StatusError::internal_server_error().brief("password hash has error").into());
    }
    let pwd = pwd.unwrap();
    let user = conn.transaction::<User, AppError, _>(|conn| {
        let ident_name = if pdata.ident_name.is_empty() {
            crate::generate_ident_name(conn)?
        } else {
            check_ident_name_preserved!(&pdata.ident_name);
            check_ident_name_other_taken!(None, &pdata.ident_name, conn);
            pdata.ident_name.to_lowercase()
        };
        check_email_other_taken!(None, &pdata.email.value, conn);
        // if !pdata.phone.value.is_empty() {
        //     check_phone_other_taken!(None, &pdata.phone.value, conn);
        // }

        let new_user = NewUser {
            ident_name: &ident_name,
            display_name: &pdata.display_name,
            password: &pwd,
            in_kernel: false,
            is_root: false,
            is_verified: false,
            // is_limited: true,
            is_limited: false,
            inviter_id: Some(cuser.id),
            invite_replied: Some(true),
            profile: pdata.profile.clone(),
            updated_by: Some(cuser.id),
            created_by: Some(cuser.id),

            points: 0,
        };
        let mut new_user = diesel::insert_into(users::table).values(&new_user).get_result::<User>(conn)?;
        let new_email = NewEmail {
            user_id: new_user.id,
            value: &pdata.email.value,
            is_master: true,
            is_verified: false,
            is_subscribed: pdata.email.is_subscribed,
            updated_by: Some(cuser.id),
            created_by: Some(cuser.id),
        };
        diesel::insert_into(emails::table).values(&new_email).get_result::<Email>(conn)?;
        if !pdata.phone.value.is_empty() {
            let new_phone = NewPhone {
                user_id: new_user.id,
                value: &pdata.phone.value,
                is_master: true,
                is_verified: false,
                is_subscribed: pdata.phone.is_subscribed,
                updated_by: None,
                created_by: None,
            };
            diesel::insert_into(phones::table).values(&new_phone).get_result::<Phone>(conn)?;
        }

        if let Some(obind) = pdata.oauth_bind {
            diesel::update(
                oauth_users::table
                    .filter(oauth_users::access_token.eq(&obind.access_token))
                    .filter(oauth_users::platform.eq(&obind.platform)),
            )
            .set(oauth_users::user_id.eq(new_user.id))
            .execute(conn)
            .ok();
        }
        Ok(new_user)
    })?;
    Ok(Json(user))
}

#[derive(AsChangeset, Deserialize, ToSchema, Debug)]
#[diesel(table_name = users)]
struct UpdateInData {
    display_name: Option<String>,
    #[salvo(schema(value_type = Option<Object>))]
    profile: Option<Value>,
}
#[endpoint(tags("user"))]
pub fn update(user_id: PathParam<i64>, pdata: JsonBody<UpdateInData>, depot: &mut Depot) -> JsonResult<User> {
    let pdata = pdata.into_inner();
    let cuser = depot.current_user()?.must_in_kernel()?;
    let conn = &mut db::conn()?;
    let user = users::table
        .find(user_id.into_inner())
        .first::<User>(conn)?
        .assign_to(cuser, "edit", conn)?;
    let user = diesel::update(&user).set(&pdata).get_result::<User>(conn)?;
    Ok(Json(user))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct SetPointsInData {
    points: i64,
}
#[endpoint(tags("user"))]
pub fn set_points(user_id: PathParam<i64>, pdata: JsonBody<SetPointsInData>, depot: &mut Depot) -> JsonResult<User> {
    let pdata = pdata.into_inner();
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let user = users::table.find(user_id.into_inner()).first::<User>(conn)?;
    if user.id == cuser.id || !user.permitted(cuser, "edit", conn)? || !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }
    let user = diesel::update(&user).set(users::points.eq(pdata.points)).get_result::<User>(conn)?;
    Ok(Json(user))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct SetLimitedInData {
    value: bool,
}
#[endpoint(tags("user"))]
pub fn set_limited(user_id: PathParam<i64>, pdata: JsonBody<SetLimitedInData>, depot: &mut Depot) -> JsonResult<User> {
    let pdata = pdata.into_inner();
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let user = users::table.find(user_id.into_inner()).first::<User>(conn)?;
    if user.id == cuser.id || !user.permitted(cuser, "edit", conn)? || !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }
    let user = if pdata.value {
        diesel::update(&user)
            .set((users::is_limited.eq(pdata.value), users::limited_at.eq(Utc::now())))
            .get_result::<User>(conn)?
    } else {
        diesel::update(&user).set(users::is_limited.eq(pdata.value)).get_result::<User>(conn)?
    };
    Ok(Json(user))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct SetLockedInData {
    value: bool,
}
#[endpoint(tags("user"))]
pub fn set_locked(user_id: PathParam<i64>, pdata: JsonBody<SetLockedInData>, depot: &mut Depot) -> JsonResult<User> {
    let pdata = pdata.into_inner();
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let user = users::table.find(user_id.into_inner()).first::<User>(conn)?;
    if user.id == cuser.id || !user.permitted(cuser, "edit", conn)? || !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }
    let user = if pdata.value {
        diesel::update(&user)
            .set((users::is_locked.eq(pdata.value), users::locked_at.eq(Utc::now())))
            .get_result::<User>(conn)?
    } else {
        diesel::update(&user).set(users::is_locked.eq(pdata.value)).get_result::<User>(conn)?
    };
    Ok(Json(user))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct SetDisabledInData {
    value: bool,
}
#[endpoint(tags("user"))]
pub fn set_disabled(user_id: PathParam<i64>, pdata: JsonBody<SetDisabledInData>, depot: &mut Depot) -> JsonResult<User> {
    let pdata = pdata.into_inner();
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let user = users::table.find(user_id.into_inner()).first::<User>(conn)?;
    if user.id == cuser.id || !user.permitted(cuser, "edit", conn)? || !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }
    let user = if pdata.value {
        diesel::update(&user)
            .set((
                users::is_disabled.eq(pdata.value),
                users::disabled_at.eq(Utc::now()),
                users::disabled_by.eq(cuser.id),
            ))
            .get_result::<User>(conn)?
    } else {
        diesel::update(&user).set(users::is_disabled.eq(pdata.value)).get_result::<User>(conn)?
    };
    Ok(Json(user))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct InviteInData {
    display_name: String,
    #[serde(default)]
    email: String,
    #[serde(default = "crate::default_as_false")]
    in_kernel: bool,
    #[serde(default)]
    #[salvo(schema(value_type = Object, additional_properties = true))]
    profile: Value,
}

#[endpoint(tags("user"))]
pub fn is_other_taken(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let user_id = req.query::<i64>("user_id");
    let ident_name: String = req.query("ident_name").unwrap_or_default();
    let email_value: String = req.query("email").unwrap_or_default();
    let phone_value: String = req.query("phone").unwrap_or_default();
    let mut taken = false;
    let conn = &mut db::conn()?;
    if !ident_name.is_empty() {
        taken = validator::is_ident_name_other_taken(user_id, &ident_name, conn)?;
    }
    if !taken && !email_value.is_empty() {
        taken = validator::is_email_other_taken(user_id, &email_value, conn)?;
    }
    if !taken && !phone_value.is_empty() {
        taken = validator::is_phone_other_taken(user_id, &phone_value, conn)?;
    }
    #[derive(Serialize, Debug)]
    struct ResultData {
        taken: bool,
    }
    res.render(Json(ResultData { taken }));
    Ok(())
}

#[endpoint(tags("user"))]
pub async fn list_roles(user_id: PathParam<i64>, req: &mut Request, depot: &mut Depot) -> PagedResult<Role> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let user = users::table.find(user_id.into_inner()).first::<User>(conn)?;

    let query = if cuser.in_kernel || cuser.id == user.id {
        roles::table
            .filter(roles::id.eq_any(role_users::table.filter(role_users::user_id.eq(user.id)).select(role_users::role_id)))
            .into_boxed()
    } else {
        let realm_ids = realms::table
            .permit(cuser, "users.roles.view", conn)?
            .select(realms::id)
            .get_results::<i64>(conn)?;
        if !realm_ids.is_empty() {
            roles::table
                .filter(
                    roles::id.eq_any(
                        role_users::table
                            .filter(role_users::user_id.eq(user.id))
                            .filter(role_users::realm_id.eq_any(realm_ids))
                            .select(role_users::role_id),
                    ),
                )
                .into_boxed()
        } else {
            roles::table.filter(roles::id.is_null()).into_boxed()
        }
    };
    let data = query_pagation_data!(
        req,
        res,
        Role,
        query,
        "updated_at desc",
        ROLE_FILTER_FIELDS.clone(),
        ROLE_JOINED_OPTIONS.clone(),
        ID_NAME_SEARCH_TMPL,
        conn
    );
    Ok(Json(data))
}
