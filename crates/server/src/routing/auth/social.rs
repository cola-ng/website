//! Server-side social sign-in. Provider tokens and client secrets never reach the browser.
use std::result::Result;
use base64::Engine;
use chrono::Utc;
use horme::prelude::*;
use salvo::http::cookie::{Cookie, SameSite};
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::auth::{hash_code, random_code};
use crate::db::{schema::*, with_conn};
use crate::models::*;
use crate::{AppConfig, AppResult, DepotExt};

const COOKIE: &str = "colang_oauth";
const COMPLETE: &str = "complete";

pub fn router() -> Router {
    Router::new()
        .push(Router::with_path("providers").get(providers))
        .push(Router::with_path("complete").post(complete))
        .push(Router::with_path("{provider}/start").get(start))
        .push(Router::with_path("{provider}/callback").get(callback))
}

struct Settings {
    base: url::Url,
    id: String,
    secret: String,
}

impl Settings {
    fn load(provider: &str) -> Result<Self, &'static str> {
        let prefix = match provider {
            "github" => "GITHUB",
            "google" => "GOOGLE",
            _ => return Err("unsupported_provider"),
        };
        let raw = std::env::var("PUBLIC_BASE_URL").unwrap_or_default();
        let mut base = url::Url::parse(&raw).map_err(|_| "not_configured")?;
        if !valid_origin(&base) {
            return Err("not_configured");
        }
        base.set_path("/");
        let id = std::env::var(format!("{prefix}_CLIENT_ID")).unwrap_or_default();
        let secret = std::env::var(format!("{prefix}_CLIENT_SECRET")).unwrap_or_default();
        if id.trim().is_empty() || secret.trim().is_empty() {
            return Err("not_configured");
        }
        Ok(Self { base, id, secret })
    }
    fn callback(&self, provider: &str) -> String {
        self.base
            .join(&format!("/api/auth/{provider}/callback"))
            .unwrap()
            .into()
    }
}

fn valid_origin(url: &url::Url) -> bool {
    url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && matches!(url.path(), "" | "/")
        && (url.scheme() == "https"
            || (url.scheme() == "http"
                && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))))
}

fn safe_next(raw: &str) -> String {
    if raw.starts_with('/')
        && !raw.starts_with("//")
        && !raw.contains(['\\', '\r', '\n'])
        && let Ok(url) = url::Url::parse("https://colang.invalid").unwrap().join(raw)
        && url.host_str() == Some("colang.invalid")
        && !url.path().starts_with("/api/")
        && url.path() != "/auth/complete"
    {
        return format!(
            "{}{}{}",
            url.path(),
            url.query().map(|q| format!("?{q}")).unwrap_or_default(),
            url.fragment().map(|f| format!("#{f}")).unwrap_or_default()
        );
    }
    "/me".into()
}

fn no_store(res: &mut Response) {
    res.add_header("Cache-Control", "no-store", true).unwrap();
    res.add_header("Referrer-Policy", "no-referrer", true)
        .unwrap();
}

fn set_cookie(res: &mut Response, value: String, secure: bool, age: i64) {
    res.add_cookie(
        Cookie::build((COOKIE, value))
            .path("/api/auth")
            .http_only(true)
            .secure(secure)
            .same_site(SameSite::Lax)
            .max_age(salvo::http::cookie::time::Duration::seconds(age))
            .build(),
    );
}

#[derive(Serialize, Deserialize)]
struct Session {
    browser_hash: String,
    verifier: String,
    next: String,
    link_user: Option<i64>,
    complete_user: Option<i64>,
}

