use chrono::Utc;
use horme::prelude::*;
use salvo::oapi::ToSchema;
use salvo::oapi::extract::JsonBody;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::hoops::{is_user_admin, require_auth};
use crate::models::*;
use crate::{AppConfig, DepotExt, JsonResult, json_ok};

mod avatar;
mod oauth;

pub fn router() -> Router {
    Router::new()
        .push(Router::with_path("register").post(register))
        .push(Router::with_path("login").post(login))
        .push(
            Router::with_path("me")
                .hoop(require_auth)
                .get(me)
                .put(update_me)
                .push(
                    Router::with_path("avatar")
                        .get(avatar::show)
                        .post(avatar::upload_avatar)
                        .delete(avatar::delete_avatar),
                )
                .push(
                    Router::with_path("oauth")
                        .get(oauth::list_identities)
                        .post(oauth::link_identity)
                        .push(
                            Router::with_path("{provider}/start")
                                .post(super::auth::social::link_start),
                        )
                        .push(Router::with_path("{id}").delete(oauth::unlink_identity)),
                )
                .push(Router::with_path("account-status").get(oauth::get_account_status))
                .push(Router::with_path("change-password").post(change_password)),
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
    /// Whether user has admin privileges
    is_admin: bool,
}

#[derive(Serialize, ToSchema)]
pub struct MeResponse {
    /// User information
    #[serde(flatten)]
    user: User,
    /// Whether user has admin privileges
    is_admin: bool,
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
pub async fn login(req: &mut Request, input: JsonBody<LoginRequest>) -> JsonResult<AuthResponse> {
    let email_input = input.email.trim().to_lowercase();
    if email_input.is_empty() || email_input.len() > 320 {
        return Err(StatusError::bad_request().brief("email is required").into());
    }

    crate::auth::limit_password_attempt(
        &req.remote_addr()
            .ip()
            .map(|ip| ip.to_string())
            .unwrap_or_default(),
        &email_input,
    )?;
    let user: User = with_conn!(move |conn| {
        base_users::table
            .filter(base_users::email.eq(email_input))
            .first::<User>(conn)
            .await
    })
    .await
    .map_err(|_| StatusError::unauthorized().brief("invalid credentials"))?;

    crate::auth::verify_password(&user, &input.password).await?;

    let config = AppConfig::get();
    let access_token = crate::auth::issue_access_token(
        user.id,
        user.token_version,
        &config.jwt_secret,
        config.jwt_ttl.as_secs(),
    )
    .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

    let is_admin = is_user_admin(user.id).await;

    json_ok(AuthResponse {
        user: user.into(),
        access_token,
        is_admin,
    })
}

// Custom error for distinguishing between email and phone conflicts
const ERR_EMAIL_EXISTS: &str = "EMAIL_EXISTS";
const ERR_PHONE_EXISTS: &str = "PHONE_EXISTS";

/// Register a new user account
#[endpoint(tags("Account"))]
pub async fn register(
    req: &mut Request,
    input: JsonBody<RegisterRequest>,
) -> JsonResult<AuthResponse> {
    let email = input.email.trim().to_lowercase();
    if email.is_empty() || email.len() > 320 {
        return Err(StatusError::bad_request().brief("email is required").into());
    }
    if !(8..=1024).contains(&input.password.len()) {
        return Err(StatusError::bad_request()
            .brief("password must contain 8 to 1024 bytes")
            .into());
    }

    crate::auth::limit_password_attempt(
        &req.remote_addr()
            .ip()
            .map(|ip| ip.to_string())
            .unwrap_or_default(),
        &email,
    )?;
    // Normalize phone: trim and filter empty strings
    let phone: Option<String> = input
        .phone
        .as_ref()
        .map(|p: &String| p.trim().to_string())
        .filter(|p: &String| !p.is_empty());

    let password_hash = crate::auth::hash_password(&input.password)
        .await
        .map_err(|error| if error.contains("busy") {
            StatusError::too_many_requests().brief("password service busy; try again")
        } else { StatusError::service_unavailable().brief("password service unavailable") })?;

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

    let user: User = with_conn!(move |conn| horme::transaction(conn, async |conn| {
        // Check if email already exists
        let email_exists = base_users::table
            .filter(base_users::email.eq(&email_for_check))
            .select(base_users::id)
            .first::<i64>(conn)
            .await
            .optional()?
            .is_some();

        if email_exists {
            return Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                Box::new(ERR_EMAIL_EXISTS.to_string()),
            )
            .into());
        }

        // Check if phone already exists (only if phone is provided)
        if let Some(ref phone_val) = phone_for_check {
            let phone_exists = base_users::table
                .filter(base_users::phone.eq(phone_val))
                .select(base_users::id)
                .first::<i64>(conn)
                .await
                .optional()?
                .is_some();

            if phone_exists {
                return Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    Box::new(ERR_PHONE_EXISTS.to_string()),
                )
                .into());
            }
        }

        // Create user
        let user = diesel::insert_into(base_users::table)
            .values(&new_user)
            .get_result::<User>(conn)
            .await?;

        // Create password in the same transaction
        let new_password = NewPassword {
            user_id: user.id,
            hash: password_hash,
            created_at: Utc::now(),
        };
        diesel::insert_into(base_passwords::table)
            .values(&new_password)
            .execute(conn)
            .await?;

        Ok(user)
    }))
    .await
    .map_err(|e| {
        if e.contains(ERR_EMAIL_EXISTS) {
            StatusError::conflict().brief("email already registered")
        } else if e.contains(ERR_PHONE_EXISTS) {
            StatusError::conflict().brief("phone already registered")
        } else if e.contains("base_users_name_key") {
            StatusError::conflict().brief("name already registered")
        } else if e.contains("UniqueViolation") || e.contains("unique") {
            StatusError::conflict().brief("email or phone already registered")
        } else {
            StatusError::internal_server_error().brief("failed to create user")
        }
    })?;

    let config = AppConfig::get();
    let access_token = crate::auth::issue_access_token(
        user.id,
        user.token_version,
        &config.jwt_secret,
        config.jwt_ttl.as_secs(),
    )
    .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

    // New users are never admins
    Ok(Json(AuthResponse {
        user: user.into(),
        access_token,
        is_admin: false,
    }))
}

