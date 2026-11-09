use diesel::prelude::*;
use salvo::prelude::*;

use crate::AppResult;
use crate::db::schema::{base_admins, base_users};
use crate::db::with_conn;

/// Requires admin privileges. Must be used after `maybe_auth` in the handler chain.
/// Checks if the authenticated user is in the base_admins table.
#[handler]
pub async fn require_admin(depot: &mut Depot) -> AppResult<()> {
    // First check if user is authenticated
    let user_id = depot
        .get::<i64>("user_id")
        .copied()
        .map_err(|_| StatusError::unauthorized().brief("authentication required"))?;

    // Check if user is admin
    let is_admin = is_user_admin(user_id).await;

    if !is_admin {
        return Err(StatusError::forbidden()
            .brief("admin access required")
            .into());
    }

    // Store admin status in depot for downstream handlers
    depot.insert("is_admin", true);
    Ok(())
}

/// Helper function to check if a user is admin (for use in handlers)
pub async fn is_user_admin(user_id: i64) -> bool {
    with_conn(move |conn| {
        base_admins::table
            .filter(base_admins::user_id.eq(user_id))
            .filter(
                base_admins::user_id.eq_any(
                    base_users::table
                        .filter(base_users::id.eq(user_id))
                        .filter(base_users::disabled_at.is_null())
                        .filter(base_users::locked_at.is_null())
                        .select(base_users::id),
                ),
            )
            .select(base_admins::id)
            .first::<i64>(conn)
            .optional()
    })
    .await
    .map(|opt| opt.is_some())
    .unwrap_or(false)
}
