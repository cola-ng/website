use std::sync::OnceLock;
use std::time::Duration;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::catcher::Catcher;
use salvo::compression::{Compression, CompressionLevel};
use salvo::conn::rustls::{Keycert, RustlsConfig};
use salvo::conn::tcp::DynTcpAcceptors;
use salvo::cors::{self, AllowHeaders, Cors};
use salvo::http::{Method, header};
use salvo::logging::Logger;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth;
use crate::config::AppConfig;
use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::*;

pub mod account;
pub mod asset;
pub mod learn;

#[handler]
pub async fn health(res: &mut Response) {
    res.render(Json(json!({ "ok": true })));
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    name: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    user: User,
    access_token: String,
}

fn bad_request(message: &str) -> StatusError {
    StatusError::bad_request().brief(message)
}

fn require_json_content_type(req: &Request) -> Result<(), StatusError> {
    let accept = req
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !accept.contains("application/json") {
        return Err(StatusError::unsupported_media_type()
            .brief("Accept header must include application/json"));
    }
    Ok(())
}

const ALLOWED_OAUTH_PROVIDERS: &[&str] = &["google", "github"];

fn validate_oauth_provider(provider: &str) -> Result<(), StatusError> {
    if !ALLOWED_OAUTH_PROVIDERS.contains(&provider.to_lowercase().as_str()) {
        return Err(StatusError::bad_request().brief(format!(
            "OAuth provider '{}' is not supported. Allowed providers: google, github",
            provider
        )));
    }
    Ok(())
}

#[handler]
pub async fn register(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    require_json_content_type(req)?;
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

    let new_user = NewUser {
        name: input.name.clone(),
        email: Some(email.clone()),
        phone: None,
    };

    let user: User = with_conn(move |conn| {
        let is_first = users::table
            .select(diesel::dsl::count_star())
            .first::<i64>(conn)?
            == 0;

        let user = diesel::insert_into(users::table)
            .values(&new_user)
            .get_result::<User>(conn)?;

        // if is_first {
        //     let role = diesel::insert_into(roles::table)
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

        //     let _ = diesel::insert_into(role_users)
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
        hash: password_hash,
        created_at: Utc::now(),
    };
    let config = AppConfig::get();
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
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    require_json_content_type(req)?;
    let input: LoginRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;
    let email_input = input.email.trim().to_lowercase();
    if email_input.is_empty() {
        return Err(bad_request("email is required"));
    }

    let user: User = with_conn(move |conn| {
        users::table
            .filter(users::email.eq(email_input))
            .first::<User>(conn)
    })
    .await
    .map_err(|_| StatusError::unauthorized().brief("invalid credentials"))?;

    let ok = auth::verify_password(user.id, &input.password)
        .await
        .map_err(|_| StatusError::unauthorized().brief("invalid credentials"))?;
    if !ok {
        return Err(StatusError::unauthorized().brief("invalid credentials"));
    }

    let config = AppConfig::get();
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
    let config = AppConfig::get();
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
    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| StatusError::unauthorized().brief("invalid token"))?;
    depot.insert("user_id", user_id);
    Ok(())
}


#[derive(Deserialize)]
pub struct CodeRequest {
    redirect_uri: String,
    state: String,
}

#[derive(Serialize)]
pub struct CodeResponse {
    code: String,
    redirect_uri: String,
    state: String,
    expires_at: DateTime<Utc>,
}

