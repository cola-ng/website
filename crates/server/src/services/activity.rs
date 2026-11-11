use horme::prelude::*;
use serde_json::Value as JsonValue;
use std::result::Result;

use crate::db::schema::user_activities;
use crate::db::with_conn;
use crate::models::{NewUserActivity, UserActivity};

/// Record a user activity
pub async fn record_activity(
    user_id: i64,
    activity_type: &str,
    description: Option<&str>,
    metadata: Option<JsonValue>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<UserActivity, String> {
    let activity_type = activity_type.to_string();
    let description = description.map(|s| s.to_string());
    let ip_address = ip_address.map(|s| s.to_string());
    let user_agent = user_agent.map(|s| s.to_string());

    with_conn!(move |conn| {
        diesel::insert_into(user_activities::table)
            .values(&NewUserActivity {
                user_id,
                activity_type,
                description,
                metadata,
                ip_address,
                user_agent,
            })
            .get_result::<UserActivity>(conn)
            .await
    })
    .await
    .map_err(|e| format!("failed to record activity: {}", e))
}

/// Record a login activity
pub async fn record_login(
    user_id: i64,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<UserActivity, String> {
    record_activity(
        user_id,
        crate::models::activity_types::LOGIN,
        Some("用户登录"),
        None,
        ip_address,
        user_agent,
    )
    .await
}

/// Record a registration activity
pub async fn record_register(
    user_id: i64,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<UserActivity, String> {
    record_activity(
        user_id,
        crate::models::activity_types::REGISTER,
        Some("用户注册"),
        None,
        ip_address,
        user_agent,
    )
    .await
}

/// Record a payment activity
pub async fn record_payment(
    user_id: i64,
    order_no: &str,
    amount_cents: i64,
    ip_address: Option<&str>,
) -> Result<UserActivity, String> {
    let metadata = serde_json::json!({
        "order_no": order_no,
        "amount_cents": amount_cents,
    });
    record_activity(
        user_id,
        crate::models::activity_types::PAYMENT,
        Some(&format!("支付订单 {}", order_no)),
        Some(metadata),
        ip_address,
        None,
    )
    .await
}

/// Record a subscription activity
pub async fn record_subscription(
    user_id: i64,
    plan_name: &str,
    action: &str, // "subscribe", "renew", "cancel"
) -> Result<UserActivity, String> {
    let metadata = serde_json::json!({
        "plan": plan_name,
        "action": action,
    });
    let description = match action {
        "subscribe" => format!("订阅 {}", plan_name),
        "renew" => format!("续费 {}", plan_name),
        "cancel" => format!("取消订阅 {}", plan_name),
        _ => format!("订阅变更 {}", plan_name),
    };
    record_activity(
        user_id,
        crate::models::activity_types::SUBSCRIPTION,
        Some(&description),
        Some(metadata),
        None,
        None,
    )
    .await
}

/// Record admin grant activity
pub async fn record_admin_grant(user_id: i64, granted_by: i64) -> Result<UserActivity, String> {
    let metadata = serde_json::json!({
        "granted_by": granted_by,
    });
    record_activity(
        user_id,
        crate::models::activity_types::ADMIN_GRANT,
        Some("被授予管理员权限"),
        Some(metadata),
        None,
        None,
    )
    .await
}

/// Record admin revoke activity
pub async fn record_admin_revoke(user_id: i64, revoked_by: i64) -> Result<UserActivity, String> {
    let metadata = serde_json::json!({
        "revoked_by": revoked_by,
    });
    record_activity(
        user_id,
        crate::models::activity_types::ADMIN_REVOKE,
        Some("管理员权限被撤销"),
        Some(metadata),
        None,
        None,
    )
    .await
}
