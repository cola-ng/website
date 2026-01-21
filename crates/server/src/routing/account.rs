use chrono::Utc;
use diesel::prelude::*;
use salvo::oapi::extract::JsonBody;
use salvo::oapi::ToSchema;
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

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    /// User display name
    name: String,
    /// User email address
    email: String,
    /// Password (min 8 characters)
    password: String,
    /// Phone number (optional)
    phone: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AuthResponse {
    /// User information
    user: User,
    /// JWT access token
    access_token: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    /// User email address
    email: String,
    /// User password
    password: String,
}

/// User login with email and password
#[endpoint(tags("Account"))]
pub async fn login(input: JsonBody<LoginRequest>) -> JsonResult<AuthResponse> {
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

/// Register a new user account
#[endpoint(tags("Account"))]
pub async fn register(input: JsonBody<RegisterRequest>) -> JsonResult<AuthResponse> {
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
    let phone: Option<String> = input
        .phone
        .as_ref()
        .map(|p: &String| p.trim().to_string())
        .filter(|p: &String| !p.is_empty());

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

/// Get current user profile
#[endpoint(tags("Account"))]
pub async fn me(depot: &mut Depot) -> JsonResult<User> {
    let user_id = depot.user_id()?;
    let user: User = with_conn(move |conn| {
        base_users::table
            .filter(base_users::id.eq(user_id))
            .first::<User>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("user not found"))?;
    Ok(Json(User::from(user)))
}

#[derive(AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = base_users)]
pub struct UpdateUserProfile {
    /// Updated display name
    pub name: Option<String>,
    /// Updated phone number
    pub phone: Option<String>,
}

/// Update current user profile
#[endpoint(tags("Account"))]
pub async fn update_me(
    input: JsonBody<UpdateUserProfile>,
    depot: &mut Depot,
) -> JsonResult<User> {
    let user_id = depot.user_id()?;

    let input_value = UpdateUserProfile {
        name: input.name.clone(),
        phone: input.phone.clone(),
    };

    let updated: User = with_conn(move |conn| {
        diesel::update(base_users::table.filter(base_users::id.eq(user_id)))
            .set(&input_value)
            .get_result::<User>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update profile"))?;

    json_ok(User::from(updated))
}
