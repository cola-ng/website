use horme::prelude::*;
use salvo::prelude::*;

use crate::AppResult;
use crate::db::schema::user_subscriptions;
use crate::db::with_conn;
use crate::hoops::is_user_admin;

/// Requires an active subscription. Must be used after `maybe_auth` in the handler chain.
/// Admin users are always allowed through.
/// Checks if the authenticated user has an active or trial subscription.
#[handler]
pub async fn require_subscribed(depot: &mut Depot) -> AppResult<()> {
    // First check if user is authenticated
    let user_id = depot
        .get::<i64>("user_id")
        .copied()
        .map_err(|_| StatusError::unauthorized().brief("authentication required"))?;

    // Admin users bypass subscription check
    if is_user_admin(user_id).await {
        depot.insert("is_admin", true);
        return Ok(());
    }

    // Check if user has active subscription
    let has_subscription = has_active_subscription(user_id).await;

    if !has_subscription {
        return Err(StatusError::payment_required()
            .brief("subscription required")
            .into());
    }

    Ok(())
}

/// Helper function to check if a user has an active subscription
pub async fn has_active_subscription(user_id: i64) -> bool {
    with_conn!(move |conn| {
        user_subscriptions::table
            .filter(user_subscriptions::user_id.eq(user_id))
            .filter(
                user_subscriptions::status
                    .eq("active")
                    .or(user_subscriptions::status.eq("trial")),
            )
            .filter(user_subscriptions::expires_at.is_null()
                .or(user_subscriptions::expires_at.gt(chrono::Utc::now())))
            .select(user_subscriptions::id)
            .first::<i64>(conn)
            .await
            .optional()
    })
    .await
    .map(|opt| opt.is_some())
    .unwrap_or(false)
}
