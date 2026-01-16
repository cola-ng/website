use chrono::Utc;
use diesel::prelude::*;
use salvo::http::header;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::{conn, with_conn};
use crate::hoops::require_auth;
use crate::models::*;
use crate::{AppConfig, AppResult, DepotExt, JsonResult, empty_ok, json_ok};

pub fn router() -> Router {
    Router::new()
        .push(Router::with_path("register").post(register))
        .push(Router::with_path("login").post(login))
        .push(
            Router::with_path("me")
                .hoop(require_auth)
                .get(me)
                .put(update_me),
        )
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    name: String,
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    user: User,
    access_token: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[handler]
pub async fn login(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> JsonResult<AuthResponse> {
    let input: LoginRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    let email_input = input.email.trim().to_lowercase();
    if email_input.is_empty() {
        return Err(StatusError::bad_request().brief("email is required").into());
    }

    let user: User = with_conn(move |conn| {
        base_users::table
            .filter(base_users::email.eq(email_input))
            .first::<User>(conn)
    })
    .await
    .map_err(|_| StatusError::unauthorized().brief("invalid credentials"))?;

     crate::auth::verify_password(&user, &input.password)
        .await?;

    let config = AppConfig::get();
    let access_token =
        crate::auth::issue_access_token(user.id, &config.jwt_secret, config.jwt_ttl.as_secs())
            .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

    json_ok(AuthResponse {
        user: user.into(),
        access_token,
    })
}

#[handler]
pub async fn register(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> JsonResult<AuthResponse> {
    let input: RegisterRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let email = input.email.trim().to_lowercase();
    if email.is_empty() {
        return Err(StatusError::bad_request().brief("email is required").into());
    }
    if input.password.len() < 8 {
        return Err(StatusError::bad_request()
            .brief("password must be at least 8 characters")
            .into());
    }

    let password_hash = crate::auth::hash_password(&input.password)
        .map_err(|_| StatusError::internal_server_error().brief("failed to create user"))?;

    let new_user = NewUser {
        name: input.name.clone(),
        email: Some(email.clone()),
        phone: None,
        display_name: None,
        inviter_id: None,
        created_by: None,
        updated_by: None,
    };

    let user: User = with_conn(move |conn| {
        let is_first = base_users::table
            .select(diesel::dsl::count_star())
            .first::<i64>(conn)?
            == 0;

        let user = diesel::insert_into(base_users::table)
            .values(&new_user)
            .get_result::<User>(conn)?;

        // if is_first {
        //     use crate::db::schema::role_permissions::dsl as rp;
        //     use crate::db::schema::role_users::dsl as ur;
        //     use crate::db::schema::roles::dsl as r;

        //     let role = diesel::insert_into(r::roles)
        //         .values(&NewRole {
        //             name: "admin".to_string(),
        //         })
        //         .on_conflict_do_nothing()
        //         .get_result::<Role>(conn)
        //         .or_else(|_| r::roles.filter(r::name.eq("admin")).first(conn))?;

        //     let _ = diesel::insert_into(rp::role_permissions)
        //         .values(&NewRolePermission {
        //             role_id: role.id,
        //             operation: "users.delete".to_string(),
        //         })
        //         .on_conflict_do_nothing()
        //         .execute(conn);

        //     let _ = diesel::insert_into(ur::role_users)
        //         .values(&RoleUser {
        //             user_id: user.id,
        //             role_id: role.id,
        //         })
        //         .on_conflict_do_nothing()
        //         .execute(conn);
        // }

        Ok(user)
    })
    .await
    .map_err(|e| {
        if e.contains("UniqueViolation") || e.contains("unique") {
            StatusError::conflict().brief("email already registered")
        } else {
            StatusError::internal_server_error().brief("failed to create user")
        }
    })?;

    let new_password = NewPassword {
        user_id: user.id,
        hash: password_hash,
        created_at: Utc::now(),
    };
    with_conn(move |conn| {
        diesel::insert_into(base_passwords::table)
            .values(&new_password)
            .execute(conn)
    })
    .await?;
    let config = AppConfig::get();
    let access_token =
        crate::auth::issue_access_token(user.id, &config.jwt_secret, config.jwt_ttl.as_secs())
            .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

    Ok(Json(AuthResponse {
        user: user.into(),
        access_token,
    }))
}

#[handler]
pub async fn me(depot: &mut Depot, res: &mut Response) -> JsonResult<User> {
    let user_id = depot.user_id()?;
    let user: User = with_conn(move |conn| {
        base_users::table
            .filter(base_users::id.eq(user_id))
            .first::<User>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("user not found"))?;
    res.headers_mut()
        .insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    Ok(Json(User::from(user)))
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = base_users)]
pub struct UpdateUserProfile {
    pub name: Option<String>,
    pub phone: Option<String>,
}
#[handler]
pub async fn update_me(req: &mut Request, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let input: UpdateUserProfile = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    let user_id = depot.user_id()?;
    let now = Utc::now();

    let updated: User = with_conn(move |conn| {
        diesel::update(base_users::table.filter(base_users::id.eq(user_id)))
            .set(&input)
            .get_result::<User>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update profile"))?;

    res.render(Json(User::from(updated)));
    Ok(())
}
