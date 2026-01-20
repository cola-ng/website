use chrono::Utc;
use diesel::prelude::*;
use salvo::http::header;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::hoops::require_auth;
use crate::models::*;
use crate::{AppConfig, AppResult, DepotExt, JsonResult, json_ok};

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
    phone: Option<String>,
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
    _res: &mut Response,
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

    crate::auth::verify_password(&user, &input.password).await?;

    let config = AppConfig::get();
    let access_token =
        crate::auth::issue_access_token(user.id, &config.jwt_secret, config.jwt_ttl.as_secs())
            .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

    json_ok(AuthResponse {
        user: user.into(),
        access_token,
    })
}

// Custom error for distinguishing between email and phone conflicts
const ERR_EMAIL_EXISTS: &str = "EMAIL_EXISTS";
const ERR_PHONE_EXISTS: &str = "PHONE_EXISTS";

#[handler]
pub async fn register(
    req: &mut Request,
    _depot: &mut Depot,
    _res: &mut Response,
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

    // Normalize phone: trim and filter empty strings
    let phone = input
        .phone
        .as_ref()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty());

    let password_hash = crate::auth::hash_password(&input.password)
        .map_err(|_| StatusError::internal_server_error().brief("failed to create user"))?;

    let email_for_check = email.clone();
    let phone_for_check = phone.clone();
    let new_user = NewUser {
        name: input.name.clone(),
        email: Some(email.clone()),
        phone: phone.clone(),
        display_name: None,
        inviter_id: None,
        created_by: None,
        updated_by: None,
    };

    let user: User = with_conn(move |conn| {
        conn.transaction(|conn| {
            // Check if email already exists
            let email_exists = base_users::table
                .filter(base_users::email.eq(&email_for_check))
                .select(base_users::id)
                .first::<i64>(conn)
                .optional()?
                .is_some();

            if email_exists {
                return Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    Box::new(ERR_EMAIL_EXISTS.to_string()),
                ));
            }

            // Check if phone already exists (only if phone is provided)
            if let Some(ref phone_val) = phone_for_check {
                let phone_exists = base_users::table
                    .filter(base_users::phone.eq(phone_val))
                    .select(base_users::id)
                    .first::<i64>(conn)
                    .optional()?
                    .is_some();

                if phone_exists {
                    return Err(diesel::result::Error::DatabaseError(
                        diesel::result::DatabaseErrorKind::UniqueViolation,
                        Box::new(ERR_PHONE_EXISTS.to_string()),
                    ));
                }
            }

            // Create user
            let user = diesel::insert_into(base_users::table)
                .values(&new_user)
                .get_result::<User>(conn)?;

            // Create password in the same transaction
            let new_password = NewPassword {
                user_id: user.id,
                hash: password_hash,
                created_at: Utc::now(),
            };
            diesel::insert_into(base_passwords::table)
                .values(&new_password)
                .execute(conn)?;

            Ok(user)
        })
    })
    .await
    .map_err(|e| {
        if e.contains(ERR_EMAIL_EXISTS) {
            StatusError::conflict().brief("email already registered")
        } else if e.contains(ERR_PHONE_EXISTS) {
            StatusError::conflict().brief("phone already registered")
        } else if e.contains("UniqueViolation") || e.contains("unique") {
            StatusError::conflict().brief("email or phone already registered")
        } else {
            StatusError::internal_server_error().brief("failed to create user")
        }
    })?;

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
    let _now = Utc::now();

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