async fn store_session(
    provider: &str,
    state: &str,
    session: Session,
    seconds: i64,
) -> AppResult<()> {
    let record = NewOauthLoginSession {
        provider: provider.into(),
        state: hash_code(state),
        redirect_uri: serde_json::to_string(&session)?,
        expires_at: Utc::now() + chrono::Duration::seconds(seconds),
    };
    with_conn!(move |conn| {
        diesel::delete(
            oauth_login_sessions::table.filter(oauth_login_sessions::expires_at.lt(Utc::now())),
        )
        .execute(conn).await?;
        diesel::insert_into(oauth_login_sessions::table)
            .values(record)
            .execute(conn).await
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("无法创建登录会话，请重试"))?;
    Ok(())
}

async fn claim_session(
    req: &Request,
    provider: &str,
    state: &str,
) -> Result<Session, &'static str> {
    if state.len() != 43 {
        return Err("invalid_state");
    }
    let browser = req
        .cookie(COOKIE)
        .map(|c| c.value())
        .ok_or("invalid_state")?;
    let hashed = hash_code(state);
    let provider = provider.to_string();
    let record = with_conn!(move |conn| {
        oauth_login_sessions::table
            .filter(oauth_login_sessions::state.eq(hashed))
            .filter(oauth_login_sessions::provider.eq(provider))
            .filter(oauth_login_sessions::expires_at.gt(Utc::now()))
            .first::<OauthLoginSession>(conn).await
    })
    .await
    .map_err(|_| "invalid_state")?;
    let session: Session =
        serde_json::from_str(&record.redirect_uri).map_err(|_| "invalid_state")?;
    if session.browser_hash != hash_code(browser) {
        return Err("invalid_state");
    }
    let count = with_conn!(move |conn| {
        diesel::delete(oauth_login_sessions::table.filter(oauth_login_sessions::id.eq(record.id)))
            .execute(conn).await
    })
    .await
    .map_err(|_| "invalid_state")?;
    if count != 1 {
        return Err("invalid_state");
    }
    Ok(session)
}

#[handler]
async fn providers(res: &mut Response) {
    no_store(res);
    let available: Vec<_> = [("github", "GitHub"), ("google", "Google")]
        .into_iter()
        .filter(|(id, _)| Settings::load(id).is_ok())
        .map(|(id, name)| json!({"id":id,"name":name}))
        .collect();
    res.render(Json(json!({"providers":available})));
}

fn authorization_url(provider: &str, settings: &Settings, state: &str, verifier: &str) -> url::Url {
    let endpoint = if provider == "github" {
        "https://github.com/login/oauth/authorize"
    } else {
        "https://accounts.google.com/o/oauth2/v2/auth"
    };
    let mut url = url::Url::parse(endpoint).unwrap();
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));
    url.query_pairs_mut()
        .append_pair("client_id", &settings.id)
        .append_pair("redirect_uri", &settings.callback(provider))
        .append_pair("state", state)
        .append_pair("response_type", "code")
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair(
            "scope",
            if provider == "github" {
                "read:user user:email"
            } else {
                "openid email profile"
            },
        );
    if provider == "google" {
        url.query_pairs_mut()
            .append_pair("prompt", "select_account");
    }
    url
}

async fn begin(req: &Request, res: &mut Response, link_user: Option<i64>) -> AppResult<String> {
    let provider = req.param::<String>("provider").unwrap_or_default();
    let settings = Settings::load(&provider)
        .map_err(|_| StatusError::service_unavailable().brief("该登录方式尚未配置"))?;
    let state = random_code();
    let verifier = random_code();
    let browser = random_code();
    let next = if link_user.is_some() {
        "/me".into()
    } else {
        safe_next(&req.query::<String>("next").unwrap_or_default())
    };
    let url = authorization_url(&provider, &settings, &state, &verifier);
    store_session(
        &provider,
        &state,
        Session {
            browser_hash: hash_code(&browser),
            verifier,
            next,
            link_user,
            complete_user: None,
        },
        600,
    )
    .await?;
    set_cookie(res, browser, settings.base.scheme() == "https", 600);
    no_store(res);
    Ok(url.into())
}

#[handler]
async fn start(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let url = begin(req, res, None).await?;
    res.render(Redirect::found(url));
    Ok(())
}

#[handler]
pub async fn link_start(req: &mut Request, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user = depot.user_id()?;
    let url = begin(req, res, Some(user)).await?;
    res.render(Json(json!({"authorization_url":url})));
    Ok(())
}

