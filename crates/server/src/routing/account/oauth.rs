use chrono::{DateTime, Utc};
use horme::prelude::*;
use salvo::oapi::ToSchema;
use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::Serialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::*;
use crate::{DepotExt, JsonResult, json_ok};

// ============================================================================
// OAuth Identity Management (Social Login)
// ============================================================================

/// Linked OAuth identity response
#[derive(Serialize, ToSchema)]
pub struct LinkedOauthIdentity {
    pub id: i64,
    pub provider: String,
    pub email: Option<String>,
    pub linked_at: DateTime<Utc>,
    pub verified: bool,
}

/// Account status response
#[derive(Serialize, ToSchema)]
pub struct AccountStatusResponse {
    /// Whether the user has a password set (native account)
    pub has_password: bool,
    /// List of linked OAuth identities
    pub linked_identities: Vec<LinkedOauthIdentity>,
}

/// List all linked OAuth identities for the current user
#[endpoint(tags("Account"))]
pub async fn list_identities(depot: &mut Depot) -> JsonResult<Vec<LinkedOauthIdentity>> {
    let user_id = depot.user_id()?;

    let identities: Vec<OauthIdentity> = with_conn!(move |conn| {
        oauth_identities::table
            .filter(oauth_identities::user_id.eq(user_id))
            .order(oauth_identities::created_at.desc())
            .load::<OauthIdentity>(conn).await
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch identities"))?;

    let result: Vec<LinkedOauthIdentity> = identities
        .into_iter()
        .map(|i| LinkedOauthIdentity {
            id: i.id,
            provider: i.provider,
            email: i.email,
            linked_at: i.created_at,
            verified: i.verified_at.is_some(),
        })
        .collect();

    json_ok(result)
}

/// Unlink an OAuth identity from the current user
#[endpoint(tags("Account"))]
pub async fn unlink_identity(
    id: PathParam<i64>,
    depot: &mut Depot,
) -> JsonResult<serde_json::Value> {
    let user_id = depot.user_id()?;
    let identity_id = id.into_inner();

    let result = with_conn!(move |conn| {
        horme::transaction(conn, async |conn| {
            diesel::sql_query("LOCK TABLE oauth_identities IN SHARE ROW EXCLUSIVE MODE")
                .execute(conn).await?;
            let identity = oauth_identities::table
                .find(identity_id)
                .filter(oauth_identities::user_id.eq(user_id))
                .first::<OauthIdentity>(conn).await
                .optional()?;
            if identity.is_none() {
                return Ok(0);
            }
            let has_password = base_passwords::table
                .filter(base_passwords::user_id.eq(user_id))
                .count()
                .get_result::<i64>(conn).await?
                > 0;
            let count = oauth_identities::table
                .filter(oauth_identities::user_id.eq(user_id))
                .filter(oauth_identities::verified_at.is_not_null())
                .count()
                .get_result::<i64>(conn).await?;
            if !has_password
                && count <= 1
                && identity.as_ref().is_some_and(|i| i.verified_at.is_some())
            {
                return Ok(-1);
            }
            diesel::delete(oauth_identities::table.find(identity_id)).execute(conn).await?;
            Ok(1)
        }).await
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("无法解除绑定，请重试"))?;
    if result == -1 {
        return Err(StatusError::bad_request()
            .brief("至少保留一种登录方式，请先设置密码或绑定其他账户")
            .into());
    }
    if result == 0 {
        return Err(StatusError::not_found().brief("identity not found").into());
    }

    json_ok(serde_json::json!({ "deleted": true }))
}

/// Get account status including whether user has password and linked identities
#[endpoint(tags("Account"))]
pub async fn get_account_status(depot: &mut Depot) -> JsonResult<AccountStatusResponse> {
    let user_id = depot.user_id()?;

    let (has_password, identities) = with_conn!(move |conn| {
        let has_password = base_passwords::table
            .filter(base_passwords::user_id.eq(user_id))
            .select(base_passwords::id)
            .first::<i64>(conn).await
            .optional()?
            .is_some();

        let identities: Vec<OauthIdentity> = oauth_identities::table
            .filter(oauth_identities::user_id.eq(user_id))
            .order(oauth_identities::created_at.desc())
            .load::<OauthIdentity>(conn).await?;

        Ok((has_password, identities))
    })
    .await
    .map_err(|_: String| {
        StatusError::internal_server_error().brief("failed to fetch account status")
    })?;

    let linked_identities: Vec<LinkedOauthIdentity> = identities
        .into_iter()
        .map(|i| LinkedOauthIdentity {
            id: i.id,
            provider: i.provider,
            email: i.email,
            linked_at: i.created_at,
            verified: i.verified_at.is_some(),
        })
        .collect();

    json_ok(AccountStatusResponse {
        has_password,
        linked_identities,
    })
}

/// Direct subject claims cannot prove ownership. Use the verified provider flow.
#[handler]
pub async fn link_identity() -> crate::AppResult<()> {
    Err(StatusError::bad_request()
        .brief("请通过第三方授权绑定账户")
        .into())
}
