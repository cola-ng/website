use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::http::header;
use salvo::http::StatusCode;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::auth;
use crate::db::with_conn;
use crate::models::{
    DesktopAuthCode, LearningRecord, NewDesktopAuthCode, NewLearningRecord, NewUser,
    UpdateUserProfile, User,
};
use crate::schema;
use crate::AppState;

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

    let state = get_state(depot)?;
    let pool = state.pool.clone();
    let new_user = NewUser {
        email: email.clone(),
        password_hash,
        name: input.name.clone(),
        phone: None,
    };

    let user: User = with_conn(pool, move |conn| {
        use schema::users::dsl::*;
        diesel::insert_into(users)
            .values(&new_user)
            .get_result::<User>(conn)
    })
    .await
    .map_err(|e| {
        if e.contains("UniqueViolation") || e.contains("unique") {
            StatusError::conflict().brief("email already registered")
        } else {
            StatusError::internal_server_error().brief("failed to create user")
        }
    })?;

    let access_token = auth::issue_access_token(
        user.id,
        &state.config.jwt_secret,
        state.config.jwt_ttl.as_secs(),
    )
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

    let state = get_state(depot)?;
    let pool = state.pool.clone();
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

    let access_token = auth::issue_access_token(
        user.id,
        &state.config.jwt_secret,
        state.config.jwt_ttl.as_secs(),
    )
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
    let state = get_state(depot)?;
    let header_value = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| StatusError::unauthorized().brief("missing authorization"))?;
    let token = header_value
        .strip_prefix("Bearer ")
        .ok_or_else(|| StatusError::unauthorized().brief("invalid authorization"))?;
    let claims = auth::decode_access_token(token, &state.config.jwt_secret)
        .map_err(|_| StatusError::unauthorized().brief("invalid token"))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusError::unauthorized().brief("invalid token"))?;
    depot.insert("user_id", user_id);
    Ok(())
}

#[handler]
pub async fn me(depot: &mut Depot, res: &mut Response) -> Result<(), StatusError> {
    let state = get_state(depot)?;
    let pool = state.pool.clone();
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
    let state = get_state(depot)?;
    let pool = state.pool.clone();
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
    let state = get_state(depot)?;
    let pool = state.pool.clone();
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
    let state = get_state(depot)?;
    let pool = state.pool.clone();
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

    let state_ref = get_state(depot)?;
    let pool = state_ref.pool.clone();
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

    let state = get_state(depot)?;
    let pool = state.pool.clone();
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

    let access_token = auth::issue_access_token(
        user_id,
        &state.config.jwt_secret,
        state.config.jwt_ttl.as_secs(),
    )
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

    let state = get_state(depot)?;
    let pool = state.pool.clone();
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

fn get_state(depot: &Depot) -> Result<std::sync::Arc<AppState>, StatusError> {
    depot
        .get::<std::sync::Arc<AppState>>("state")
        .cloned()
        .map_err(|_| StatusError::internal_server_error().brief("missing app state"))
}

fn get_user_id(depot: &Depot) -> Result<Uuid, StatusError> {
    depot
        .get::<Uuid>("user_id")
        .copied()
        .map_err(|_| StatusError::unauthorized().brief("missing user"))
}

pub fn router(state: std::sync::Arc<AppState>) -> Router {
    let api = Router::with_path("api")
        .push(Router::with_path("health").get(health))
        .push(
            Router::with_path("auth")
                .push(Router::with_path("register").post(register))
                .push(Router::with_path("login").post(login)),
        )
        .push(
            Router::with_path("me")
                .hoop(auth_required)
                .get(me)
                .put(update_me)
                .push(
                    Router::with_path("records")
                        .get(list_records)
                        .post(create_record),
                ),
        )
        .push(
            Router::with_path("desktop/auth")
                .push(
                    Router::with_path("code")
                        .hoop(auth_required)
                        .post(create_desktop_code),
                )
                .push(Router::with_path("consume").post(consume_desktop_code)),
        )
        .push(
            Router::with_path("chat")
                .hoop(auth_required)
                .push(Router::with_path("send").post(chat_send)),
        );

    Router::new()
        .hoop(crate::hoops::StateHoop { state })
        .hoop(cors)
        .push(api)
}