#[handler]
pub async fn create_code(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    require_json_content_type(req)?;
    let input: CodeRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;
    if input.redirect_uri.trim().is_empty() {
        return Err(bad_request("redirect_uri is required"));
    }
    if input.state.trim().is_empty() {
        return Err(bad_request("state is required"));
    }

    let user_id = get_user_id(depot)?;

    let code = auth::random_code();
    let code_hash = auth::hash_desktop_code(&code);
    let expires_at = Utc::now() + chrono::Duration::minutes(5);
    let record = NewAuthCode {
        user_id,
        code_hash: code_hash.clone(),
        redirect_uri: input.redirect_uri.clone(),
        state: input.state.clone(),
        expires_at,
    };

    let _saved: AuthCode = with_conn(move |conn| {
        diesel::insert_into(auth_codes::table)
            .values(&record)
            .get_result::<AuthCode>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create desktop code"))?;

    res.render(Json(CodeResponse {
        code,
        redirect_uri: input.redirect_uri,
        state: input.state,
        expires_at,
    }));
    Ok(())
}

#[derive(Deserialize)]
pub struct ConsumeCodeRequest {
    code: String,
    redirect_uri: String,
}

#[derive(Serialize)]
pub struct ConsumeCodeResponse {
    access_token: String,
}

#[handler]
pub async fn consume_code(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    require_json_content_type(req)?;
    let input: ConsumeCodeRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;
    if input.code.trim().is_empty() || input.redirect_uri.trim().is_empty() {
        return Err(bad_request("code and redirect_uri are required"));
    }

    let config = AppConfig::get();
    let code_hash_value = auth::hash_desktop_code(&input.code);
    let redirect_uri_value = input.redirect_uri.clone();
    let now = Utc::now();

    let user_id: i64 = with_conn(move |conn| {
        let item: AuthCode = auth_codes::table
            .filter(auth_codes::code_hash.eq(code_hash_value))
            .filter(auth_codes::redirect_uri.eq(redirect_uri_value))
            .filter(auth_codes::used_at.is_null())
            .filter(auth_codes::expires_at.gt(now))
            .first::<AuthCode>(conn)?;

        diesel::update(auth_codes::table.filter(auth_codes::id.eq(item.id)))
            .set(auth_codes::used_at.eq(now))
            .execute(conn)?;
        Ok(item.user_id)
    })
    .await
    .map_err(|_| StatusError::unauthorized().brief("invalid or expired code"))?;

    let access_token =
        auth::issue_access_token(user_id, &config.jwt_secret, config.jwt_ttl.as_secs())
            .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

    res.render(Json(ConsumeCodeResponse { access_token }));
    Ok(())
}

fn get_user_id(depot: &Depot) -> Result<i64, StatusError> {
    depot
        .get::<i64>("user_id")
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
        let user_id = match get_user_id(depot) {
            Ok(v) => v,
            Err(e) => {
                res.render(e);
                ctrl.skip_rest();
                return;
            }
        };
        let operation = self.operation;
        let allowed = with_conn(move |conn| {
            use diesel::prelude::*;

            use crate::db::schema::role_permissions::dsl as rp;
            use crate::db::schema::role_users::dsl as ur;
            let exists = diesel::select(diesel::dsl::exists(
                rp::role_permissions
                    .inner_join(ur::role_users.on(ur::role_id.eq(rp::role_id)))
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

fn get_path_id(req: &Request, key: &str) -> Result<i64, StatusError> {
    let raw = req
        .params()
        .get(key)
        .cloned()
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;
    raw.parse()
        .map_err(|_| StatusError::bad_request().brief("invalid id"))
}

#[handler]
async fn admin_delete_user(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let user_id = get_path_id(req, "user_id")?;
    with_conn(move |conn| {
        diesel::delete(users::table.filter(users::id.eq(user_id))).execute(conn)?;
        Ok(())
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete user"))?;
    res.render(Json(json!({ "ok": true })));
    Ok(())
}

// #[derive(Deserialize)]
// struct OauthLoginRequest {
//     provider: String,
//     provider_user_id: String,
//     email: Option<String>,
// }

// #[derive(Serialize)]
// #[serde(tag = "status", rename_all = "snake_case")]
// enum OauthLoginResponse {
//     Ok {
//         user: User,
//         access_token: String,
//     },
//     NeedsBind {
//         oauth_identity_id: i64,
//         provider: String,
//         email: Option<String>,
//     },
// }

// #[handler]
// async fn oauth_login(
//     req: &mut Request,
//     _depot: &mut Depot,
//     res: &mut Response,
// ) -> Result<(), StatusError> {
//     require_json_content_type(req)?;
//     let input: OauthLoginRequest = req
//         .parse_json()
//         .await
//         .map_err(|_| bad_request("invalid json"))?;
//     if input.provider.trim().is_empty() || input.provider_user_id.trim().is_empty() {
//         return Err(bad_request("provider and provider_user_id are required"));
//     }
//     validate_oauth_provider(&input.provider)?;

//     let provider = input.provider.trim().to_string();
//     let provider_user_id = input.provider_user_id.trim().to_string();
//     let email = input.email.clone().map(|e| e.trim().to_lowercase());

//     let identity: OauthIdentity = with_conn(move |conn| {
//         use crate::db::schema::oauth_identities::dsl as oi;
//         let existing = oi::oauth_identities
//             .filter(oi::provider.eq(&provider))
//             .filter(oi::provider_user_id.eq(&provider_user_id))
//             .first::<OauthIdentity>(conn)
//             .optional()?;
//         if let Some(i) = existing {
//             if email.is_some() && i.email.is_none() {
//                 return diesel::update(oi::oauth_identities.filter(oi::id.eq(i.id)))
//                     .set((oi::email.eq(email), oi::updated_at.eq(Utc::now())))
//                     .get_result::<OauthIdentity>(conn);
//             }
//             return Ok(i);
//         }
//         diesel::insert_into(oi::oauth_identities)
//             .values(&NewOauthIdentity {
//                 provider,
//                 provider_user_id,
//                 email,
//                 user_id: None,
//             })
//             .get_result::<OauthIdentity>(conn)
//     })
//     .await
//     .map_err(|_| StatusError::internal_server_error().brief("failed to start oauth login"))?;

//     if let Some(user_id) = identity.user_id {
//         let user: User =
//             with_conn(move |conn| users::table.filter(id.eq(user_id)).first::<User>(conn))
//                 .await
//                 .map_err(|_| StatusError::unauthorized().brief("invalid oauth link"))?;

//         let config = AppConfig::get();
//         let access_token =
//             auth::issue_access_token(user.id, &config.jwt_secret, config.jwt_ttl.as_secs())
//                 .map_err(|_| StatusError::internal_server_error().brief("failed to issue
// token"))?;         res.render(Json(OauthLoginResponse::Ok {
//             user: user.into(),
//             access_token,
//         }));
//         return Ok(());
//     }

//     res.render(Json(OauthLoginResponse::NeedsBind {
//         oauth_identity_id: identity.id,
//         provider: identity.provider,
//         email: identity.email,
//     }));
//     Ok(())
// }

// #[derive(Deserialize)]
// struct OauthBindRequest {
//     oauth_identity_id: i64,
//     email: String,
//     password: String,
// }

// #[handler]
// async fn oauth_bind(
//     req: &mut Request,
//     _depot: &mut Depot,
//     res: &mut Response,
// ) -> Result<(), StatusError> {
//     require_json_content_type(req)?;
//     let input: OauthBindRequest = req
//         .parse_json()
//         .await
//         .map_err(|_| bad_request("invalid json"))?;
//     let email_input = input.email.trim().to_lowercase();
//     if email_input.is_empty() || input.password.is_empty() {
//         return Err(bad_request("email and password are required"));
//     }
//     let password = input.password.clone();
//     let oauth_identity_id = input.oauth_identity_id;

//     let (user, _identity): (User, OauthIdentity) = with_conn(move |conn| {
//         use crate::db::schema::oauth_identities::dsl as oi;
//         use crate::db::schema::users::dsl as u;

//         let user = u::users
//             .filter(u::email.eq(&email_input))
//             .first::<User>(conn)?;
//         let ok = crate::auth::verify_password(&password, &user.password_hash)
//             .map_err(|_| diesel::result::Error::NotFound)?;
//         if !ok {
//             return Err(diesel::result::Error::NotFound);
//         }

//         let identity = oi::oauth_identities
//             .filter(oi::id.eq(oauth_identity_id))
//             .first::<OauthIdentity>(conn)?;

//         let identity = diesel::update(oi::oauth_identities.filter(oi::id.eq(identity.id)))
//             .set((oi::user_id.eq(Some(user.id)), oi::updated_at.eq(Utc::now())))
//             .get_result::<OauthIdentity>(conn)?;

//         Ok((user, identity))
//     })
//     .await
//     .map_err(|_| StatusError::unauthorized().brief("invalid credentials or oauth identity"))?;

//     let config = AppConfig::get();
//     let access_token =
//         auth::issue_access_token(user.id, &config.jwt_secret, config.jwt_ttl.as_secs())
//             .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

//     res.render(Json(OauthLoginResponse::Ok {
//         user: user.into(),
//         access_token,
//     }));
//     Ok(())
// }

// #[derive(Deserialize)]
// struct OauthSkipRequest {
//     oauth_identity_id: i64,
//     name: Option<String>,
//     email: Option<String>,
// }

// #[handler]
// async fn oauth_skip(
//     req: &mut Request,
//     _depot: &mut Depot,
//     res: &mut Response,
// ) -> Result<(), StatusError> {
//     require_json_content_type(req)?;
//     let input: OauthSkipRequest = req
//         .parse_json()
//         .await
//         .map_err(|_| bad_request("invalid json"))?;
//     let oauth_identity_id = input.oauth_identity_id;
//     let name = input.name.clone().map(|v| v.trim().to_string());
//     let email_override = input.email.clone().map(|v| v.trim().to_lowercase());

//     let user: User = with_conn(move |conn| {
//         use crate::db::schema::oauth_identities::dsl as oi;
//         use crate::db::schema::users::dsl as u;

//         let identity = oi::oauth_identities
//             .filter(oi::id.eq(oauth_identity_id))
//             .first::<OauthIdentity>(conn)?;
//         if identity.user_id.is_some() {
//             return Err(diesel::result::Error::NotFound);
//         }
//         let email = email_override
//             .or(identity.email.clone())
//             .unwrap_or_else(|| {
//                 format!("{}@{}.local", identity.provider_user_id, identity.provider)
//             });

//         let password_hash = crate::auth::hash_password(&crate::auth::random_code())
//             .map_err(|_| diesel::result::Error::RollbackTransaction)?;

//         let user = diesel::insert_into(u::users)
//             .values(&NewUser {
//                 email: email.clone(),
//                 password_hash,
//                 name: name.clone().filter(|s| !s.is_empty()),
//                 phone: None,
//             })
//             .get_result::<User>(conn)?;

//         let _ = diesel::update(oi::oauth_identities.filter(oi::id.eq(identity.id)))
//             .set((oi::user_id.eq(Some(user.id)), oi::updated_at.eq(Utc::now())))
//             .execute(conn)?;

//         Ok(user)
//     })
//     .await
//     .map_err(|_| StatusError::internal_server_error().brief("failed to create user"))?;

//     let config = AppConfig::get();
//     let access_token =
//         auth::issue_access_token(user.id, &config.jwt_secret, config.jwt_ttl.as_secs())
//             .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

//     res.render(Json(OauthLoginResponse::Ok {
//         user: user.into(),
//         access_token,
//     }));
//     Ok(())
// }

pub fn router(config: AppConfig) -> Router {
    let api = Router::with_path("api")
        .push(Router::with_path("health").get(health))
        .push(account::router())
        .push(learn::router(auth_required))
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
        .hoop(
            Cors::new()
                .allow_origin(cors::Any)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers(AllowHeaders::list([
                    salvo::http::header::ACCEPT,
                    salvo::http::header::CONTENT_TYPE,
                    salvo::http::header::AUTHORIZATION,
                    salvo::http::header::RANGE,
                ]))
                .max_age(Duration::from_secs(86400))
                .into_handler(),
        )
        .push(api)
        .push(admin)
}
