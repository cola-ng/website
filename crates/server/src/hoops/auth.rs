
use salvo::http::header;
use salvo::prelude::*;

use crate::{AppConfig, AppResult};

#[handler]
pub async fn require_auth(
    req: &mut Request,
    depot: &mut Depot,
    _res: &mut Response,
) -> AppResult<()> {
    let config = AppConfig::get();
    let header_value = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| StatusError::unauthorized().brief("missing authorization"))?;
    let token = header_value
        .strip_prefix("Bearer ")
        .ok_or_else(|| StatusError::unauthorized().brief("invalid authorization"))?;
    let claims = crate::auth::decode_access_token(token, &config.jwt_secret)
        .map_err(|_| StatusError::unauthorized().brief("invalid token"))?;
    let user_id = claims
        .sub
        .parse::<i64>()
        .map_err(|_| StatusError::unauthorized().brief("invalid token"))?;
    depot.insert("user_id", user_id);
    Ok(())
}
