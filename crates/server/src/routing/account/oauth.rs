use chrono::{DateTime, Utc};
use diesel::prelude::*;

use salvo::oapi::extract::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::*;
use crate::db::schema::*;
use crate::db::with_conn;
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

    let identities: Vec<OauthIdentity> = with_conn(move |conn| {
        oauth_identities::table
            .filter(oauth_identities::user_id.eq(user_id))
            .order(oauth_identities::created_at.desc())
            .load::<OauthIdentity>(conn)
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
        })
        .collect();

    json_ok(result)
}

/// Unlink an OAuth identity from the current user
#[endpoint(tags("Account"))]
pub async fn unlink_identity(id: PathParam<i64>, depot: &mut Depot) -> JsonResult<serde_json::Value> {
    let user_id = depot.user_id()?;
    let identity_id = id.into_inner();

    // Check if user has a password or other linked identities
    let (has_password, identity_count) = with_conn(move |conn| {
        let has_password = base_passwords::table
            .filter(base_passwords::user_id.eq(user_id))
            .select(base_passwords::id)
            .first::<i64>(conn)
            .optional()?
            .is_some();

        let identity_count: i64 = oauth_identities::table
            .filter(oauth_identities::user_id.eq(user_id))
            .count()
            .get_result(conn)?;

        Ok((has_password, identity_count))
    })
    .await
    .map_err(|_: String| StatusError::internal_server_error().brief("failed to check account status"))?;

    // Prevent unlinking if it's the only way to login
    if !has_password && identity_count <= 1 {
        return Err(StatusError::bad_request()
            .brief("cannot unlink the only login method")
            .into());
    }

    // Delete the identity
    let deleted = with_conn(move |conn| {
        diesel::delete(
            oauth_identities::table
                .filter(oauth_identities::id.eq(identity_id))
                .filter(oauth_identities::user_id.eq(user_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to unlink identity"))?;

    if deleted == 0 {
        return Err(StatusError::not_found().brief("identity not found").into());
    }

    json_ok(serde_json::json!({ "deleted": true }))
}

/// Get account status including whether user has password and linked identities
#[endpoint(tags("Account"))]
pub async fn get_account_status(depot: &mut Depot) -> JsonResult<AccountStatusResponse> {
    let user_id = depot.user_id()?;

    let (has_password, identities) = with_conn(move |conn| {
        let has_password = base_passwords::table
            .filter(base_passwords::user_id.eq(user_id))
            .select(base_passwords::id)
            .first::<i64>(conn)
            .optional()?
            .is_some();

        let identities: Vec<OauthIdentity> = oauth_identities::table
            .filter(oauth_identities::user_id.eq(user_id))
            .order(oauth_identities::created_at.desc())
            .load::<OauthIdentity>(conn)?;

        Ok((has_password, identities))
    })
    .await
    .map_err(|_: String| StatusError::internal_server_error().brief("failed to fetch account status"))?;

    let linked_identities: Vec<LinkedOauthIdentity> = identities
        .into_iter()
        .map(|i| LinkedOauthIdentity {
            id: i.id,
            provider: i.provider,
            email: i.email,
            linked_at: i.created_at,
        })
        .collect();

    json_ok(AccountStatusResponse {
        has_password,
        linked_identities,
    })
}

/// Request to link a new OAuth identity
#[derive(Deserialize, ToSchema)]
pub struct LinkIdentityRequest {
    /// OAuth provider (e.g., "google", "github")
    pub provider: String,
    /// User ID from the OAuth provider
    pub provider_user_id: String,
    /// Email from the OAuth provider (optional)
    pub email: Option<String>,
}

/// Link a new OAuth identity to the current user
#[endpoint(tags("Account"))]
pub async fn link_identity(
    input: JsonBody<LinkIdentityRequest>,
    depot: &mut Depot,
) -> JsonResult<LinkedOauthIdentity> {
    let user_id = depot.user_id()?;
    let provider = input.provider.trim().to_lowercase();
    let provider_user_id = input.provider_user_id.trim().to_string();
    let email = input.email.clone().map(|e| e.trim().to_lowercase());

    // Validate provider
    if provider != "google" && provider != "github" {
        return Err(StatusError::bad_request()
            .brief("unsupported oauth provider")
            .into());
    }

    if provider_user_id.is_empty() {
        return Err(StatusError::bad_request()
            .brief("provider_user_id is required")
            .into());
    }

    // Check if this OAuth identity already exists
    let existing = with_conn({
        let provider = provider.clone();
        let provider_user_id = provider_user_id.clone();
        move |conn| {
            oauth_identities::table
                .filter(oauth_identities::provider.eq(&provider))
                .filter(oauth_identities::provider_user_id.eq(&provider_user_id))
                .first::<OauthIdentity>(conn)
                .optional()
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to check existing identity"))?;

    if let Some(existing) = existing {
        if existing.user_id == Some(user_id) {
            // Already linked to this user
            return json_ok(LinkedOauthIdentity {
                id: existing.id,
                provider: existing.provider,
                email: existing.email,
                linked_at: existing.created_at,
            });
        } else if existing.user_id.is_some() {
            // Linked to another user
            return Err(StatusError::conflict()
                .brief("this oauth identity is already linked to another account")
                .into());
        }
        // Not linked to any user, update to link to current user
        let updated = with_conn(move |conn| {
            diesel::update(oauth_identities::table.filter(oauth_identities::id.eq(existing.id)))
                .set((
                    oauth_identities::user_id.eq(Some(user_id)),
                    oauth_identities::updated_at.eq(Utc::now()),
                ))
                .get_result::<OauthIdentity>(conn)
        })
        .await
        .map_err(|_| StatusError::internal_server_error().brief("failed to link identity"))?;

        return json_ok(LinkedOauthIdentity {
            id: updated.id,
            provider: updated.provider,
            email: updated.email,
            linked_at: updated.created_at,
        });
    }

    // Check if user already has this provider linked
    let already_linked = with_conn({
        let provider = provider.clone();
        move |conn| {
            oauth_identities::table
                .filter(oauth_identities::user_id.eq(user_id))
                .filter(oauth_identities::provider.eq(&provider))
                .first::<OauthIdentity>(conn)
                .optional()
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to check linked identities"))?;

    if already_linked.is_some() {
        return Err(StatusError::conflict()
            .brief("you already have this provider linked")
            .into());
    }

    // Create new OAuth identity
    let new_identity = with_conn(move |conn| {
        diesel::insert_into(oauth_identities::table)
            .values(&NewOauthIdentity {
                provider,
                provider_user_id,
                email,
                user_id: Some(user_id),
            })
            .get_result::<OauthIdentity>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create identity"))?;

    json_ok(LinkedOauthIdentity {
        id: new_identity.id,
        provider: new_identity.provider,
        email: new_identity.email,
        linked_at: new_identity.created_at,
    })
}
