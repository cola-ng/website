use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::http::StatusCode;
use salvo::http::header;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::auth;
use crate::config::AppConfig;
use crate::db::DbPool;
use crate::db::with_conn;
use crate::models::{
    DesktopAuthCode, LearningRecord, NewDesktopAuthCode, NewLearningRecord, NewOauthIdentity,
    NewRole, NewRolePermission, NewUser, NewUserRole, OauthIdentity, UpdateUserProfile, User,
};
use crate::schema;

pub mod account;

#[handler]
pub async fn cors(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("*")
        .to_string();
    res.headers_mut()
        .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.parse().unwrap());
    res.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        "content-type, authorization".parse().unwrap(),
    );
    res.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        "GET,POST,PUT,PATCH,DELETE,OPTIONS".parse().unwrap(),
    );
    res.headers_mut()
        .insert(header::ACCESS_CONTROL_MAX_AGE, "86400".parse().unwrap());
    res.headers_mut()
        .insert(header::VARY, "origin".parse().unwrap());

    if req.method() == salvo::http::Method::OPTIONS {
        res.status_code(StatusCode::NO_CONTENT);
        ctrl.skip_rest();
        return;
    }
    ctrl.call_next(req, depot, res).await;
}

#[handler]
pub async fn health(res: &mut Response) {
    res.render(Json(json!({ "ok": true })));
}

