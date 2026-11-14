use horme::prelude::*;
use salvo::http::header;
use salvo::prelude::*;
use std::result::Result;

use crate::{AppConfig, AppResult};

/// Key for storing auth error message in depot
const AUTH_ERROR_KEY: &str = "auth_error";

/// Extracts user_id from token and stores in depot.
/// If auth fails, stores the error message in depot instead of returning error.
/// Always succeeds - downstream handlers should use `require_auth` or check depot manually.
#[handler]
pub async fn maybe_auth(req: &mut Request, depot: &mut Depot, _res: &mut Response) {
    let _ = depot.remove::<i64>("user_id");
    let _ = depot.remove::<i64>("token_version");
    let _ = depot.remove::<String>(AUTH_ERROR_KEY);
    let _ = depot.remove::<bool>("auth_unavailable");
    let config = AppConfig::get();

    let result = (|| -> Result<(i64, i64), &'static str> {
        let header_value = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or("missing authorization")?;

        let token = header_value
            .strip_prefix("Bearer ")
            .ok_or("invalid authorization")?;

        let claims = crate::auth::decode_access_token(token, &config.jwt_secret)
            .map_err(|_| "invalid token")?;

        Ok((
            claims.sub.parse::<i64>().map_err(|_| "invalid token")?,
            claims.version,
        ))
    })();

    match result {
        Ok((user_id, version)) => {
            use crate::db::schema::base_users;
            let active = crate::db::with_conn!(move |conn| {
                base_users::table
                    .find(user_id)
                    .filter(base_users::disabled_at.is_null())
                    .filter(base_users::locked_at.is_null())
                    .filter(base_users::token_version.eq(version))
                    .select(base_users::id)
                    .first::<i64>(conn)
                    .await
                    .optional()
            })
            .await;
            match active {
                Ok(Some(_)) => {
                    depot.insert("user_id", user_id);
                    depot.insert("token_version", version);
                }
                Ok(None) => {
                    depot.insert(AUTH_ERROR_KEY, "登录已失效，请重新登录".to_string());
                }
                Err(_) => {
                    depot.insert("auth_unavailable", true);
                }
            }
        }
        Err(error_msg) => {
            depot.insert(AUTH_ERROR_KEY, error_msg.to_string());
        }
    }
}

/// Requires authentication. Must be used after `maybe_auth` in the handler chain.
/// Checks depot for user_id or auth_error and returns appropriate response.
#[handler]
pub async fn require_auth(depot: &mut Depot, _res: &mut Response) -> AppResult<()> {
    if depot
        .get::<bool>("auth_unavailable")
        .copied()
        .unwrap_or(false)
    {
        return Err(StatusError::service_unavailable()
            .brief("无法验证账户状态，请重试")
            .into());
    }
    if depot.get::<i64>("user_id").is_ok() {
        return Ok(());
    }

    // Check for stored auth error message
    if let Ok(error_msg) = depot.get::<String>(AUTH_ERROR_KEY) {
        return Err(StatusError::unauthorized().brief(error_msg.as_str()).into());
    }

    // No auth was attempted (maybe_auth wasn't called)
    Err(StatusError::unauthorized()
        .brief("authentication required")
        .into())
}
