use diesel::prelude::*;
use salvo::http::header;
use salvo::prelude::*;

use crate::{AppConfig, AppResult};

/// Key for storing auth error message in depot
const AUTH_ERROR_KEY: &str = "auth_error";

/// Extracts user_id from token and stores in depot.
/// If auth fails, stores the error message in depot instead of returning error.
/// Always succeeds - downstream handlers should use `require_auth` or check depot manually.
#[handler]
pub async fn maybe_auth(req: &mut Request, depot: &mut Depot, _res: &mut Response) {
    let config = AppConfig::get();

    let result = (|| -> Result<i64, &'static str> {
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

        claims.sub.parse::<i64>().map_err(|_| "invalid token")
    })();

    match result {
        Ok(user_id) => {
            depot.insert("user_id", user_id);
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
    // Check if user_id is present (auth succeeded)
    if let Ok(user_id) = depot.get::<i64>("user_id").copied() {
        use crate::db::schema::base_users;
        let active = crate::db::with_conn(move |conn| {
            base_users::table
                .find(user_id)
                .filter(base_users::disabled_at.is_null())
                .filter(base_users::locked_at.is_null())
                .select(base_users::id)
                .first::<i64>(conn)
                .optional()
        })
        .await
        .map_err(|_| StatusError::service_unavailable().brief("无法验证账户状态，请重试"))?;
        if active.is_none() {
            return Err(StatusError::unauthorized()
                .brief("账户已停用或锁定，请重新登录")
                .into());
        }
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
