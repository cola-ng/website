use chrono::Utc;
use diesel::dsl::count;
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::oapi::extract::JsonBody;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::hoops::require_auth;
use crate::models::*;
use crate::{DepotExt, JsonResult, json_ok};

pub fn router() -> Router {
    Router::new()
        .push(
            Router::with_path("wallet")
                .hoop(require_auth)
                .get(get_wallet)
                .push(Router::with_path("transactions").get(list_transactions)),
        )
        .push(
            Router::with_path("subscription")
                .push(Router::with_path("plans").get(list_plans))
                .push(
                    Router::with_path("current")
                        .hoop(require_auth)
                        .get(get_current_subscription),
                )
                .push(
                    Router::with_path("subscribe")
                        .hoop(require_auth)
                        .post(subscribe),
                )
                .push(
                    Router::with_path("cancel")
                        .hoop(require_auth)
                        .post(cancel_subscription),
                ),
        )
}

// ============================================================================
// Wallet Endpoints
// ============================================================================

/// Get user's wallet info
#[endpoint(tags("Wallet"))]
pub async fn get_wallet(depot: &mut Depot) -> JsonResult<WalletInfo> {
    let user_id = depot.user_id()?;

    let (account, transactions) = with_conn(move |conn| {
        // Get or create wallet account
        let account: WalletAccount = wallet_accounts::table
            .filter(wallet_accounts::user_id.eq(user_id))
            .first(conn)
            .optional()?
            .map(Ok)
            .unwrap_or_else(|| {
                // Create wallet if not exists
                let new_wallet = NewWalletAccount {
                    user_id,
                    currency: "CNY".to_string(),
                };
                diesel::insert_into(wallet_accounts::table)
                    .values(&new_wallet)
                    .get_result(conn)
            })?;

        // Get recent transactions
        let transactions: Vec<WalletTransaction> = wallet_transactions::table
            .filter(wallet_transactions::user_id.eq(user_id))
            .order(wallet_transactions::created_at.desc())
            .limit(10)
            .load(conn)?;

        Ok::<_, diesel::result::Error>((account, transactions))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get wallet"))?;

    json_ok(WalletInfo {
        account,
        recent_transactions: transactions,
    })
}

#[derive(Deserialize, ToSchema)]
pub struct ListTransactionsQuery {
    /// Page number (1-indexed)
    page: Option<i64>,
    /// Items per page (default 20, max 100)
    per_page: Option<i64>,
    /// Filter by transaction type
    transaction_type: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct TransactionsResponse {
    transactions: Vec<WalletTransaction>,
    total: i64,
    page: i64,
    per_page: i64,
}

/// List wallet transactions with pagination
#[endpoint(tags("Wallet"))]
pub async fn list_transactions(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<TransactionsResponse> {
    let user_id = depot.user_id()?;
    let query: ListTransactionsQuery = req.parse_queries().unwrap_or(ListTransactionsQuery {
        page: None,
        per_page: None,
        transaction_type: None,
    });

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let transaction_type = query.transaction_type.clone();

    let (transactions, total) = with_conn(move |conn| {
        // Count query
        let total: i64 = if let Some(ref tx_type) = transaction_type {
            wallet_transactions::table
                .filter(wallet_transactions::user_id.eq(user_id))
                .filter(wallet_transactions::transaction_type.eq(tx_type))
                .select(count(wallet_transactions::id))
                .first(conn)?
        } else {
            wallet_transactions::table
                .filter(wallet_transactions::user_id.eq(user_id))
                .select(count(wallet_transactions::id))
                .first(conn)?
        };

        // Data query
        let transactions: Vec<WalletTransaction> = if let Some(ref tx_type) = transaction_type {
            wallet_transactions::table
                .filter(wallet_transactions::user_id.eq(user_id))
                .filter(wallet_transactions::transaction_type.eq(tx_type))
                .order(wallet_transactions::created_at.desc())
                .offset(offset)
                .limit(per_page)
                .load(conn)?
        } else {
            wallet_transactions::table
                .filter(wallet_transactions::user_id.eq(user_id))
                .order(wallet_transactions::created_at.desc())
                .offset(offset)
                .limit(per_page)
                .load(conn)?
        };

        Ok::<_, diesel::result::Error>((transactions, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list transactions"))?;

    json_ok(TransactionsResponse {
        transactions,
        total,
        page,
        per_page,
    })
}

// ============================================================================
// Subscription Endpoints
// ============================================================================

/// List available subscription plans
#[endpoint(tags("Subscription"))]
pub async fn list_plans() -> JsonResult<Vec<SubscriptionPlan>> {
    let plans = with_conn(move |conn| {
        subscription_plans::table
            .filter(subscription_plans::is_active.eq(true))
            .order(subscription_plans::sort_order.asc())
            .load::<SubscriptionPlan>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list plans"))?;

    json_ok(plans)
}

/// Get user's current subscription status
#[endpoint(tags("Subscription"))]
pub async fn get_current_subscription(depot: &mut Depot) -> JsonResult<CurrentSubscription> {
    let user_id = depot.user_id()?;

    let result = with_conn(move |conn| {
        // Get active subscription
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

        let result = match subscription {
            Some(sub) => {
                let plan: SubscriptionPlan = subscription_plans::table
                    .filter(subscription_plans::id.eq(sub.plan_id))
                    .first(conn)?;

                let is_trial = sub.status == "trial";
                let days_remaining = sub.expires_at.map(|exp| {
                    let now = Utc::now();
                    (exp - now).num_days().max(0)
                });

                CurrentSubscription {
                    has_subscription: true,
                    subscription: Some(SubscriptionWithPlan {
                        subscription: sub,
                        plan,
                    }),
                    is_trial,
                    days_remaining,
                }
            }
            None => CurrentSubscription {
                has_subscription: false,
                subscription: None,
                is_trial: false,
                days_remaining: None,
            },
        };

        Ok::<_, diesel::result::Error>(result)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get subscription"))?;

    json_ok(result)
}

#[derive(Deserialize, ToSchema)]
pub struct SubscribeRequest {
    /// Plan code to subscribe to
    plan_code: String,
    /// Start as trial if available
    use_trial: Option<bool>,
}

#[derive(Serialize, ToSchema)]
pub struct SubscribeResponse {
    subscription: UserSubscription,
    plan: SubscriptionPlan,
    /// If payment required, this contains order info
    payment_required: bool,
    /// Order number if payment required
    order_no: Option<String>,
}

/// Subscribe to a plan
#[endpoint(tags("Subscription"))]
pub async fn subscribe(
    input: JsonBody<SubscribeRequest>,
    depot: &mut Depot,
) -> JsonResult<SubscribeResponse> {
    let user_id = depot.user_id()?;
    let plan_code = input.plan_code.clone();
    let use_trial = input.use_trial.unwrap_or(true);

    let (subscription, plan) = with_conn(move |conn| {
        // Get the plan
        let plan: SubscriptionPlan = subscription_plans::table
            .filter(subscription_plans::code.eq(&plan_code))
            .filter(subscription_plans::is_active.eq(true))
            .first(conn)
            .optional()?
            .ok_or_else(|| diesel::result::Error::NotFound)?;

        // Check if user already has active subscription
        let existing: Option<UserSubscription> = user_subscriptions::table
            .filter(user_subscriptions::user_id.eq(user_id))
            .filter(
                user_subscriptions::status
                    .eq("active")
                    .or(user_subscriptions::status.eq("trial")),
            )
            .first(conn)
            .optional()?;

        if existing.is_some() {
            return Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                Box::new("already subscribed".to_string()),
            ));
        }

        // For free tier, create subscription immediately
        if plan.tier == "free" {
            let new_sub = NewUserSubscription {
                user_id,
                plan_id: plan.id,
                status: "active".to_string(),
                expires_at: None,
                auto_renew: false,
                trial_ends_at: None,
                next_billing_at: None,
            };

            let subscription: UserSubscription = diesel::insert_into(user_subscriptions::table)
                .values(&new_sub)
                .get_result(conn)?;

            return Ok((subscription, plan));
        }

        // For paid plans, check if trial is available and requested
        let trial_days = plan.trial_days.unwrap_or(0);
        if use_trial && trial_days > 0 {
            let trial_ends = Utc::now() + chrono::Duration::days(trial_days as i64);
            let new_sub = NewUserSubscription {
                user_id,
                plan_id: plan.id,
                status: "trial".to_string(),
                expires_at: Some(trial_ends),
                auto_renew: true,
                trial_ends_at: Some(trial_ends),
                next_billing_at: Some(trial_ends),
            };

            let subscription: UserSubscription = diesel::insert_into(user_subscriptions::table)
                .values(&new_sub)
                .get_result(conn)?;

            return Ok((subscription, plan));
        }

        // For paid plans without trial, need payment
        // For now, just return error - actual payment flow would create order
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::NotNullViolation,
            Box::new("payment_required".to_string()),
        ))
    })
    .await
    .map_err(|e| {
        if e.contains("already subscribed") {
            StatusError::conflict().brief("already have an active subscription")
        } else if e.contains("payment_required") {
            StatusError::payment_required().brief("payment required for this plan")
        } else if e.contains("NotFound") {
            StatusError::not_found().brief("plan not found")
        } else {
            StatusError::internal_server_error().brief("failed to subscribe")
        }
    })?;

    json_ok(SubscribeResponse {
        subscription,
        plan,
        payment_required: false,
        order_no: None,
    })
}

#[derive(Deserialize, ToSchema)]
pub struct CancelSubscriptionRequest {
    /// Reason for cancellation
    reason: Option<String>,
}

/// Cancel current subscription
#[endpoint(tags("Subscription"))]
pub async fn cancel_subscription(
    input: JsonBody<CancelSubscriptionRequest>,
    depot: &mut Depot,
) -> JsonResult<UserSubscription> {
    let user_id = depot.user_id()?;
    let reason = input.reason.clone();

    let subscription = with_conn(move |conn| {
        // Get active subscription
        let subscription: UserSubscription = user_subscriptions::table
            .filter(user_subscriptions::user_id.eq(user_id))
            .filter(
                user_subscriptions::status
                    .eq("active")
                    .or(user_subscriptions::status.eq("trial")),
            )
            .first(conn)
            .optional()?
            .ok_or_else(|| diesel::result::Error::NotFound)?;

        // Update subscription to cancelled
        let update = UpdateUserSubscription {
            status: Some("cancelled".to_string()),
            cancelled_at: Some(Utc::now()),
            cancel_reason: reason,
            auto_renew: Some(false),
            expires_at: None,
            last_payment_id: None,
            next_billing_at: Some(None),
            updated_at: Utc::now(),
        };

        let updated: UserSubscription = diesel::update(
            user_subscriptions::table.filter(user_subscriptions::id.eq(subscription.id)),
        )
        .set(&update)
        .get_result(conn)?;

        Ok::<_, diesel::result::Error>(updated)
    })
    .await
    .map_err(|e| {
        if e.contains("NotFound") {
            StatusError::not_found().brief("no active subscription found")
        } else {
            StatusError::internal_server_error().brief("failed to cancel subscription")
        }
    })?;

    json_ok(subscription)
}
