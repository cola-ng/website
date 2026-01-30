use diesel::dsl::count;
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::oapi::extract::JsonBody;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::hoops::{require_admin, require_auth};
use crate::models::*;
use crate::{DepotExt, JsonResult, json_ok};

pub fn router() -> Router {
    Router::with_path("admin")
        .hoop(require_auth)
        .hoop(require_admin)
        .push(Router::with_path("stats").get(dashboard_stats))
        .push(
            Router::with_path("users")
                .get(list_users)
                .push(Router::with_path("{user_id}").get(get_user)),
        )
        .push(
            Router::with_path("admins")
                .get(list_admins)
                .push(Router::with_path("grant").post(grant_admin))
                .push(Router::with_path("revoke").post(revoke_admin)),
        )
}

#[derive(Serialize, ToSchema)]
pub struct DashboardStats {
    /// Total number of users
    total_users: i64,
    /// Total number of admins
    total_admins: i64,
    /// Total number of words in dictionary
    total_words: i64,
    /// Users registered today
    users_today: i64,
}

/// Get admin dashboard statistics
#[endpoint(tags("Admin"))]
pub async fn dashboard_stats() -> JsonResult<DashboardStats> {
    let stats = with_conn(move |conn| {
        let total_users: i64 = base_users::table
            .select(count(base_users::id))
            .first(conn)?;

        let total_admins: i64 = base_admins::table
            .select(count(base_admins::id))
            .first(conn)?;

        let total_words: i64 = dict_words::table
            .select(count(dict_words::id))
            .first(conn)?;

        let today = chrono::Utc::now().date_naive();
        let users_today: i64 = base_users::table
            .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(
                &format!("DATE(created_at) = '{}'", today),
            ))
            .select(count(base_users::id))
            .first(conn)?;

        Ok::<_, diesel::result::Error>(DashboardStats {
            total_users,
            total_admins,
            total_words,
            users_today,
        })
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get stats"))?;

    json_ok(stats)
}

#[derive(Serialize, ToSchema)]
pub struct UserListItem {
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct UserListResponse {
    users: Vec<UserListItem>,
    total: i64,
    page: i64,
    per_page: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct ListUsersQuery {
    /// Page number (1-indexed)
    page: Option<i64>,
    /// Items per page (default 20)
    per_page: Option<i64>,
    /// Search by name or email
    search: Option<String>,
}

/// List all users with pagination
#[endpoint(tags("Admin"))]
pub async fn list_users(req: &mut Request) -> JsonResult<UserListResponse> {
    let query: ListUsersQuery = req.parse_queries().unwrap_or(ListUsersQuery {
        page: None,
        per_page: None,
        search: None,
    });

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let search = query.search.clone();

    let (users, total) = with_conn(move |conn| {
        // Count query
        let total: i64 = if let Some(ref search_term) = search {
            let pattern = format!("%{}%", search_term.to_lowercase());
            base_users::table
                .filter(
                    base_users::name
                        .ilike(&pattern)
                        .or(base_users::email.ilike(&pattern)),
                )
                .select(count(base_users::id))
                .first(conn)?
        } else {
            base_users::table
                .select(count(base_users::id))
                .first(conn)?
        };

        // Data query
        let users: Vec<User> = if let Some(ref search_term) = search {
            let pattern = format!("%{}%", search_term.to_lowercase());
            base_users::table
                .filter(
                    base_users::name
                        .ilike(&pattern)
                        .or(base_users::email.ilike(&pattern)),
                )
                .order(base_users::created_at.desc())
                .offset(offset)
                .limit(per_page)
                .load(conn)?
        } else {
            base_users::table
                .order(base_users::created_at.desc())
                .offset(offset)
                .limit(per_page)
                .load(conn)?
        };

        // Get admin status for these users
        let user_ids: Vec<i64> = users.iter().map(|u| u.id).collect();
        let admin_user_ids: Vec<i64> = base_admins::table
            .filter(base_admins::user_id.eq_any(&user_ids))
            .select(base_admins::user_id)
            .load(conn)?;

        let user_items: Vec<UserListItem> = users
            .into_iter()
            .map(|u| UserListItem {
                id: u.id,
                name: u.name,
                email: u.email,
                phone: u.phone,
                is_admin: admin_user_ids.contains(&u.id),
                created_at: u.created_at,
            })
            .collect();

        Ok::<_, diesel::result::Error>((user_items, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list users"))?;

    json_ok(UserListResponse {
        users,
        total,
        page,
        per_page,
    })
}

#[derive(Serialize, ToSchema)]
pub struct UserDetailResponse {
    #[serde(flatten)]
    user: User,
    is_admin: bool,
    admin_info: Option<AdminInfo>,
    subscription: Option<SubscriptionInfo>,
    wallet: Option<WalletAccount>,
    recent_payments: Vec<PaymentOrder>,
}

#[derive(Serialize, ToSchema)]
pub struct AdminInfo {
    granted_at: chrono::DateTime<chrono::Utc>,
    granted_by: Option<i64>,
    notes: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct SubscriptionInfo {
    #[serde(flatten)]
    subscription: UserSubscription,
    plan: SubscriptionPlan,
}

/// Get user details by ID
#[endpoint(tags("Admin"))]
pub async fn get_user(req: &mut Request) -> JsonResult<UserDetailResponse> {
    let user_id: i64 = req
        .param::<i64>("user_id")
        .ok_or_else(|| StatusError::bad_request().brief("user_id is required"))?;

    let result = with_conn(move |conn| {
        let user: User = base_users::table
            .filter(base_users::id.eq(user_id))
            .first(conn)?;

        let admin: Option<Admin> = base_admins::table
            .filter(base_admins::user_id.eq(user_id))
            .first(conn)
            .optional()?;

        // Get current subscription
        let subscription: Option<UserSubscription> = user_subscriptions::table
            .filter(user_subscriptions::user_id.eq(user_id))
            .filter(
                user_subscriptions::status
                    .eq("active")
                    .or(user_subscriptions::status.eq("trial")),
            )
            .order(user_subscriptions::created_at.desc())
            .first(conn)
            .optional()?;

        let subscription_info = if let Some(ref sub) = subscription {
            let plan: SubscriptionPlan = subscription_plans::table
                .filter(subscription_plans::id.eq(sub.plan_id))
                .first(conn)?;
            Some(SubscriptionInfo {
                subscription: sub.clone(),
                plan,
            })
        } else {
            None
        };

        // Get wallet
        let wallet: Option<WalletAccount> = wallet_accounts::table
            .filter(wallet_accounts::user_id.eq(user_id))
            .first(conn)
            .optional()?;

        // Get recent payments
        let recent_payments: Vec<PaymentOrder> = payment_orders::table
            .filter(payment_orders::user_id.eq(user_id))
            .order(payment_orders::created_at.desc())
            .limit(10)
            .load(conn)?;

        Ok::<_, diesel::result::Error>((user, admin, subscription_info, wallet, recent_payments))
    })
    .await
    .map_err(|_| StatusError::not_found().brief("user not found"))?;

    let (user, admin, subscription, wallet, recent_payments) = result;

    let admin_info = admin.map(|a| AdminInfo {
        granted_at: a.granted_at,
        granted_by: a.granted_by,
        notes: a.notes,
    });

    json_ok(UserDetailResponse {
        user,
        is_admin: admin_info.is_some(),
        admin_info,
        subscription,
        wallet,
        recent_payments,
    })
}

#[derive(Serialize, ToSchema)]
pub struct AdminListItem {
    pub id: i64,
    pub user_id: i64,
    pub user_name: String,
    pub user_email: Option<String>,
    pub granted_at: chrono::DateTime<chrono::Utc>,
    pub granted_by: Option<i64>,
    pub notes: Option<String>,
}

/// List all admins
#[endpoint(tags("Admin"))]
pub async fn list_admins() -> JsonResult<Vec<AdminListItem>> {
    let admins = with_conn(move |conn| {
        let admins: Vec<Admin> = base_admins::table
            .order(base_admins::granted_at.desc())
            .load(conn)?;

        let user_ids: Vec<i64> = admins.iter().map(|a| a.user_id).collect();
        let users: Vec<User> = base_users::table
            .filter(base_users::id.eq_any(&user_ids))
            .load(conn)?;

        let admin_items: Vec<AdminListItem> = admins
            .into_iter()
            .map(|a| {
                let user = users.iter().find(|u| u.id == a.user_id);
                AdminListItem {
                    id: a.id,
                    user_id: a.user_id,
                    user_name: user.map(|u| u.name.clone()).unwrap_or_default(),
                    user_email: user.and_then(|u| u.email.clone()),
                    granted_at: a.granted_at,
                    granted_by: a.granted_by,
                    notes: a.notes,
                }
            })
            .collect();

        Ok::<_, diesel::result::Error>(admin_items)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list admins"))?;

    json_ok(admins)
}

#[derive(Deserialize, ToSchema)]
pub struct GrantAdminRequest {
    /// User ID to grant admin privileges
    user_id: i64,
    /// Optional notes about why admin was granted
    notes: Option<String>,
}

/// Grant admin privileges to a user
#[endpoint(tags("Admin"))]
pub async fn grant_admin(
    input: JsonBody<GrantAdminRequest>,
    depot: &mut Depot,
) -> JsonResult<Admin> {
    let current_user_id = depot.user_id()?;
    let target_user_id = input.user_id;
    let notes = input.notes.clone();

    let admin = with_conn(move |conn| {
        // Check if user exists
        let _user: User = base_users::table
            .filter(base_users::id.eq(target_user_id))
            .first(conn)
            .map_err(|_| {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::NotNullViolation,
                    Box::new("user not found".to_string()),
                )
            })?;

        // Check if already admin
        let existing: Option<Admin> = base_admins::table
            .filter(base_admins::user_id.eq(target_user_id))
            .first(conn)
            .optional()?;

        if let Some(admin) = existing {
            return Ok(admin);
        }

        // Grant admin
        let new_admin = NewAdmin {
            user_id: target_user_id,
            granted_by: Some(current_user_id),
            notes: notes.as_deref(),
        };

        diesel::insert_into(base_admins::table)
            .values(&new_admin)
            .get_result::<Admin>(conn)
    })
    .await
    .map_err(|e| {
        if e.contains("user not found") {
            StatusError::not_found().brief("user not found")
        } else {
            StatusError::internal_server_error().brief("failed to grant admin")
        }
    })?;

    json_ok(admin)
}

#[derive(Deserialize, ToSchema)]
pub struct RevokeAdminRequest {
    /// User ID to revoke admin privileges from
    user_id: i64,
}

/// Revoke admin privileges from a user
#[endpoint(tags("Admin"))]
pub async fn revoke_admin(
    input: JsonBody<RevokeAdminRequest>,
    depot: &mut Depot,
) -> JsonResult<()> {
    let current_user_id = depot.user_id()?;
    let target_user_id = input.user_id;

    // Prevent self-revoke
    if current_user_id == target_user_id {
        return Err(StatusError::bad_request()
            .brief("cannot revoke your own admin privileges")
            .into());
    }

    with_conn(move |conn| {
        diesel::delete(base_admins::table.filter(base_admins::user_id.eq(target_user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to revoke admin"))?;

    json_ok(())
}
