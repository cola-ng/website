use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::db::schema::*;

// ============================================================================
// Wallet Account
// ============================================================================

#[derive(Identifiable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = wallet_accounts)]
pub struct WalletAccount {
    pub id: i64,
    pub user_id: i64,
    pub balance: i64,
    pub frozen_balance: i64,
    pub total_deposited: i64,
    pub total_spent: i64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = wallet_accounts)]
pub struct NewWalletAccount {
    pub user_id: i64,
    pub currency: String,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = wallet_accounts)]
pub struct UpdateWalletBalance {
    pub balance: i64,
    pub frozen_balance: Option<i64>,
    pub total_deposited: Option<i64>,
    pub total_spent: Option<i64>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Wallet Transaction
// ============================================================================

#[derive(Identifiable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = wallet_transactions)]
pub struct WalletTransaction {
    pub id: i64,
    pub user_id: i64,
    pub wallet_id: i64,
    pub transaction_type: String,
    pub amount: i64,
    pub balance_before: i64,
    pub balance_after: i64,
    pub reference_type: Option<String>,
    pub reference_id: Option<i64>,
    pub description: Option<String>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = wallet_transactions)]
pub struct NewWalletTransaction {
    pub user_id: i64,
    pub wallet_id: i64,
    pub transaction_type: String,
    pub amount: i64,
    pub balance_before: i64,
    pub balance_after: i64,
    pub reference_type: Option<String>,
    pub reference_id: Option<i64>,
    pub description: Option<String>,
    pub metadata: Option<JsonValue>,
}

// ============================================================================
// Subscription Plan
// ============================================================================

#[derive(Identifiable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = subscription_plans)]
pub struct SubscriptionPlan {
    pub id: i64,
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub tier: String,
    pub billing_cycle: Option<String>,
    pub price_credits: i64,
    pub price_display: Option<String>,
    pub features: Option<JsonValue>,
    pub limits: Option<JsonValue>,
    pub trial_days: Option<i32>,
    pub is_active: bool,
    pub sort_order: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// User Subscription
// ============================================================================

#[derive(Identifiable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = user_subscriptions)]
pub struct UserSubscription {
    pub id: i64,
    pub user_id: i64,
    pub plan_id: i64,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub auto_renew: bool,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub last_payment_id: Option<i64>,
    pub next_billing_at: Option<DateTime<Utc>>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = user_subscriptions)]
pub struct NewUserSubscription {
    pub user_id: i64,
    pub plan_id: i64,
    pub status: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub auto_renew: bool,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub next_billing_at: Option<DateTime<Utc>>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = user_subscriptions)]
pub struct UpdateUserSubscription {
    pub status: Option<String>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub auto_renew: Option<bool>,
    pub last_payment_id: Option<i64>,
    pub next_billing_at: Option<Option<DateTime<Utc>>>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Payment Order
// ============================================================================

#[derive(Identifiable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = payment_orders)]
pub struct PaymentOrder {
    pub id: i64,
    pub user_id: i64,
    pub order_no: String,
    pub order_type: String,
    pub amount_credits: i64,
    pub amount_cents: i64,
    pub currency: String,
    pub provider: String,
    pub provider_order_id: Option<String>,
    pub provider_data: Option<JsonValue>,
    pub status: String,
    pub paid_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
    pub reference_type: Option<String>,
    pub reference_id: Option<i64>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = payment_orders)]
pub struct NewPaymentOrder {
    pub user_id: i64,
    pub order_no: String,
    pub order_type: String,
    pub amount_credits: i64,
    pub amount_cents: i64,
    pub currency: String,
    pub provider: String,
    pub reference_type: Option<String>,
    pub reference_id: Option<i64>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = payment_orders)]
pub struct UpdatePaymentOrder {
    pub provider_order_id: Option<String>,
    pub provider_data: Option<JsonValue>,
    pub status: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Payment Refund
// ============================================================================

#[derive(Identifiable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = payment_refunds)]
pub struct PaymentRefund {
    pub id: i64,
    pub user_id: i64,
    pub order_id: i64,
    pub refund_no: String,
    pub provider_refund_id: Option<String>,
    pub amount_cents: i64,
    pub amount_credits: i64,
    pub reason: Option<String>,
    pub status: String,
    pub completed_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
    pub processed_by: Option<i64>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = payment_refunds)]
pub struct NewPaymentRefund {
    pub user_id: i64,
    pub order_id: i64,
    pub refund_no: String,
    pub amount_cents: i64,
    pub amount_credits: i64,
    pub reason: Option<String>,
    pub processed_by: Option<i64>,
}

// ============================================================================
// Response Types
// ============================================================================

/// Wallet info with recent transactions
#[derive(Serialize, ToSchema, Debug)]
pub struct WalletInfo {
    pub account: WalletAccount,
    pub recent_transactions: Vec<WalletTransaction>,
}

/// User subscription with plan details
#[derive(Serialize, ToSchema, Debug)]
pub struct SubscriptionWithPlan {
    #[serde(flatten)]
    pub subscription: UserSubscription,
    pub plan: SubscriptionPlan,
}

/// Current subscription status
#[derive(Serialize, ToSchema, Debug)]
pub struct CurrentSubscription {
    pub has_subscription: bool,
    pub subscription: Option<SubscriptionWithPlan>,
    pub is_trial: bool,
    pub days_remaining: Option<i64>,
}