#[derive(Serialize)]
pub struct PublicUser {
    id: Uuid,
    email: String,
    name: Option<String>,
    phone: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<User> for PublicUser {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            email: value.email,
            name: value.name,
            phone: value.phone,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    email: String,
    password: String,
    name: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    user: PublicUser,
    access_token: String,
}

fn bad_request(message: &str) -> StatusError {
    StatusError::bad_request().brief(message)
}

#[handler]
pub async fn register(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let input: RegisterRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;

    let email = input.email.trim().to_lowercase();
    if email.is_empty() {
        return Err(bad_request("email is required"));
    }
    if input.password.len() < 8 {
        return Err(bad_request("password must be at least 8 characters"));
    }

    let password_hash = auth::hash_password(&input.password)
        .map_err(|_| StatusError::internal_server_error().brief("failed to create user"))?;

    let pool = get_pool(depot)?;
    let new_user = NewUser {
        email: email.clone(),
        password_hash,
        name: input.name.clone(),
        phone: None,
    };

    let user: User = with_conn(pool, move |conn| {
        use schema::users::dsl::*;
        let is_first = users.select(diesel::dsl::count_star()).first::<i64>(conn)? == 0;

        let user = diesel::insert_into(users)
            .values(&new_user)
            .get_result::<User>(conn)?;

        if is_first {
            use crate::schema::role_permissions::dsl as rp;
            use crate::schema::roles::dsl as r;
            use crate::schema::user_roles::dsl as ur;

            let role = diesel::insert_into(r::roles)
                .values(&NewRole {
                    name: "admin".to_string(),
                })
                .on_conflict_do_nothing()
                .get_result::<crate::models::Role>(conn)
                .or_else(|_| r::roles.filter(r::name.eq("admin")).first(conn))?;

            let _ = diesel::insert_into(rp::role_permissions)
                .values(&NewRolePermission {
                    role_id: role.id,
                    operation: "users.delete".to_string(),
                })
                .on_conflict_do_nothing()
                .execute(conn);

            let _ = diesel::insert_into(ur::user_roles)
                .values(&NewUserRole {
                    user_id: user.id,
                    role_id: role.id,
                })
                .on_conflict_do_nothing()
                .execute(conn);
        }

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

    let config = get_config(depot)?;
    let access_token =
        auth::issue_access_token(user.id, &config.jwt_secret, config.jwt_ttl.as_secs())
            .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

    res.render(Json(AuthResponse {
        user: user.into(),
        access_token,
    }));
    Ok(())
}

#[handler]
pub async fn login(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let input: LoginRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;
    let email_input = input.email.trim().to_lowercase();
    if email_input.is_empty() {
        return Err(bad_request("email is required"));
    }

    let pool = get_pool(depot)?;
    let password = input.password.clone();

    let user: User = with_conn(pool, move |conn| {
        use schema::users::dsl::*;
        users.filter(email.eq(email_input)).first::<User>(conn)
    })
    .await
    .map_err(|_| StatusError::unauthorized().brief("invalid credentials"))?;

    let ok = auth::verify_password(&password, &user.password_hash)
        .map_err(|_| StatusError::unauthorized().brief("invalid credentials"))?;
    if !ok {
        return Err(StatusError::unauthorized().brief("invalid credentials"));
    }

    let config = get_config(depot)?;
    let access_token =
        auth::issue_access_token(user.id, &config.jwt_secret, config.jwt_ttl.as_secs())
            .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

    res.render(Json(AuthResponse {
        user: user.into(),
        access_token,
    }));
    Ok(())
}

#[handler]
pub async fn auth_required(
    req: &mut Request,
    depot: &mut Depot,
    _res: &mut Response,
) -> Result<(), StatusError> {
    let config = get_config(depot)?;
    let header_value = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| StatusError::unauthorized().brief("missing authorization"))?;
    let token = header_value
        .strip_prefix("Bearer ")
        .ok_or_else(|| StatusError::unauthorized().brief("invalid authorization"))?;
    let claims = auth::decode_access_token(token, &config.jwt_secret)
        .map_err(|_| StatusError::unauthorized().brief("invalid token"))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusError::unauthorized().brief("invalid token"))?;
    depot.insert("user_id", user_id);
    Ok(())
}

#[handler]
pub async fn me(depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    let pool = get_pool(depot)?;
    let user_id = get_user_id(depot)?;
    let user: User = with_conn(pool, move |conn| {
        use schema::users::dsl::*;
        users.filter(id.eq(user_id)).first::<User>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("user not found"))?;
    res.render(Json(PublicUser::from(user)));
    Ok(())
}

#[handler]
pub async fn update_me(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let input: UpdateUserProfile = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;
    let pool = get_pool(depot)?;
    let user_id = get_user_id(depot)?;
    let now = Utc::now();

    let updated: User = with_conn(pool, move |conn| {
        use schema::users::dsl::*;
        diesel::update(users.filter(id.eq(user_id)))
            .set((
                name.eq(input.name),
                phone.eq(input.phone),
                updated_at.eq(now),
            ))
            .get_result::<User>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update profile"))?;

    res.render(Json(PublicUser::from(updated)));
    Ok(())
}

#[derive(Deserialize)]
pub struct CreateRecordRequest {
    record_type: String,
    content: serde_json::Value,
}

#[handler]
pub async fn list_records(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let pool = get_pool(depot)?;
    let user_id_value = get_user_id(depot)?;
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let records: Vec<LearningRecord> = with_conn(pool, move |conn| {
        use schema::learning_records::dsl::*;
        learning_records
            .filter(user_id.eq(user_id_value))
            .order(created_at.desc())
            .limit(limit)
            .load::<LearningRecord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list records"))?;

    res.render(Json(records));
    Ok(())
}

#[handler]
pub async fn create_record(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let input: CreateRecordRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;
    if input.record_type.trim().is_empty() {
        return Err(bad_request("record_type is required"));
    }
    let pool = get_pool(depot)?;
    let user_id = get_user_id(depot)?;
    let new_record = NewLearningRecord {
        user_id,
        record_type: input.record_type,
        content: input.content,
    };
    let record: LearningRecord = with_conn(pool, move |conn| {
        use schema::learning_records::dsl::*;
        diesel::insert_into(learning_records)
            .values(&new_record)
            .get_result::<LearningRecord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create record"))?;
    res.status_code(StatusCode::CREATED);
    res.render(Json(record));
    Ok(())
}

#[derive(Deserialize)]
pub struct DesktopCodeRequest {
    redirect_uri: String,
    state: String,
}

#[derive(Serialize)]
pub struct DesktopCodeResponse {
    code: String,
    redirect_uri: String,
    state: String,
    expires_at: DateTime<Utc>,
}

#[handler]
pub async fn create_desktop_code(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let input: DesktopCodeRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;
    if input.redirect_uri.trim().is_empty() {
        return Err(bad_request("redirect_uri is required"));
    }
    if input.state.trim().is_empty() {
        return Err(bad_request("state is required"));
    }

    let pool = get_pool(depot)?;
    let user_id = get_user_id(depot)?;

    let code = auth::random_desktop_code();
    let code_hash = auth::hash_desktop_code(&code);
    let expires_at = Utc::now() + chrono::Duration::minutes(5);
    let record = NewDesktopAuthCode {
        user_id,
        code_hash: code_hash.clone(),
        redirect_uri: input.redirect_uri.clone(),
        state: input.state.clone(),
        expires_at,
    };

    let _saved: DesktopAuthCode = with_conn(pool, move |conn| {
        use schema::desktop_auth_codes::dsl::*;
        diesel::insert_into(desktop_auth_codes)
            .values(&record)
            .get_result::<DesktopAuthCode>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create desktop code"))?;

    res.render(Json(DesktopCodeResponse {
        code,
        redirect_uri: input.redirect_uri,
        state: input.state,
        expires_at,
    }));
    Ok(())
}

#[derive(Deserialize)]
pub struct ConsumeDesktopCodeRequest {
    code: String,
    redirect_uri: String,
}

#[derive(Serialize)]
pub struct ConsumeDesktopCodeResponse {
    access_token: String,
}

#[handler]
pub async fn consume_desktop_code(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let input: ConsumeDesktopCodeRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;
    if input.code.trim().is_empty() || input.redirect_uri.trim().is_empty() {
        return Err(bad_request("code and redirect_uri are required"));
    }

    let pool = get_pool(depot)?;
    let config = get_config(depot)?;
    let code_hash_value = auth::hash_desktop_code(&input.code);
    let redirect_uri_value = input.redirect_uri.clone();
    let now = Utc::now();

    let user_id: Uuid = with_conn(pool, move |conn| {
        use schema::desktop_auth_codes::dsl::*;
        let item: DesktopAuthCode = desktop_auth_codes
            .filter(code_hash.eq(code_hash_value))
            .filter(redirect_uri.eq(redirect_uri_value))
            .filter(used_at.is_null())
            .filter(expires_at.gt(now))
            .first::<DesktopAuthCode>(conn)?;

        diesel::update(desktop_auth_codes.filter(id.eq(item.id)))
            .set(used_at.eq(now))
            .execute(conn)?;
        Ok(item.user_id)
    })
    .await
    .map_err(|_| StatusError::unauthorized().brief("invalid or expired code"))?;

    let access_token =
        auth::issue_access_token(user_id, &config.jwt_secret, config.jwt_ttl.as_secs())
            .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

    res.render(Json(ConsumeDesktopCodeResponse { access_token }));
    Ok(())
}

#[derive(Deserialize)]
pub struct ChatSendRequest {
    message: String,
}

#[derive(Serialize)]
pub struct ChatSendResponse {
    reply: String,
    corrections: Vec<String>,
    suggestions: Vec<String>,
}

fn simple_corrections(message: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = message.to_lowercase();
    if lower.contains("i has ") {
        out.push("Use \"I have …\" instead of \"I has …\".".to_string());
    }
    if lower.contains("he go ") {
        out.push("Use \"he goes …\" for third person singular.".to_string());
    }
    out
}

#[handler]
pub async fn chat_send(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let input: ChatSendRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;
    if input.message.trim().is_empty() {
        return Err(bad_request("message is required"));
    }

    let corrections = simple_corrections(&input.message);
    let suggestions = vec![
        "Try answering with one more detail.".to_string(),
        "Ask me a follow-up question to keep the conversation going.".to_string(),
    ];
    let reply = format!(
        "Let's continue. Tell me more about: {}",
        input.message.trim()
    );

    let pool = get_pool(depot)?;
    let user_id = get_user_id(depot)?;
    let content = json!({
        "user_message": input.message,
        "assistant_reply": reply,
        "corrections": corrections,
        "suggestions": suggestions
    });
    let record = NewLearningRecord {
        user_id,
        record_type: "chat_turn".to_string(),
        content,
    };
    let _ = with_conn(pool, move |conn| {
        use schema::learning_records::dsl::*;
        diesel::insert_into(learning_records)
            .values(&record)
            .execute(conn)?;
        Ok(())
    })
    .await;

    res.render(Json(ChatSendResponse {
        reply,
        corrections,
        suggestions,
    }));
    Ok(())
}

fn get_pool(depot: &Depot) -> Result<&DbPool, StatusError> {
    depot
        .get::<DbPool>("pool")
        .map_err(|_| StatusError::internal_server_error().brief("missing db pool"))
}

fn get_config(depot: &Depot) -> Result<&AppConfig, StatusError> {
    depot
        .get::<AppConfig>("config")
        .map_err(|_| StatusError::internal_server_error().brief("missing app config"))
}

fn get_user_id(depot: &Depot) -> Result<Uuid, StatusError> {
    depot
        .get::<Uuid>("user_id")
        .copied()
        .map_err(|_| StatusError::unauthorized().brief("missing user"))
}

#[derive(Clone)]
struct RequirePermission {
    operation: &'static str,
}

#[salvo::async_trait]
impl Handler for RequirePermission {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        let pool = match get_pool(depot) {
            Ok(p) => p,
            Err(e) => {
                res.render(e);
                ctrl.skip_rest();
                return;
            }
        };
        let user_id = match get_user_id(depot) {
            Ok(v) => v,
            Err(e) => {
                res.render(e);
                ctrl.skip_rest();
                return;
            }
        };
        let operation = self.operation;
        let allowed = with_conn(pool, move |conn| {
            use crate::schema::role_permissions::dsl as rp;
            use crate::schema::user_roles::dsl as ur;
            use diesel::prelude::*;
            let exists = diesel::select(diesel::dsl::exists(
                rp::role_permissions
                    .inner_join(ur::user_roles.on(ur::role_id.eq(rp::role_id)))
                    .filter(ur::user_id.eq(user_id))
                    .filter(rp::operation.eq(operation)),
            ))
            .get_result::<bool>(conn)?;
            Ok(exists)
        })
        .await
        .unwrap_or(false);

        if !allowed {
            res.render(StatusError::forbidden().brief("permission denied"));
            ctrl.skip_rest();
            return;
        }
        ctrl.call_next(req, depot, res).await;
    }
}

fn require_permission(operation: &'static str) -> RequirePermission {
    RequirePermission { operation }
}

fn get_path_uuid(req: &Request, key: &str) -> Result<Uuid, StatusError> {
    let raw = req
        .params()
        .get(key)
        .cloned()
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;
    Uuid::parse_str(&raw).map_err(|_| StatusError::bad_request().brief("invalid id"))
}

#[handler]
async fn admin_delete_user(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let pool = get_pool(depot)?;
    let user_id = get_path_uuid(req, "user_id")?;
    with_conn(pool, move |conn| {
        use crate::schema::users::dsl::*;
        diesel::delete(users.filter(id.eq(user_id))).execute(conn)?;
        Ok(())
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete user"))?;
    res.render(Json(json!({ "ok": true })));
    Ok(())
}

#[derive(Deserialize)]
struct OauthLoginRequest {
    provider: String,
    provider_user_id: String,
    email: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum OauthLoginResponse {
    Ok {
        user: PublicUser,
        access_token: String,
    },
    NeedsBind {
        oauth_identity_id: Uuid,
        provider: String,
        email: Option<String>,
    },
}

#[handler]
async fn oauth_login(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let input: OauthLoginRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;
    if input.provider.trim().is_empty() || input.provider_user_id.trim().is_empty() {
        return Err(bad_request("provider and provider_user_id are required"));
    }

    let pool = get_pool(depot)?;
    let provider = input.provider.trim().to_string();
    let provider_user_id = input.provider_user_id.trim().to_string();
    let email = input.email.clone().map(|e| e.trim().to_lowercase());

    let identity: OauthIdentity = with_conn(pool, move |conn| {
        use crate::schema::oauth_identities::dsl as oi;
        let existing = oi::oauth_identities
            .filter(oi::provider.eq(&provider))
            .filter(oi::provider_user_id.eq(&provider_user_id))
            .first::<OauthIdentity>(conn)
            .optional()?;
        if let Some(i) = existing {
            if email.is_some() && i.email.is_none() {
                return diesel::update(oi::oauth_identities.filter(oi::id.eq(i.id)))
                    .set((oi::email.eq(email), oi::updated_at.eq(Utc::now())))
                    .get_result::<OauthIdentity>(conn);
            }
            return Ok(i);
        }
        diesel::insert_into(oi::oauth_identities)
            .values(&NewOauthIdentity {
                provider,
                provider_user_id,
                email,
                user_id: None,
            })
            .get_result::<OauthIdentity>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to start oauth login"))?;

    if let Some(user_id) = identity.user_id {
        let pool2 = get_pool(depot)?;
        let user: User = with_conn(pool2, move |conn| {
            use crate::schema::users::dsl::*;
            users.filter(id.eq(user_id)).first::<User>(conn)
        })
        .await
        .map_err(|_| StatusError::unauthorized().brief("invalid oauth link"))?;

        let config = get_config(depot)?;
        let access_token =
            auth::issue_access_token(user.id, &config.jwt_secret, config.jwt_ttl.as_secs())
                .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;
        res.render(Json(OauthLoginResponse::Ok {
            user: user.into(),
            access_token,
        }));
        return Ok(());
    }

    res.render(Json(OauthLoginResponse::NeedsBind {
        oauth_identity_id: identity.id,
        provider: identity.provider,
        email: identity.email,
    }));
    Ok(())
}

#[derive(Deserialize)]
struct OauthBindRequest {
    oauth_identity_id: Uuid,
    email: String,
    password: String,
}

#[handler]
async fn oauth_bind(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let input: OauthBindRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;
    let email_input = input.email.trim().to_lowercase();
    if email_input.is_empty() || input.password.is_empty() {
        return Err(bad_request("email and password are required"));
    }
    let pool = get_pool(depot)?;
    let password = input.password.clone();
    let oauth_identity_id = input.oauth_identity_id;

    let (user, _identity): (User, OauthIdentity) = with_conn(pool, move |conn| {
        use crate::schema::oauth_identities::dsl as oi;
        use crate::schema::users::dsl as u;

        let user = u::users
            .filter(u::email.eq(&email_input))
            .first::<User>(conn)?;
        let ok = crate::auth::verify_password(&password, &user.password_hash)
            .map_err(|_| diesel::result::Error::NotFound)?;
        if !ok {
            return Err(diesel::result::Error::NotFound);
        }

        let identity = oi::oauth_identities
            .filter(oi::id.eq(oauth_identity_id))
            .first::<OauthIdentity>(conn)?;

        let identity = diesel::update(oi::oauth_identities.filter(oi::id.eq(identity.id)))
            .set((oi::user_id.eq(Some(user.id)), oi::updated_at.eq(Utc::now())))
            .get_result::<OauthIdentity>(conn)?;

        Ok((user, identity))
    })
    .await
    .map_err(|_| StatusError::unauthorized().brief("invalid credentials or oauth identity"))?;

    let config = get_config(depot)?;
    let access_token =
        auth::issue_access_token(user.id, &config.jwt_secret, config.jwt_ttl.as_secs())
            .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

    res.render(Json(OauthLoginResponse::Ok {
        user: user.into(),
        access_token,
    }));
    Ok(())
}

#[derive(Deserialize)]
struct OauthSkipRequest {
    oauth_identity_id: Uuid,
    name: Option<String>,
    email: Option<String>,
}

#[handler]
async fn oauth_skip(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let input: OauthSkipRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;
    let pool = get_pool(depot)?;
    let oauth_identity_id = input.oauth_identity_id;
    let name = input.name.clone().map(|v| v.trim().to_string());
    let email_override = input.email.clone().map(|v| v.trim().to_lowercase());

    let user: User = with_conn(pool, move |conn| {
        use crate::schema::oauth_identities::dsl as oi;
        use crate::schema::users::dsl as u;

        let identity = oi::oauth_identities
            .filter(oi::id.eq(oauth_identity_id))
            .first::<OauthIdentity>(conn)?;
        if identity.user_id.is_some() {
            return Err(diesel::result::Error::NotFound);
        }
        let email = email_override
            .or(identity.email.clone())
            .unwrap_or_else(|| {
                format!("{}@{}.local", identity.provider_user_id, identity.provider)
            });

        let password_hash = crate::auth::hash_password(&Uuid::new_v4().to_string())
            .map_err(|_| diesel::result::Error::RollbackTransaction)?;

        let user = diesel::insert_into(u::users)
            .values(&NewUser {
                email: email.clone(),
                password_hash,
                name: name.clone().filter(|s| !s.is_empty()),
                phone: None,
            })
            .get_result::<User>(conn)?;

        let _ = diesel::update(oi::oauth_identities.filter(oi::id.eq(identity.id)))
            .set((oi::user_id.eq(Some(user.id)), oi::updated_at.eq(Utc::now())))
            .execute(conn)?;

        Ok(user)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create user"))?;

    let config = get_config(depot)?;
    let access_token =
        auth::issue_access_token(user.id, &config.jwt_secret, config.jwt_ttl.as_secs())
            .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

    res.render(Json(OauthLoginResponse::Ok {
        user: user.into(),
        access_token,
    }));
    Ok(())
}

pub fn router(pool: DbPool, config: AppConfig) -> Router {
    let api = Router::with_path("api")
        .push(Router::with_path("health").get(health))
        .push(account::router())
        .push(
            Router::with_path("chat")
                .hoop(auth_required)
                .push(Router::with_path("send").post(chat_send)),
        );

    let admin = Router::with_path("api/admin")
        .hoop(auth_required)
        .hoop(require_permission("users.delete"))
        .push(Router::with_path("users/{user_id}").delete(admin_delete_user));

    Router::new()
        .hoop(cors)
        .hoop(StateHoop { pool, config })
        .push(api)
        .push(admin)
}