struct Identity {
    subject: String,
    email: Option<String>,
    name: String,
}

fn client() -> Result<reqwest::Client, &'static str> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Colang-OAuth/1.0")
        .build()
        .map_err(|_| "provider_unavailable")
}

async fn provider_identity(
    provider: &str,
    settings: &Settings,
    code: &str,
    verifier: &str,
) -> Result<Identity, &'static str> {
    let client = client()?;
    let endpoint = if provider == "github" {
        "https://github.com/login/oauth/access_token"
    } else {
        "https://oauth2.googleapis.com/token"
    };
    let token: Value = client
        .post(endpoint)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", settings.id.as_str()),
            ("client_secret", settings.secret.as_str()),
            ("code", code),
            ("redirect_uri", settings.callback(provider).as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(|_| "provider_unavailable")?
        .error_for_status()
        .map_err(|_| "exchange_failed")?
        .json()
        .await
        .map_err(|_| "exchange_failed")?;
    let access_token = token["access_token"]
        .as_str()
        .filter(|v| !v.is_empty())
        .ok_or("exchange_failed")?;
    let endpoint = if provider == "github" {
        "https://api.github.com/user"
    } else {
        "https://openidconnect.googleapis.com/v1/userinfo"
    };
    let user: Value = client
        .get(endpoint)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_| "provider_unavailable")?
        .error_for_status()
        .map_err(|_| "exchange_failed")?
        .json()
        .await
        .map_err(|_| "exchange_failed")?;
    let (subject, email) = if provider == "github" {
        let subject = user["id"]
            .as_u64()
            .filter(|v| *v > 0)
            .ok_or("invalid_identity")?
            .to_string();
        let emails: Vec<Value> = client
            .get("https://api.github.com/user/emails")
            .bearer_auth(access_token)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|_| "provider_unavailable")?
            .error_for_status()
            .map_err(|_| "exchange_failed")?
            .json()
            .await
            .map_err(|_| "invalid_identity")?;
        let email = emails
            .iter()
            .filter(|e| e["verified"] == true)
            .max_by_key(|e| e["primary"] == true)
            .and_then(|e| e["email"].as_str())
            .map(str::to_lowercase);
        (subject, email)
    } else {
        let subject = user["sub"]
            .as_str()
            .filter(|v| !v.is_empty())
            .ok_or("invalid_identity")?
            .to_string();
        let email = if user["email_verified"] == true {
            user["email"].as_str().map(str::to_lowercase)
        } else {
            None
        };
        (subject, email)
    };
    let name = user["name"]
        .as_str()
        .or_else(|| user["login"].as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Colang 学员")
        .chars()
        .take(80)
        .collect();
    Ok(Identity {
        subject,
        email,
        name,
    })
}

