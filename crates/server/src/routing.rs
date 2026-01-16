use std::time::Duration;

use salvo::cors::{self, AllowHeaders, Cors};
use salvo::http::Method;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::AppConfig;
use crate::{AppResult, DepotExt};

mod account;
mod asset;
mod auth;
mod learn;

pub fn router(_config: AppConfig) -> Router {
    Router::with_path("api")
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
        .push(Router::with_path("health").get(health))
        .push(account::router())
        .push(auth::router())
        .push(asset::router())
        .push(learn::router())
}

#[handler]
pub async fn health(res: &mut Response) {
    res.render(Json(json!({ "ok": true })));
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
pub async fn chat_send(req: &mut Request, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let input: ChatSendRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input.message.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("message is required")
            .into());
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

    let _user_id = depot.user_id()?;
    let _content = json!({
        "user_message": input.message,
        "assistant_reply": reply,
        "corrections": corrections,
        "suggestions": suggestions
    });

    res.render(Json(ChatSendResponse {
        reply,
        corrections,
        suggestions,
    }));
    Ok(())
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

// #[handler]
// async fn admin_delete_user(
//     req: &mut Request,
//     _depot: &mut Depot,
//     res: &mut Response,
// ) -> AppResult<()> {
//     let user_id = get_path_id(req, "user_id")?;
//     with_conn(move |conn| {
//         diesel::delete(base_users::table.filter(base_users::id.eq(user_id))).execute(conn)?;
//         Ok(())
//     })
//     .await
//     .map_err(|_| StatusError::internal_server_error().brief("failed to delete user"))?;
//     res.render(Json(json!({ "ok": true })));
//     Ok(())
// }

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
// ) -> AppResult<()> {
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
// ) -> AppResult<()> {
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
// ) -> AppResult<()> {
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
