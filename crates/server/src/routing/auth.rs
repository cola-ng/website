use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::oapi::extract::JsonBody;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::db::schema::*;
use crate::db::with_conn;
use crate::hoops::require_auth;
use crate::models::*;
use crate::{AppResult, DepotExt, JsonResult, json_ok};

pub(super) mod social;

pub fn router() -> Router {
    Router::with_path("auth")
        .push(social::router())
        .push(
            Router::with_path("code")
                .hoop(require_auth)
                .post(create_code),
        )
        .push(Router::with_path("consume").post(consume_code))
}

#[derive(Deserialize, ToSchema)]
pub struct CodeRequest {
    /// OAuth redirect URI
    redirect_uri: String,
    /// OAuth state parameter
    state: String,
}

#[derive(Serialize, ToSchema)]
pub struct CodeResponse {
    /// Authorization code
    code: String,
    /// Original redirect URI
    redirect_uri: String,
    /// Original state parameter
    state: String,
    /// Code expiration time
    expires_at: DateTime<Utc>,
}

/// Create an authorization code for OAuth flow
#[endpoint(tags("Auth"))]
pub async fn create_code(
    input: JsonBody<CodeRequest>,
    depot: &mut Depot,
) -> JsonResult<CodeResponse> {
    if !valid_desktop_callback(&input.redirect_uri) {
        return Err(StatusError::bad_request()
            .brief("redirect_uri must be a loopback HTTP or registered desktop callback")
            .into());
    }
    if input.state.trim().is_empty() {
        return Err(StatusError::bad_request().brief("state is required").into());
    }

    let user_id = depot.user_id()?;

    let code = crate::auth::random_code();
    let code_hash = crate::auth::hash_code(&code);
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
    .map_err(|_| StatusError::internal_server_error().brief("failed to create code"))?;

    json_ok(CodeResponse {
        code,
        redirect_uri: input.redirect_uri.clone(),
        state: input.state.clone(),
        expires_at,
    })
}

#[derive(Deserialize, ToSchema)]
pub struct ConsumeCodeRequest {
    /// Authorization code to consume
    code: String,
    /// Original redirect URI
    redirect_uri: String,
}

#[derive(Serialize, ToSchema)]
pub struct ConsumeCodeResponse {
    /// JWT access token
    access_token: String,
}

/// Exchange authorization code for access token
#[endpoint(tags("Auth"))]
pub async fn consume_code(input: JsonBody<ConsumeCodeRequest>) -> JsonResult<ConsumeCodeResponse> {
    if input.code.trim().is_empty() || input.redirect_uri.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("code and redirect_uri are required")
            .into());
    }

    let config = AppConfig::get();
    let code_hash_value = crate::auth::hash_code(&input.code);
    let redirect_uri_value = input.redirect_uri.clone();
    let now = Utc::now();

    let user_id: i64 = with_conn(move |conn| {
        // Claim and return the code in one statement so concurrent exchanges
        // cannot both consume the same authorization code.
        diesel::update(
            auth_codes::table
                .filter(auth_codes::code_hash.eq(code_hash_value))
                .filter(auth_codes::redirect_uri.eq(redirect_uri_value))
                .filter(auth_codes::used_at.is_null())
                .filter(auth_codes::expires_at.gt(now)),
        )
        .set(auth_codes::used_at.eq(now))
        .returning(auth_codes::user_id)
        .get_result::<i64>(conn)
    })
    .await
    .map_err(|_| StatusError::unauthorized().brief("invalid or expired code"))?;

    let access_token =
        crate::auth::issue_access_token(user_id, &config.jwt_secret, config.jwt_ttl.as_secs())
            .map_err(|_| StatusError::internal_server_error().brief("failed to issue token"))?;

    json_ok(ConsumeCodeResponse { access_token })
}

fn valid_desktop_callback(raw: &str) -> bool {
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    url.username().is_empty()
        && url.password().is_none()
        && ((url.scheme() == "http"
            && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")))
            || (matches!(url.scheme(), "colang" | "amigo") && url.host_str().is_some()))
}

#[cfg(test)]
mod callback_tests {
    use super::valid_desktop_callback;
    #[test]
    fn callback_requires_a_desktop_destination() {
        assert!(valid_desktop_callback("http://127.0.0.1:3456/callback"));
        assert!(valid_desktop_callback("amigo://auth"));
        assert!(!valid_desktop_callback("https://example.com/collect"));
        assert!(!valid_desktop_callback("javascript:alert(1)"));
        assert!(!valid_desktop_callback("http://localhost.evil.test"));
        assert!(!valid_desktop_callback("http://user@localhost"));
    }
}