/// Get current user profile
#[endpoint(tags("Account"))]
pub async fn me(depot: &mut Depot) -> JsonResult<MeResponse> {
    let user_id = depot.user_id()?;
    let user: User = with_conn!(move |conn| {
        base_users::table
            .filter(base_users::id.eq(user_id))
            .first::<User>(conn)
            .await
    })
    .await
    .map_err(|_| StatusError::not_found().brief("user not found"))?;

    let is_admin = is_user_admin(user_id).await;

    json_ok(MeResponse {
        user: user.into(),
        is_admin,
    })
}

#[derive(AsChangeset, Deserialize, ToSchema)]
#[diesel(table_name = base_users)]
pub struct UpdateUserProfile {
    /// Updated display name
    pub name: Option<String>,
    /// Updated phone number
    pub phone: Option<String>,
    pub handle: Option<String>,
}

/// Update current user profile
#[endpoint(tags("Account"))]
pub async fn update_me(input: JsonBody<UpdateUserProfile>, depot: &mut Depot) -> JsonResult<User> {
    let user_id = depot.user_id()?;

    let input_value = UpdateUserProfile {
        name: input.name.clone(),
        phone: input.phone.clone(),
        handle: input
            .handle
            .as_deref()
            .map(|h| h.trim().to_ascii_lowercase()),
    };
    if let Some(handle) = &input_value.handle {
        if handle
            .strip_prefix("user_")
            .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            && handle != &format!("user_{user_id}")
        {
            return Err(StatusError::bad_request()
                .brief("user_数字 为系统保留格式，请选择其他 Handle")
                .into());
        }
        if !(3..=32).contains(&handle.len())
            || !handle.as_bytes()[0].is_ascii_lowercase()
            || !handle
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
        {
            return Err(StatusError::bad_request()
                .brief("Handle 需为 3–32 位，以字母开头，仅含英文字母、数字、下划线或短横线")
                .into());
        }
    }

    let updated: User = with_conn!(move |conn| {
        diesel::update(base_users::table.filter(base_users::id.eq(user_id)))
            .set(&input_value)
            .get_result::<User>(conn)
            .await
    })
    .await
    .map_err(|e| {
        if e.contains("base_users_handle_unique") {
            StatusError::conflict().brief("此 Handle 已被使用，请选择其他标识")
        } else if e.contains("base_users_name_key") {
            StatusError::conflict().brief("name already registered")
        } else {
            StatusError::internal_server_error().brief("failed to update profile")
        }
    })?;

    json_ok(User::from(updated))
}

