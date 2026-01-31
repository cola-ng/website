use chrono::Datelike;
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
        .push(Router::with_path("stats/users/daily").get(user_stats_daily))
        .push(Router::with_path("stats/ai/today").get(ai_usage_today))
        .push(Router::with_path("stats/ai/top-users").get(ai_top_users))
        .push(Router::with_path("stats/finance").get(finance_stats))
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
        .push(Router::with_path("orders").get(list_orders))
        .push(Router::with_path("subscriptions").get(list_subscriptions))
        // Chat scenario management
        .push(
            Router::with_path("stages")
                .get(list_stages)
                .post(create_stage)
                .push(
                    Router::with_path("{stage_id}")
                        .get(get_stage)
                        .put(update_stage)
                        .delete(delete_stage),
                ),
        )
        // Read-along management
        .push(
            Router::with_path("read-subjects")
                .get(list_read_subjects)
                .post(create_read_subject)
                .push(
                    Router::with_path("{subject_id}")
                        .get(get_read_subject)
                        .put(update_read_subject)
                        .delete(delete_read_subject),
                ),
        )
        .push(
            Router::with_path("read-sentences")
                .get(list_read_sentences)
                .post(create_read_sentence)
                .push(
                    Router::with_path("{sentence_id}")
                        .get(get_read_sentence)
                        .put(update_read_sentence)
                        .delete(delete_read_sentence),
                ),
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
    recent_activities: Vec<UserActivityResponse>,
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

        // Get recent activities
        let recent_activities: Vec<UserActivity> = user_activities::table
            .filter(user_activities::user_id.eq(user_id))
            .order(user_activities::created_at.desc())
            .limit(50)
            .load(conn)?;

        Ok::<_, diesel::result::Error>((user, admin, subscription_info, wallet, recent_payments, recent_activities))
    })
    .await
    .map_err(|_| StatusError::not_found().brief("user not found"))?;

    let (user, admin, subscription, wallet, recent_payments, recent_activities) = result;

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
        recent_activities: recent_activities.into_iter().map(|a| a.into()).collect(),
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

// ============================================================================
// User Statistics API
// ============================================================================