async fn resolve_user(
    provider: String,
    identity: Identity,
    link_user: Option<i64>,
) -> Result<i64, &'static str> {
    // Serialize identity/account resolution so simultaneous first logins cannot create orphan users.
    let result = with_conn!(move |conn| {
        horme::transaction(conn, async |conn| {
            diesel::sql_query("LOCK TABLE oauth_identities IN SHARE ROW EXCLUSIVE MODE")
                .execute(conn).await?;
            let existing = oauth_identities::table
                .filter(oauth_identities::provider.eq(&provider))
                .filter(oauth_identities::provider_user_id.eq(&identity.subject))
                .first::<OauthIdentity>(conn).await
                .optional()?;
            if let Some(id) = existing
                .as_ref()
                .filter(|i| i.verified_at.is_some())
                .and_then(|i| i.user_id)
            {
                if link_user.is_some_and(|user| user != id) {
                    return Ok(Err("already_linked"));
                }
                let user = base_users::table.find(id).first::<User>(conn).await?;
                if user.disabled_at.is_some() || user.locked_at.is_some() {
                    return Ok(Err("account_disabled"));
                }
                return Ok(Ok(id));
            }
            let user_id = if let Some(id) = link_user {
                let user = base_users::table.find(id).first::<User>(conn).await?;
                if user.disabled_at.is_some() || user.locked_at.is_some() {
                    return Ok(Err("account_disabled"));
                }
                if oauth_identities::table
                    .filter(oauth_identities::provider.eq(&provider))
                    .filter(oauth_identities::user_id.eq(id))
                    .filter(oauth_identities::verified_at.is_not_null())
                    .count()
                    .get_result::<i64>(conn).await?
                    > 0
                {
                    return Ok(Err("provider_already_linked"));
                }
                id
            } else {
                let Some(email) = identity.email.as_ref().filter(|e| e.contains('@')) else {
                    return Ok(Err("verified_email_required"));
                };
                // Never merge an existing password account by email alone. It must explicitly link while signed in.
                if base_users::table
                    .filter(base_users::email.eq(email))
                    .count()
                    .get_result::<i64>(conn).await?
                    > 0
                {
                    return Ok(Err("account_exists"));
                }
                let new = NewUser {
                    name: format!("{}-{}", identity.name, &random_code()[..8]),
                    email: Some(email.clone()),
                    phone: None,
                    display_name: Some(identity.name.clone()),
                    inviter_id: None,
                    updated_by: None,
                    created_by: None,
                };
                let user = diesel::insert_into(base_users::table)
                    .values(new)
                    .get_result::<User>(conn).await?;
                diesel::update(base_users::table.find(user.id))
                    .set(base_users::verified_at.eq(Some(Utc::now())))
                    .execute(conn).await?;
                user.id
            };
            if let Some(existing) = existing {
                diesel::update(oauth_identities::table.find(existing.id))
                    .set((
                        oauth_identities::user_id.eq(Some(user_id)),
                        oauth_identities::email.eq(identity.email),
                        oauth_identities::updated_at.eq(Utc::now()),
                        oauth_identities::verified_at.eq(Some(Utc::now())),
                    ))
                    .execute(conn).await?;
            } else {
                diesel::insert_into(oauth_identities::table)
                    .values(NewOauthIdentity {
                        provider,
                        provider_user_id: identity.subject,
                        email: identity.email,
                        user_id: Some(user_id),
                        verified_at: Some(Utc::now()),
                    })
                    .execute(conn).await?;
            }
            Ok(Ok(user_id))
        }).await
    })
    .await;
    result.map_err(|_| "account_conflict")?
}

#[handler]
async fn callback(req: &mut Request, res: &mut Response) {
    no_store(res);
    let provider = req.param::<String>("provider").unwrap_or_default();
    let result = callback_result(req, &provider).await;
    let secure = Settings::load(&provider).is_ok_and(|s| s.base.scheme() == "https");
    match result {
        Ok(code) => completion_redirect(res, &code),
        Err(reason) => {
            set_cookie(res, String::new(), secure, 0);
            res.render(Redirect::found(format!("/login?oauth_error={reason}")));
        }
    }
}

fn completion_redirect(res: &mut Response, code: &str) {
    // http::Uri removes fragments. Keep the browser-only result code in the
    // Location header directly, without round-tripping it through http::Uri.
    res.status_code(StatusCode::FOUND);
    res.add_header("Location", format!("/auth/complete#code={code}"), true)
        .expect("a generated URL-safe code is a valid header value");
}

async fn callback_result(req: &Request, provider: &str) -> Result<String, &'static str> {
    let settings = Settings::load(provider)?;
    let state = req.query::<String>("state").unwrap_or_default();
    let mut session = claim_session(req, provider, &state).await?;
    if req.query::<String>("error").is_some() {
        return Err("access_denied");
    }
    let code = req
        .query::<String>("code")
        .filter(|c| !c.is_empty() && c.len() <= 4096)
        .ok_or("exchange_failed")?;
    let identity = provider_identity(provider, &settings, &code, &session.verifier).await?;
    let user = resolve_user(provider.into(), identity, session.link_user).await?;
    session.complete_user = Some(user);
    session.verifier.clear();
    let code = random_code();
    store_session(COMPLETE, &code, session, 60)
        .await
        .map_err(|_| "provider_unavailable")?;
    Ok(code)
}