#[derive(Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    /// Current password (required if user already has a password)
    current_password: Option<String>,
    /// New password (min 8 characters)
    new_password: String,
}

#[derive(Serialize, ToSchema)]
pub struct ChangePasswordResponse {
    success: bool,
    access_token: String,
}

/// Change user password
#[endpoint(tags("Account"))]
pub async fn change_password(
    input: JsonBody<ChangePasswordRequest>,
    depot: &mut Depot,
) -> JsonResult<ChangePasswordResponse> {
    let user_id = depot.user_id()?;

    if !(8..=1024).contains(&input.new_password.len()) {
        return Err(StatusError::bad_request()
            .brief("password must contain 8 to 1024 bytes")
            .into());
    }

    // Verify outside the transaction, then compare the version under a row lock.
    // A concurrent password change must invalidate this attempt, even for OAuth accounts.
    let (old_hash, version) = with_conn!(move |conn| {
        let version = base_users::table
            .find(user_id)
            .select(base_users::token_version)
            .first::<i64>(conn)
            .await?;
        let hash = base_passwords::table
            .filter(base_passwords::user_id.eq(user_id))
            .select(base_passwords::hash)
            .first::<String>(conn)
            .await
            .optional()?;
        Ok((hash, version))
    })
    .await
    .map_err(|_| StatusError::service_unavailable().brief("cannot verify password state"))?;
    if let Some(hash) = old_hash {
        crate::auth::verify_hash(hash, input.current_password.clone().unwrap_or_default()).await?;
    }
    let new_hash = crate::auth::hash_password(&input.new_password)
        .await
        .map_err(|_| {
            StatusError::too_many_requests().brief("password service busy or invalid password")
        })?;
    let next_version = with_conn!(move |conn| horme::transaction(conn, async |conn| {
        let current = base_users::table
            .find(user_id)
            .for_update()
            .select(base_users::token_version)
            .first::<i64>(conn)
            .await?;
        if current != version {
            return Err(horme::Error::from(
                diesel::result::Error::RollbackTransaction,
            ));
        }
        diesel::insert_into(base_passwords::table)
            .values(NewPassword {
                user_id,
                hash: new_hash.clone(),
                created_at: Utc::now(),
            })
            .on_conflict(base_passwords::user_id)
            .do_update()
            .set((
                base_passwords::hash.eq(new_hash),
                base_passwords::created_at.eq(Utc::now()),
            ))
            .execute(conn)
            .await?;
        let next = diesel::update(base_users::table.find(user_id))
            .set(base_users::token_version.eq(base_users::token_version + 1))
            .returning(base_users::token_version)
            .get_result::<i64>(conn)
            .await?;
        Ok(next)
    }))
    .await
    .map_err(|error| {
        if error.to_ascii_lowercase().contains("rollback") {
            StatusError::conflict().brief("password changed concurrently; retry")
        } else {
            StatusError::service_unavailable().brief("password update failed; retry")
        }
    })?;
    let config = AppConfig::get();
    let access_token = crate::auth::issue_access_token(
        user_id,
        next_version,
        &config.jwt_secret,
        config.jwt_ttl.as_secs(),
    )
    .map_err(|_| StatusError::internal_server_error())?;
    json_ok(ChangePasswordResponse {
        success: true,
        access_token,
    })
}