#[derive(Serialize, ToSchema)]
pub struct DailyUserStats {
    /// Date in YYYY-MM-DD format
    date: String,
    /// Number of new users registered on this date
    new_users: i64,
    /// Cumulative total users up to this date
    total_users: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct UserStatsQuery {
    /// Number of days to look back (default 30, max 365)
    days: Option<i32>,
}

/// Get daily user registration statistics
#[endpoint(tags("Admin"))]
pub async fn user_stats_daily(req: &mut Request) -> JsonResult<Vec<DailyUserStats>> {
    let query: UserStatsQuery = req.parse_queries().unwrap_or(UserStatsQuery { days: None });
    let days = query.days.unwrap_or(30).clamp(1, 365);

    let stats = with_conn(move |conn| {
        // Get daily registration counts for the last N days
        let results: Vec<DailyCount> = diesel::sql_query(format!(
            r#"
            SELECT
                to_char(created_at::date, 'YYYY-MM-DD') as date,
                COUNT(*) as count
            FROM base_users
            WHERE created_at >= CURRENT_DATE - INTERVAL '{} days'
            GROUP BY created_at::date
            ORDER BY created_at::date ASC
            "#,
            days
        ))
        .load::<DailyCount>(conn)?;

        // Get total users before the period
        let start_date = chrono::Utc::now().date_naive() - chrono::Duration::days(days as i64);
        let users_before: i64 = base_users::table
            .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(
                &format!("DATE(created_at) < '{}'", start_date),
            ))
            .select(count(base_users::id))
            .first(conn)?;

        // Build cumulative stats
        let mut cumulative = users_before;
        let stats: Vec<DailyUserStats> = results
            .into_iter()
            .map(|r| {
                cumulative += r.count;
                DailyUserStats {
                    date: r.date,
                    new_users: r.count,
                    total_users: cumulative,
                }
            })
            .collect();

        Ok::<_, diesel::result::Error>(stats)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get user stats"))?;

    json_ok(stats)
}

#[derive(QueryableByName)]
struct DailyCount {
    #[diesel(sql_type = diesel::sql_types::Text)]
    date: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

// ============================================================================
// AI Usage Statistics API
// ============================================================================

#[derive(Serialize, ToSchema)]
pub struct ServiceUsage {
    service_type: String,
    requests: i64,
    tokens: i64,
}

#[derive(Serialize, ToSchema)]
pub struct HourlyUsage {
    hour: i32,
    requests: i64,
    tokens: i64,
}

#[derive(Serialize, ToSchema)]
pub struct TodayAiUsage {
    /// Total number of AI requests today
    total_requests: i64,
    /// Total tokens used today
    total_tokens: i64,
    /// Total ASR seconds used today
    total_asr_seconds: f64,
    /// Total TTS seconds used today
    total_tts_seconds: f64,
    /// Usage breakdown by service type
    by_service: Vec<ServiceUsage>,
    /// Usage breakdown by hour
    by_hour: Vec<HourlyUsage>,
}

/// Get today's AI usage statistics
#[endpoint(tags("Admin"))]
pub async fn ai_usage_today() -> JsonResult<TodayAiUsage> {
    let stats = with_conn(move |conn| {
        let today = chrono::Utc::now().date_naive();

        // Get totals for today
        let totals: AiTotals = diesel::sql_query(format!(
            r#"
            SELECT
                COALESCE(COUNT(*), 0)::bigint as total_requests,
                COALESCE(SUM(COALESCE(tokens_used, 0)), 0)::bigint as total_tokens,
                COALESCE(SUM(CASE WHEN service_type = 'asr' THEN COALESCE(duration_seconds, 0) ELSE 0 END), 0)::float8 as total_asr_seconds,
                COALESCE(SUM(CASE WHEN service_type = 'tts' THEN COALESCE(duration_seconds, 0) ELSE 0 END), 0)::float8 as total_tts_seconds
            FROM ai_usage_records
            WHERE DATE(created_at) = '{}'
            "#,
            today
        ))
        .get_result::<AiTotals>(conn)?;

        // Get breakdown by service type
        let by_service: Vec<ServiceUsage> = diesel::sql_query(format!(
            r#"
            SELECT
                service_type,
                COUNT(*)::bigint as requests,
                COALESCE(SUM(COALESCE(tokens_used, 0)), 0)::bigint as tokens
            FROM ai_usage_records
            WHERE DATE(created_at) = '{}'
            GROUP BY service_type
            ORDER BY requests DESC
            "#,
            today
        ))
        .load::<ServiceUsageRow>(conn)?
        .into_iter()
        .map(|r| ServiceUsage {
            service_type: r.service_type,
            requests: r.requests,
            tokens: r.tokens,
        })
        .collect();

        // Get breakdown by hour
        let by_hour: Vec<HourlyUsage> = diesel::sql_query(format!(
            r#"
            SELECT
                EXTRACT(HOUR FROM created_at)::int as hour,
                COUNT(*)::bigint as requests,
                COALESCE(SUM(COALESCE(tokens_used, 0)), 0)::bigint as tokens
            FROM ai_usage_records
            WHERE DATE(created_at) = '{}'
            GROUP BY EXTRACT(HOUR FROM created_at)
            ORDER BY hour ASC
            "#,
            today
        ))
        .load::<HourlyUsageRow>(conn)?
        .into_iter()
        .map(|r| HourlyUsage {
            hour: r.hour,
            requests: r.requests,
            tokens: r.tokens,
        })
        .collect();

        Ok::<_, diesel::result::Error>(TodayAiUsage {
            total_requests: totals.total_requests,
            total_tokens: totals.total_tokens,
            total_asr_seconds: totals.total_asr_seconds,
            total_tts_seconds: totals.total_tts_seconds,
            by_service,
            by_hour,
        })
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get AI usage stats"))?;

    json_ok(stats)
}

#[derive(QueryableByName)]
struct AiTotals {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    total_requests: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    total_tokens: i64,
    #[diesel(sql_type = diesel::sql_types::Double)]
    total_asr_seconds: f64,
    #[diesel(sql_type = diesel::sql_types::Double)]
    total_tts_seconds: f64,
}

#[derive(QueryableByName)]
struct ServiceUsageRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    service_type: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    requests: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    tokens: i64,
}

#[derive(QueryableByName)]
struct HourlyUsageRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    hour: i32,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    requests: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    tokens: i64,
}

#[derive(Serialize, ToSchema)]
pub struct TopAiUser {
    user_id: i64,
    user_name: String,
    total_requests: i64,
    total_tokens: i64,
    chat_requests: i64,
    asr_requests: i64,
    tts_requests: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct TopUsersQuery {
    /// Maximum number of users to return (default 10, max 50)
    limit: Option<i64>,
}

/// Get top AI users today
#[endpoint(tags("Admin"))]
pub async fn ai_top_users(req: &mut Request) -> JsonResult<Vec<TopAiUser>> {
    let query: TopUsersQuery = req.parse_queries().unwrap_or(TopUsersQuery { limit: None });
    let limit = query.limit.unwrap_or(10).clamp(1, 50);

    let users = with_conn(move |conn| {
        let today = chrono::Utc::now().date_naive();

        // Get top users by token usage
        let top_users: Vec<TopAiUserRow> = diesel::sql_query(format!(
            r#"
            SELECT
                a.user_id,
                u.name as user_name,
                COUNT(*)::bigint as total_requests,
                COALESCE(SUM(COALESCE(a.tokens_used, 0)), 0)::bigint as total_tokens,
                COUNT(CASE WHEN a.service_type = 'chat' THEN 1 END)::bigint as chat_requests,
                COUNT(CASE WHEN a.service_type = 'asr' THEN 1 END)::bigint as asr_requests,
                COUNT(CASE WHEN a.service_type = 'tts' THEN 1 END)::bigint as tts_requests
            FROM ai_usage_records a
            JOIN base_users u ON u.id = a.user_id
            WHERE DATE(a.created_at) = '{}'
            GROUP BY a.user_id, u.name
            ORDER BY total_tokens DESC
            LIMIT {}
            "#,
            today, limit
        ))
        .load::<TopAiUserRow>(conn)?;

        let users: Vec<TopAiUser> = top_users
            .into_iter()
            .map(|r| TopAiUser {
                user_id: r.user_id,
                user_name: r.user_name,
                total_requests: r.total_requests,
                total_tokens: r.total_tokens,
                chat_requests: r.chat_requests,
                asr_requests: r.asr_requests,
                tts_requests: r.tts_requests,
            })
            .collect();

        Ok::<_, diesel::result::Error>(users)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get top AI users"))?;

    json_ok(users)
}

#[derive(QueryableByName)]
struct TopAiUserRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    user_id: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    user_name: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    total_requests: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    total_tokens: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    chat_requests: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    asr_requests: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    tts_requests: i64,
}

// ============================================================================
// Financial Management API
// ============================================================================

#[derive(Serialize, ToSchema)]
pub struct FinanceStats {
    /// Total revenue in cents (paid orders)
    total_revenue_cents: i64,
    /// Total number of orders
    total_orders: i64,
    /// Orders by status
    orders_by_status: Vec<OrderStatusCount>,
    /// Revenue today in cents
    revenue_today_cents: i64,
    /// Orders today
    orders_today: i64,
    /// Revenue this month in cents
    revenue_month_cents: i64,
    /// Orders this month
    orders_month: i64,
    /// Active subscriptions count
    active_subscriptions: i64,
    /// Trial subscriptions count
    trial_subscriptions: i64,
}

#[derive(Serialize, ToSchema)]
pub struct OrderStatusCount {
    status: String,
    count: i64,
}

/// Get financial statistics
#[endpoint(tags("Admin"))]
pub async fn finance_stats() -> JsonResult<FinanceStats> {
    let stats = with_conn(move |conn| {
        let today = chrono::Utc::now().date_naive();
        let month_start = today.with_day(1).unwrap_or(today);

        // Get all stats in one query
        let main_stats: FinanceStatsRow = diesel::sql_query(format!(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN status = 'paid' THEN amount_cents ELSE 0 END), 0)::bigint as total_revenue_cents,
                COUNT(*)::bigint as total_orders,
                COALESCE(SUM(CASE WHEN status = 'paid' AND DATE(paid_at) = '{}' THEN amount_cents ELSE 0 END), 0)::bigint as revenue_today_cents,
                COUNT(CASE WHEN DATE(created_at) = '{}' THEN 1 END)::bigint as orders_today,
                COALESCE(SUM(CASE WHEN status = 'paid' AND DATE(paid_at) >= '{}' THEN amount_cents ELSE 0 END), 0)::bigint as revenue_month_cents,
                COUNT(CASE WHEN DATE(created_at) >= '{}' THEN 1 END)::bigint as orders_month
            FROM payment_orders
            "#,
            today, today, month_start, month_start
        ))
        .get_result(conn)?;

        // Orders by status
        let orders_by_status: Vec<OrderStatusCount> = diesel::sql_query(
            r#"
            SELECT status, COUNT(*)::bigint as count
            FROM payment_orders
            GROUP BY status
            ORDER BY count DESC
            "#
        )
        .load::<OrderStatusCountRow>(conn)?
        .into_iter()
        .map(|r| OrderStatusCount {
            status: r.status,
            count: r.count,
        })
        .collect();

        // Subscription stats
        let sub_stats: SubscriptionStatsRow = diesel::sql_query(
            r#"
            SELECT
                COUNT(CASE WHEN status = 'active' THEN 1 END)::bigint as active_count,
                COUNT(CASE WHEN status = 'trial' THEN 1 END)::bigint as trial_count
            FROM user_subscriptions
            "#
        )
        .get_result(conn)?;

        Ok::<_, diesel::result::Error>(FinanceStats {
            total_revenue_cents: main_stats.total_revenue_cents,
            total_orders: main_stats.total_orders,
            orders_by_status,
            revenue_today_cents: main_stats.revenue_today_cents,
            orders_today: main_stats.orders_today,
            revenue_month_cents: main_stats.revenue_month_cents,
            orders_month: main_stats.orders_month,
            active_subscriptions: sub_stats.active_count,
            trial_subscriptions: sub_stats.trial_count,
        })
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get finance stats"))?;

    json_ok(stats)
}

#[derive(QueryableByName)]
struct FinanceStatsRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    total_revenue_cents: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    total_orders: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    revenue_today_cents: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    orders_today: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    revenue_month_cents: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    orders_month: i64,
}

#[derive(QueryableByName)]
struct SubscriptionStatsRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    active_count: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    trial_count: i64,
}

#[derive(QueryableByName)]
struct OrderStatusCountRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    status: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[derive(Serialize, ToSchema)]
pub struct OrderListItem {
    pub id: i64,
    pub user_id: i64,
    pub user_name: String,
    pub user_email: Option<String>,
    pub order_no: String,
    pub order_type: String,
    pub amount_cents: i64,
    pub amount_credits: i64,
    pub currency: String,
    pub provider: String,
    pub status: String,
    pub paid_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct OrderListResponse {
    orders: Vec<OrderListItem>,
    total: i64,
    page: i64,
    per_page: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct ListOrdersQuery {
    /// Page number (1-indexed)
    page: Option<i64>,
    /// Items per page (default 20)
    per_page: Option<i64>,
    /// Filter by status (pending, paid, failed, expired, refunded)
    status: Option<String>,
    /// Filter by order type (subscription, recharge)
    order_type: Option<String>,
    /// Filter by provider (alipay, wechat, etc.)
    provider: Option<String>,
    /// Search by order_no or user name
    search: Option<String>,
}

/// List all payment orders with pagination and filters
#[endpoint(tags("Admin"))]
pub async fn list_orders(req: &mut Request) -> JsonResult<OrderListResponse> {
    let query: ListOrdersQuery = req.parse_queries().unwrap_or(ListOrdersQuery {
        page: None,
        per_page: None,
        status: None,
        order_type: None,
        provider: None,
        search: None,
    });

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let status = query.status.clone();
    let order_type = query.order_type.clone();
    let provider = query.provider.clone();
    let search = query.search.clone();

    let (orders, total) = with_conn(move |conn| {
        // Build WHERE conditions
        let mut conditions: Vec<String> = vec!["1=1".to_string()];

        if let Some(ref s) = status {
            conditions.push(format!("status = '{}'", s.replace('\'', "''")));
        }

        if let Some(ref ot) = order_type {
            conditions.push(format!("order_type = '{}'", ot.replace('\'', "''")));
        }

        if let Some(ref p) = provider {
            conditions.push(format!("provider = '{}'", p.replace('\'', "''")));
        }

        if let Some(ref s) = search {
            conditions.push(format!("order_no ILIKE '%{}%'", s.replace('\'', "''").to_lowercase()));
        }

        let where_clause = conditions.join(" AND ");

        // Count query
        let count_result: CountRow = diesel::sql_query(format!(
            "SELECT COUNT(*)::bigint as count FROM payment_orders WHERE {}",
            where_clause
        ))
        .get_result(conn)?;

        let total = count_result.count;

        // Data query with user info
        let rows: Vec<OrderRow> = diesel::sql_query(format!(
            r#"
            SELECT
                o.id, o.user_id, o.order_no, o.order_type,
                o.amount_cents, o.amount_credits, o.currency, o.provider,
                o.status, o.paid_at, o.created_at,
                u.name as user_name, u.email as user_email
            FROM payment_orders o
            JOIN base_users u ON u.id = o.user_id
            WHERE {}
            ORDER BY o.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, per_page, offset
        ))
        .load(conn)?;

        let order_items: Vec<OrderListItem> = rows
            .into_iter()
            .map(|r| OrderListItem {
                id: r.id,
                user_id: r.user_id,
                user_name: r.user_name,
                user_email: r.user_email,
                order_no: r.order_no,
                order_type: r.order_type,
                amount_cents: r.amount_cents,
                amount_credits: r.amount_credits,
                currency: r.currency,
                provider: r.provider,
                status: r.status,
                paid_at: r.paid_at,
                created_at: r.created_at,
            })
            .collect();

        Ok::<_, diesel::result::Error>((order_items, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list orders"))?;

    json_ok(OrderListResponse {
        orders,
        total,
        page,
        per_page,
    })
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct OrderRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    id: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    user_id: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    order_no: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    order_type: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    amount_cents: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    amount_credits: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    currency: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    provider: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    status: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    paid_at: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    user_name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    user_email: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct SubscriptionListItem {
    pub id: i64,
    pub user_id: i64,
    pub user_name: String,
    pub user_email: Option<String>,
    pub plan_name: String,
    pub plan_tier: String,
    pub status: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub auto_renew: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct SubscriptionListResponse {
    subscriptions: Vec<SubscriptionListItem>,
    total: i64,
    page: i64,
    per_page: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct ListSubscriptionsQuery {
    /// Page number (1-indexed)
    page: Option<i64>,
    /// Items per page (default 20)
    per_page: Option<i64>,
    /// Filter by status (active, trial, expired, cancelled)
    status: Option<String>,
    /// Search by user name or email
    search: Option<String>,
}

/// List all subscriptions with pagination
#[endpoint(tags("Admin"))]
pub async fn list_subscriptions(req: &mut Request) -> JsonResult<SubscriptionListResponse> {
    let query: ListSubscriptionsQuery = req.parse_queries().unwrap_or(ListSubscriptionsQuery {
        page: None,
        per_page: None,
        status: None,
        search: None,
    });

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let status = query.status.clone();
    let search = query.search.clone();

    let (subscriptions, total) = with_conn(move |conn| {
        // Build WHERE conditions
        let mut conditions: Vec<String> = vec!["1=1".to_string()];

        if let Some(ref st) = status {
            conditions.push(format!("s.status = '{}'", st.replace('\'', "''")));
        }

        if let Some(ref term) = search {
            let escaped = term.replace('\'', "''").to_lowercase();
            conditions.push(format!(
                "(LOWER(u.name) LIKE '%{}%' OR LOWER(u.email) LIKE '%{}%')",
                escaped, escaped
            ));
        }

        let where_clause = conditions.join(" AND ");

        // Count query
        let count_result: CountRow = diesel::sql_query(format!(
            r#"
            SELECT COUNT(*)::bigint as count
            FROM user_subscriptions s
            JOIN base_users u ON u.id = s.user_id
            WHERE {}
            "#,
            where_clause
        ))
        .get_result(conn)?;

        let total = count_result.count;

        // Data query
        let rows: Vec<SubscriptionRow> = diesel::sql_query(format!(
            r#"
            SELECT
                s.id, s.user_id, s.plan_id, s.status, s.started_at, s.expires_at,
                s.auto_renew, s.created_at,
                u.name as user_name, u.email as user_email,
                p.name_zh as plan_name, p.tier as plan_tier
            FROM user_subscriptions s
            JOIN base_users u ON u.id = s.user_id
            JOIN subscription_plans p ON p.id = s.plan_id
            WHERE {}
            ORDER BY s.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, per_page, offset
        ))
        .load(conn)?;

        let items: Vec<SubscriptionListItem> = rows
            .into_iter()
            .map(|r| SubscriptionListItem {
                id: r.id,
                user_id: r.user_id,
                user_name: r.user_name,
                user_email: r.user_email,
                plan_name: r.plan_name,
                plan_tier: r.plan_tier,
                status: r.status,
                started_at: r.started_at,
                expires_at: r.expires_at,
                auto_renew: r.auto_renew,
                created_at: r.created_at,
            })
            .collect();

        Ok::<_, diesel::result::Error>((items, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list subscriptions"))?;

    json_ok(SubscriptionListResponse {
        subscriptions,
        total,
        page,
        per_page,
    })
}

#[derive(QueryableByName)]
struct SubscriptionRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    id: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    user_id: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    plan_id: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    status: String,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    started_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    auto_renew: bool,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    user_name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    user_email: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    plan_name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    plan_tier: String,
}

// ============================================================================
// Chat Scenario (Stage) Management
// ============================================================================

use salvo::oapi::extract::PathParam;
use crate::models::asset::{Stage, NewStage, UpdateStage, ReadSubject, NewReadSubject, UpdateReadSubject, ReadSentence, NewReadSentence, UpdateReadSentence};

#[derive(Serialize, ToSchema)]
pub struct StageListResponse {
    pub stages: Vec<Stage>,
    pub total: i64,
}

/// List all chat stages/scenarios
#[endpoint(tags("Admin"))]
pub async fn list_stages() -> JsonResult<StageListResponse> {
    let (stages, total) = with_conn(move |conn| {
        let stages = asset_stages::table
            .order(asset_stages::display_order.asc().nulls_last())
            .load::<Stage>(conn)?;
        let total = stages.len() as i64;
        Ok::<_, diesel::result::Error>((stages, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list stages"))?;

    json_ok(StageListResponse { stages, total })
}

/// Get a single stage by ID
#[endpoint(tags("Admin"))]
pub async fn get_stage(id: PathParam<i64>) -> JsonResult<Stage> {
    let stage_id = id.into_inner();
    let stage = with_conn(move |conn| {
        asset_stages::table
            .filter(asset_stages::id.eq(stage_id))
            .first::<Stage>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("stage not found"))?;

    json_ok(stage)
}

/// Create a new chat stage/scenario
#[endpoint(tags("Admin"))]
pub async fn create_stage(input: JsonBody<NewStage>) -> JsonResult<Stage> {
    let new_stage = input.into_inner();
    let stage = with_conn(move |conn| {
        diesel::insert_into(asset_stages::table)
            .values(&new_stage)
            .get_result::<Stage>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create stage"))?;

    json_ok(stage)
}

/// Update an existing chat stage/scenario
#[endpoint(tags("Admin"))]
pub async fn update_stage(
    id: PathParam<i64>,
    input: JsonBody<UpdateStage>,
) -> JsonResult<Stage> {
    let stage_id = id.into_inner();
    let update_data = input.into_inner();

    let stage = with_conn(move |conn| {
        diesel::update(asset_stages::table.filter(asset_stages::id.eq(stage_id)))
            .set(&update_data)
            .get_result::<Stage>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update stage"))?;

    json_ok(stage)
}

/// Delete a chat stage/scenario
#[endpoint(tags("Admin"))]
pub async fn delete_stage(id: PathParam<i64>) -> JsonResult<serde_json::Value> {
    let stage_id = id.into_inner();

    with_conn(move |conn| {
        diesel::delete(asset_stages::table.filter(asset_stages::id.eq(stage_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete stage"))?;

    json_ok(serde_json::json!({ "deleted": true }))
}

// ============================================================================
// Read-Along Subject Management
// ============================================================================

#[derive(Serialize, ToSchema)]
pub struct ReadSubjectListResponse {
    pub subjects: Vec<ReadSubject>,
    pub total: i64,
}

/// List all read-along subjects
#[endpoint(tags("Admin"))]
pub async fn list_read_subjects() -> JsonResult<ReadSubjectListResponse> {
    let (subjects, total) = with_conn(move |conn| {
        let subjects = asset_read_subjects::table
            .order(asset_read_subjects::id.asc())
            .load::<ReadSubject>(conn)?;
        let total = subjects.len() as i64;
        Ok::<_, diesel::result::Error>((subjects, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list read subjects"))?;

    json_ok(ReadSubjectListResponse { subjects, total })
}

/// Get a single read-along subject by ID
#[endpoint(tags("Admin"))]
pub async fn get_read_subject(id: PathParam<i64>) -> JsonResult<ReadSubject> {
    let subject_id = id.into_inner();
    let subject = with_conn(move |conn| {
        asset_read_subjects::table
            .filter(asset_read_subjects::id.eq(subject_id))
            .first::<ReadSubject>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("read subject not found"))?;

    json_ok(subject)
}

/// Create a new read-along subject
#[endpoint(tags("Admin"))]
pub async fn create_read_subject(input: JsonBody<NewReadSubject>) -> JsonResult<ReadSubject> {
    let new_subject = input.into_inner();
    let subject = with_conn(move |conn| {
        diesel::insert_into(asset_read_subjects::table)
            .values(&new_subject)
            .get_result::<ReadSubject>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create read subject"))?;

    json_ok(subject)
}

/// Update an existing read-along subject
#[endpoint(tags("Admin"))]
pub async fn update_read_subject(
    id: PathParam<i64>,
    input: JsonBody<UpdateReadSubject>,
) -> JsonResult<ReadSubject> {
    let subject_id = id.into_inner();
    let update_data = input.into_inner();

    let subject = with_conn(move |conn| {
        diesel::update(asset_read_subjects::table.filter(asset_read_subjects::id.eq(subject_id)))
            .set(&update_data)
            .get_result::<ReadSubject>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update read subject"))?;

    json_ok(subject)
}

/// Delete a read-along subject
#[endpoint(tags("Admin"))]
pub async fn delete_read_subject(id: PathParam<i64>) -> JsonResult<serde_json::Value> {
    let subject_id = id.into_inner();

    with_conn(move |conn| {
        diesel::delete(asset_read_subjects::table.filter(asset_read_subjects::id.eq(subject_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete read subject"))?;

    json_ok(serde_json::json!({ "deleted": true }))
}

// ============================================================================
// Read-Along Sentence Management
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct ReadSentenceQuery {
    /// Filter by subject ID
    pub subject_id: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct ReadSentenceListResponse {
    pub sentences: Vec<ReadSentence>,
    pub total: i64,
}

/// List read-along sentences (optionally filter by subject)
#[endpoint(tags("Admin"))]
pub async fn list_read_sentences(query: salvo::oapi::extract::QueryParam<ReadSentenceQuery, false>) -> JsonResult<ReadSentenceListResponse> {
    let subject_id = query.into_inner().and_then(|q| q.subject_id);

    let (sentences, total) = with_conn(move |conn| {
        let mut query = asset_read_sentences::table.into_boxed();

        if let Some(sid) = subject_id {
            query = query.filter(asset_read_sentences::subject_id.eq(sid));
        }

        let sentences = query
            .order((asset_read_sentences::subject_id.asc(), asset_read_sentences::sentence_order.asc()))
            .load::<ReadSentence>(conn)?;
        let total = sentences.len() as i64;
        Ok::<_, diesel::result::Error>((sentences, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list read sentences"))?;

    json_ok(ReadSentenceListResponse { sentences, total })
}

/// Get a single read-along sentence by ID
#[endpoint(tags("Admin"))]
pub async fn get_read_sentence(id: PathParam<i64>) -> JsonResult<ReadSentence> {
    let sentence_id = id.into_inner();
    let sentence = with_conn(move |conn| {
        asset_read_sentences::table
            .filter(asset_read_sentences::id.eq(sentence_id))
            .first::<ReadSentence>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("read sentence not found"))?;

    json_ok(sentence)
}

/// Create a new read-along sentence
#[endpoint(tags("Admin"))]
pub async fn create_read_sentence(input: JsonBody<NewReadSentence>) -> JsonResult<ReadSentence> {
    let new_sentence = input.into_inner();
    let sentence = with_conn(move |conn| {
        diesel::insert_into(asset_read_sentences::table)
            .values(&new_sentence)
            .get_result::<ReadSentence>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create read sentence"))?;

    json_ok(sentence)
}

/// Update an existing read-along sentence
#[endpoint(tags("Admin"))]
pub async fn update_read_sentence(
    id: PathParam<i64>,
    input: JsonBody<UpdateReadSentence>,
) -> JsonResult<ReadSentence> {
    let sentence_id = id.into_inner();
    let update_data = input.into_inner();

    let sentence = with_conn(move |conn| {
        diesel::update(asset_read_sentences::table.filter(asset_read_sentences::id.eq(sentence_id)))
            .set(&update_data)
            .get_result::<ReadSentence>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update read sentence"))?;

    json_ok(sentence)
}

/// Delete a read-along sentence
#[endpoint(tags("Admin"))]
pub async fn delete_read_sentence(id: PathParam<i64>) -> JsonResult<serde_json::Value> {
    let sentence_id = id.into_inner();

    with_conn(move |conn| {
        diesel::delete(asset_read_sentences::table.filter(asset_read_sentences::id.eq(sentence_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete read sentence"))?;

    json_ok(serde_json::json!({ "deleted": true }))
}
