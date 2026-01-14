use std::sync::LazyLock;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use salvo::oapi::ToSchema;
use serde_json::Value;

use crate::db::url_filter::JoinedOption;
use crate::models::trade::Coupon;
use crate::db::schema::*;

pub static BALANCE_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "realm_id", "kind", "current_amount", "initial_amount"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static BALANCE_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);

#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = wallet_balances)]
pub struct Balance {
    pub id: i64,
    pub realm_id: i64,
    pub kind: String, // cash, bonus, gift?
    #[salvo(schema(value_type = String))]
    pub current_amount: BigDecimal,
    #[salvo(schema(value_type = String))]
    pub initial_amount: BigDecimal,
    pub currency: String,
    pub parent_id: Option<i64>,  // 如果赠送的钱，需要知道是因为哪一个充值赠送的，如果发生退费，先退到赠送的里面。
    pub payment_id: Option<i64>, // 是那笔支付引起的余额变化，退费时需要。
    pub payment_intent_id: Option<i64>,
    pub stripe_intent_id: Option<String>,
    pub refund_id: Option<i64>,
    pub refund_item_id: Option<i64>,
    pub expires_at: Option<DateTime<Utc>>, // 如果是赠送的，可能有失效时间.
    pub description: Option<String>,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Deserialize, Clone, Debug)]
#[diesel(table_name = wallet_balances)]
pub struct NewBalance<'a> {
    pub realm_id: i64,
    pub kind: &'a str,
    pub current_amount: BigDecimal,
    pub initial_amount: BigDecimal,
    pub currency: &'a str,
    pub parent_id: Option<i64>,
    pub payment_id: Option<i64>,
    pub payment_intent_id: Option<i64>,
    pub stripe_intent_id: Option<&'a str>,
    pub refund_id: Option<i64>,
    pub refund_item_id: Option<i64>,
    pub expires_at: Option<DateTime<Utc>>,

    pub description: Option<&'a str>,
    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}

pub static HISTORY_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "realm_id", "kind", "happend_amount", "balance_change", "updated_by", "created_by"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static HISTORY_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = wallet_histories)]
pub struct History {
    pub id: i64,
    pub realm_id: i64,
    // payment:stripe, payment:balance 支付
    // refund:stripe, refund:balance 退款至, refund:failed 退款失败
    // recharge:stripe 充值
    // withdraw 提现
    pub kind: String,
    pub payment_id: Option<i64>,
    pub refund_id: Option<i64>,
    pub balance_ids: Vec<i64>,
    #[salvo(schema(value_type = String))]
    pub happend_amount: BigDecimal, // 正数，如果发生退款，从余额中自动扣除的赠费不计入这里。
    #[salvo(schema(value_type = String))]
    pub balance_change: BigDecimal, // 可以为负数，如果发生退款，从余额中自动扣除的赠费需要计入这里。
    pub currency: String,
    pub description: Option<String>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Serialize, Clone, Debug)]
#[diesel(table_name = wallet_histories)]
pub struct NewHistory<'a> {
    pub realm_id: i64,
    pub kind: &'a str,
    pub payment_id: Option<i64>,
    pub refund_id: Option<i64>,
    pub balance_ids: Vec<i64>,
    pub happend_amount: BigDecimal,
    pub balance_change: BigDecimal,
    pub currency: &'a str,
    pub description: Option<&'a str>,

    pub created_by: Option<i64>,
}
#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = wallet_balance_changes)]
pub struct BalanceChange {
    pub id: i64,
    pub realm_id: i64,
    pub history_id: i64,
    pub balance_id: i64,
    pub payment_id: Option<i64>,
    pub payment_intent_id: Option<i64>,
    pub refund_id: Option<i64>,
    pub refund_item_id: Option<i64>,
    #[salvo(schema(value_type = String))]
    pub initial_amount: BigDecimal,
    #[salvo(schema(value_type = String))]
    pub changed_amount: BigDecimal,
    pub currency: String,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Serialize, Clone, Debug)]
#[diesel(table_name = wallet_balance_changes)]
pub struct NewBalanceChange<'a> {
    pub realm_id: i64,
    pub history_id: i64,
    pub balance_id: i64,
    pub payment_id: Option<i64>,
    pub payment_intent_id: Option<i64>,
    pub refund_id: Option<i64>,
    pub refund_item_id: Option<i64>,
    pub initial_amount: BigDecimal,
    pub changed_amount: BigDecimal,
    pub currency: &'a str,

    pub created_by: Option<i64>,
}

pub static OBTAINED_COUPON_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "realm_id", "coupon_id", "updated_by", "created_by"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static OBTAINED_COUPON_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = wallet_obtained_coupons)]
pub struct ObtainedCoupon {
    pub id: i64,
    pub realm_id: i64,
    pub coupon_id: i64,
    pub started_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_used: bool,
    pub used_by_order_id: Option<i64>,
    pub used_by_invoice_id: Option<i64>,
    pub obtained_by_order_id: Option<i64>,
    pub obtained_by_invoice_id: Option<i64>,
    pub is_invalid: bool,
    pub invalid_for: Option<String>,
    pub description: Option<String>,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Serialize, Clone, Debug)]
#[diesel(table_name = wallet_obtained_coupons)]
pub struct NewObtainedCoupon<'a> {
    pub realm_id: i64,
    pub coupon_id: i64,
    pub started_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub obtained_by_order_id: Option<i64>,
    pub obtained_by_invoice_id: Option<i64>,
    pub description: Option<&'a str>,

    pub created_by: Option<i64>,
}

pub static USED_COUPON_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "realm_id", "coupon_id", "order_id", "invoice_id", "used_by", "used_at"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static USED_COUPON_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = wallet_used_coupons)]
pub struct UsedCoupon {
    pub id: i64,
    pub realm_id: i64,
    pub coupon_id: i64,
    pub obtained_id: Option<i64>,
    pub order_id: Option<i64>,
    pub invoice_id: Option<i64>,
    #[salvo(schema(value_type = Option<Coupon>))]
    pub snapshot: Option<Value>,

    pub used_by: Option<i64>,
    pub used_at: DateTime<Utc>,
    #[salvo(schema(value_type = String))]
    pub cut_amount: BigDecimal,
}

#[derive(Insertable, Serialize, Clone, Debug)]
#[diesel(table_name = wallet_used_coupons)]
pub struct NewUsedCoupon {
    pub realm_id: i64,
    pub coupon_id: i64,
    pub obtained_id: Option<i64>,
    pub order_id: Option<i64>,
    pub invoice_id: Option<i64>,
    pub snapshot: Option<Value>,

    pub used_by: Option<i64>,
    pub cut_amount: BigDecimal,
}

#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = wallet_cards)]
pub struct Card {
    pub id: i64,
    pub realm_id: i64,
    pub exp_month: Option<i64>,
    pub exp_year: Option<i64>,
    pub fingerprint: String,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Serialize, Clone, Debug)]
#[diesel(table_name = wallet_cards)]
pub struct NewCard<'a> {
    pub realm_id: i64,
    pub exp_month: i64,
    pub exp_year: i64,
    pub fingerprint: &'a str,

    pub created_by: Option<i64>,
}