#[derive(Deserialize)]
struct CompleteRequest {
    code: String,
}

#[handler]
async fn complete(req: &mut Request, res: &mut Response) -> AppResult<()> {
    no_store(res);
    let input = req
        .parse_json::<CompleteRequest>()
        .await
        .map_err(|_| StatusError::bad_request().brief("登录结果无效，请重试"))?;
    let session = claim_session(req, COMPLETE, &input.code)
        .await
        .map_err(|_| StatusError::unauthorized().brief("登录结果已过期，请重新登录"))?;
    let user_id = session
        .complete_user
        .ok_or_else(|| StatusError::unauthorized().brief("登录结果无效"))?;
    let user = with_conn!(move |conn| base_users::table.find(user_id).first::<User>(conn).await)
        .await
        .map_err(|_| StatusError::unauthorized().brief("账户不存在"))?;
    if user.disabled_at.is_some() || user.locked_at.is_some() {
        return Err(StatusError::forbidden().brief("账户已停用或锁定").into());
    }
    let config = AppConfig::get();
    let token =
        crate::auth::issue_access_token(user.id, user.token_version, &config.jwt_secret, config.jwt_ttl.as_secs())
            .map_err(|_| StatusError::internal_server_error().brief("无法完成登录，请重试"))?;
    let is_admin = crate::hoops::is_user_admin(user.id).await;
    let secure = std::env::var("PUBLIC_BASE_URL").is_ok_and(|v| v.starts_with("https://"));
    set_cookie(res, String::new(), secure, 0);
    res.render(Json(
        json!({"user":user,"is_admin":is_admin,"access_token":token,"next":session.next}),
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_redirect_preserves_browser_fragment() {
        let code = random_code();
        let mut response = Response::new();
        completion_redirect(&mut response, &code);
        assert_eq!(response.status_code, Some(StatusCode::FOUND));
        assert_eq!(
            response.headers()["Location"],
            format!("/auth/complete#code={code}")
        );
    }
    #[test]
    fn return_paths_stay_on_site() {
        for value in [
            "https://evil.test",
            "//evil.test",
            "/\\evil.test",
            "/%2f%2fevil.test",
            "/api/auth/complete",
            "/auth/complete",
            "/\r\nevil",
        ] {
            let result = safe_next(value);
            assert!(!result.starts_with("//"));
            assert!(!result.contains('\\'));
            assert_eq!(
                url::Url::parse("https://colang.invalid")
                    .unwrap()
                    .join(&result)
                    .unwrap()
                    .host_str(),
                Some("colang.invalid")
            );
        }
        assert_eq!(
            safe_next("/auth?redirect_uri=amigo%3A%2F%2Fauth&state=abc"),
            "/auth?redirect_uri=amigo%3A%2F%2Fauth&state=abc"
        );
    }
    #[test]
    fn only_explicit_secure_or_loopback_origins() {
        for raw in [
            "https://colang.example",
            "http://127.0.0.1:5173",
            "http://localhost:8119",
        ] {
            assert!(valid_origin(&url::Url::parse(raw).unwrap()));
        }
        for raw in [
            "http://colang.example",
            "https://user@colang.example",
            "https://colang.example/path",
            "https://colang.example?x=1",
        ] {
            assert!(!valid_origin(&url::Url::parse(raw).unwrap()));
        }
    }
    #[test]
    fn both_providers_use_pkce_and_minimal_scopes() {
        let settings = Settings {
            base: url::Url::parse("https://colang.example").unwrap(),
            id: "client".into(),
            secret: "secret".into(),
        };
        for provider in ["github", "google"] {
            let url = authorization_url(provider, &settings, "state", "verifier");
            let params: std::collections::HashMap<_, _> = url.query_pairs().collect();
            assert_eq!(params["code_challenge_method"], "S256");
            assert_eq!(params["state"], "state");
            assert!(!url.as_str().contains("secret"));
            assert!(!params["scope"].contains("repo"));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "requires an explicitly isolated local colang_qa PostgreSQL database"]
    async fn oauth_database_flow() {
        let database = std::env::var("QA_DATABASE_URL").expect("QA_DATABASE_URL required");
        let url = url::Url::parse(&database).unwrap();
        assert!(
            matches!(url.host_str(), Some("localhost" | "127.0.0.1")) && url.path() == "/colang_qa"
        );
        let config = AppConfig {
            bind_addr: "127.0.0.1:0".into(),
            jwt_secret: random_code(),
            jwt_ttl: std::time::Duration::from_secs(600),
            database: crate::config::DbConfig {
                url: database,
                ..Default::default()
            },
            space_path: String::new(),
        };
        crate::config::APP_CONFIG.set(config.clone()).unwrap();
        crate::db::init(&config.database).await;
        let suffix = random_code();
        let subject = format!("google-{suffix}");
        let email = format!("oauth-{suffix}@example.test");
        let identity = || Identity {
            subject: subject.clone(),
            email: Some(email.clone()),
            name: "OAuth QA".into(),
        };
        let first = resolve_user("google".into(), identity(), None)
            .await
            .unwrap();
        assert_eq!(
            resolve_user("google".into(), identity(), None)
                .await
                .unwrap(),
            first
        );
        assert_eq!(
            resolve_user(
                "github".into(),
                Identity {
                    subject: format!("github-{suffix}"),
                    email: Some(email.clone()),
                    name: "QA".into()
                },
                None
            )
            .await,
            Err("account_exists")
        );
        assert_eq!(
            resolve_user(
                "github".into(),
                Identity {
                    subject: format!("github-{suffix}"),
                    email: Some(email.clone()),
                    name: "QA".into()
                },
                Some(first)
            )
            .await
            .unwrap(),
            first
        );
        assert_eq!(
            resolve_user(
                "github".into(),
                Identity {
                    subject: format!("other-{suffix}"),
                    email: None,
                    name: "QA".into()
                },
                Some(first)
            )
            .await,
            Err("provider_already_linked")
        );
        assert_eq!(
            resolve_user(
                "google".into(),
                Identity {
                    subject: format!("unverified-{suffix}"),
                    email: None,
                    name: "QA".into()
                },
                None
            )
            .await,
            Err("verified_email_required")
        );
        let other = resolve_user(
            "google".into(),
            Identity {
                subject: format!("second-{suffix}"),
                email: Some(format!("other-{suffix}@example.test")),
                name: "QA".into(),
            },
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            resolve_user("google".into(), identity(), Some(other)).await,
            Err("already_linked")
        );
        let legacy_subject = format!("legacy-{suffix}");
        let legacy_insert = legacy_subject.clone();
        with_conn!(move |conn| {
            diesel::insert_into(oauth_identities::table)
                .values(NewOauthIdentity {
                    provider: "github".into(),
                    provider_user_id: legacy_insert,
                    email: None,
                    user_id: Some(other),
                    verified_at: None,
                })
                .execute(conn).await
        })
        .await
        .unwrap();
        let real_owner = resolve_user(
            "github".into(),
            Identity {
                subject: legacy_subject,
                email: Some(format!("legacy-owner-{suffix}@example.test")),
                name: "Actual owner".into(),
            },
            None,
        )
        .await
        .unwrap();
        assert_ne!(
            real_owner, other,
            "An unverified legacy binding must never select the login account"
        );
        with_conn!(move |conn| {
            diesel::update(base_users::table.find(first))
                .set(base_users::disabled_at.eq(Some(Utc::now())))
                .execute(conn).await
        })
        .await
        .unwrap();
        assert_eq!(
            resolve_user("google".into(), identity(), None).await,
            Err("account_disabled")
        );
    }
}
