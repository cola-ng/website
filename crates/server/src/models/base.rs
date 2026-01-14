use std::sync::LazyLock;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

use salvo::http::StatusError;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AppResult;
use crate::db::url_filter::JoinedOption;
use crate::schema::*;

pub static REALM_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "name", "kind", "level", "labels", "updated_by", "created_by"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static REALM_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct Realm {
    pub id: i64,

    pub name: String,
    pub kind: String,
    pub max_allowed_mixture_duration: i64,
    pub stripe_customer_id: Option<String>,
    pub referral_code: String,
    pub referred_by: Option<i64>,
    #[serde(skip_serializing)]
    pub waiting_referred_coupon: bool,
    //level 和 labels 仅允许内部人员修改，用于表示客户类型。
    pub level: i64,
    pub labels: Vec<String>,

    pub is_disabled: bool,
    pub disabled_at: Option<DateTime<Utc>>,
    pub disabled_by: Option<i64>,

    #[salvo(schema(value_type = Object, additional_properties = true))]
    pub profile: Value,

    #[salvo(schema(value_type = String))]
    pub balance: BigDecimal,

    pub force_quiz_watermark: Option<String>,

    pub description: Option<String>,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Deserialize, Clone, Debug)]
#[diesel(table_name = realms)]
pub struct NewRealm<'a> {
    pub name: &'a str,
    pub kind: &'a str,
    pub referral_code: &'a str,
    pub referred_by: Option<i64>,
    pub waiting_referred_coupon: bool,
    pub level: i64,
    pub labels: Vec<&'a str>,
    pub profile: Value,
    pub description: Option<&'a str>,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}

pub static USER_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        "id",
        "ident_name",
        "display_name",
        "in_kernel",
        "is_limited",
        "inviter_id",
        "updated_by",
        "created_by",
    ]
    .into_iter()
    .map(String::from)
    .collect()
});
pub static USER_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(|| {
    url_filter_joined_options![
        "emails", "id"=>"user_id", "e.value"=>"value";
        "phones", "id"=>"user_id", "p.value"=>"value";
    ]
});
// pub static USER_SEARCH_TMPL: &str = "id::varchar(255)='{{data}}' or ident_name ilike E'%{{data}}%' or display_name ilike E'%{{data}}%'";
pub static USER_SEARCH_TMPL: &str = "id::varchar(255)='{{data}}' or ident_name ilike E'%{{data}}%' or display_name ilike E'%{{data}}%' or id in (select user_id from emails where emails.value ilike E'%{{data}}%') or id in (select user_id from phones where phones.value ilike E'%{{data}}%')";
#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct User {
    pub id: i64,
    pub realm_id: i64,

    pub ident_name: String,
    pub display_name: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub in_kernel: bool,
    pub is_root: bool,

    pub is_verified: bool,
    pub verified_at: Option<DateTime<Utc>>,

    pub is_limited: bool,
    pub limited_at: Option<DateTime<Utc>>,
    pub limited_by: Option<i64>,

    pub is_locked: bool,
    pub locked_at: Option<DateTime<Utc>>,
    pub locked_by: Option<i64>,

    pub is_disabled: bool,
    pub disabled_at: Option<DateTime<Utc>>,
    pub disabled_by: Option<i64>,

    pub inviter_id: Option<i64>,
    pub invite_replied: Option<bool>,
    #[salvo(schema(value_type = Object, additional_properties = true))]
    pub profile: Value,
    #[serde(skip_serializing)]
    pub is_robot: bool,
    pub avatar: Option<String>,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub points: i64,
}
#[derive(Insertable, Deserialize, Clone, Debug)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub realm_id: i64,

    pub ident_name: &'a str,
    pub display_name: &'a str,
    pub password: &'a str,
    pub in_kernel: bool,
    pub is_root: bool,
    pub is_verified: bool,
    pub is_limited: bool,
    pub inviter_id: Option<i64>,
    pub invite_replied: Option<bool>,
    pub profile: Value,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,

    pub points: i64,
}
impl User {
    pub fn must_in_kernel(&self) -> AppResult<&Self> {
        if !self.in_kernel {
            Err(StatusError::forbidden().brief("Current user is not in kernel.").into())
        } else {
            Ok(self)
        }
    }
    pub fn must_root(&self) -> AppResult<&Self> {
        if !self.is_root {
            Err(StatusError::forbidden().brief("Current user is not root.").into())
        } else {
            Ok(self)
        }
    }
    pub fn must_in_kernel_or_realm(&self, realm_id: i64, conn: &mut PgConnection) -> AppResult<&Self> {
        let query = realm_users::table
            .filter(realm_users::realm_id.eq(realm_id))
            .filter(realm_users::user_id.eq(self.id));
        if !self.in_kernel && !diesel_exists!(query, conn) {
            Err(StatusError::forbidden().brief("Current user is not in kernel or realm.").into())
        } else {
            Ok(self)
        }
    }
}

pub static ROLE_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "name", "description", "realm_id", "owner_id", "updated_by", "created_by"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static ROLE_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
pub struct Role {
    pub id: i64,
    pub code: Option<String>,
    pub name: String,
    pub kind: String,
    pub realm_id: i64,
    pub owner_id: i64,
    pub description: Option<String>,
    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Debug)]
#[diesel(table_name = roles)]
pub struct NewRole<'a> {
    pub code: Option<&'a str>,
    pub name: &'a str,
    pub kind: Option<&'a str>,
    pub realm_id: i64,
    pub owner_id: i64,
    pub description: Option<&'a str>,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}

#[derive(Queryable, Insertable, Serialize, Debug)]
pub struct RoleUser {
    pub realm_id: i64,
    pub role_id: i64,
    pub user_id: i64,
}

#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
pub struct Operation {
    pub id: i64,
    pub code: String,
    pub title: String,
    pub entity: String,
    pub action: String,
    pub scope: String,
    pub filter_name: Option<String>,
    pub filter_value: Option<String>,
    #[salvo(schema(value_type = Option<Object>))]
    pub depends: Option<Value>,
    #[salvo(schema(value_type = Option<Object>))]
    pub contains: Option<Value>,
    pub sort_rank: i64,
    pub description: Option<String>,
}

#[derive(Identifiable, Insertable, Queryable, QueryableByName, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = permissions)]
pub struct Permission {
    pub id: i64,
    pub realm_id: i64,
    pub role_id: i64,
    pub entity: String,
    pub action: String,
    pub scope: String,
    pub filter_name: String,
    pub filter_int_value: Option<i64>,
    pub filter_text_value: Option<String>,
}
#[derive(Insertable, Debug)]
#[diesel(table_name = permissions)]
pub struct NewPermission<'a> {
    pub realm_id: i64,
    pub role_id: i64,
    pub entity: &'a str,
    pub action: &'a str,
    pub scope: &'a str,
    pub filter_name: &'a str,
    pub filter_int_value: Option<i64>,
    pub filter_text_value: Option<&'a str>,
}

#[derive(Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
#[diesel(table_name = realm_users)]
pub struct RealmUser {
    pub id: i64,
    pub realm_id: i64,
    pub user_id: i64,
    pub is_root: bool,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Serialize, Clone, Debug)]
#[diesel(table_name = realm_users)]
pub struct NewRealmUser {
    pub realm_id: i64,
    pub user_id: i64,
    pub is_root: bool,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}

pub static EMAIL_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        "id",
        "user_id",
        "value",
        "is_verified",
        "is_master",
        "is_subscribed",
        "updated_by",
        "created_by",
    ]
    .into_iter()
    .map(String::from)
    .collect()
});
pub static EMAIL_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct Email {
    pub id: i64,
    pub user_id: i64,
    pub value: String,
    pub domain: String,
    pub is_verified: bool,
    pub is_master: bool,
    pub is_subscribed: bool,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Serialize, Clone, Debug)]
#[diesel(table_name = emails)]
pub struct NewEmail<'a> {
    pub user_id: i64,
    pub value: &'a str,
    pub is_verified: bool,
    pub is_subscribed: bool,
    pub is_master: bool,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct PostedEmail {
    #[serde(default)]
    pub value: String,
    #[serde(default = "crate::default_as_false")]
    pub is_subscribed: bool,
}
impl Default for PostedEmail {
    fn default() -> Self {
        PostedEmail {
            value: "".to_owned(),
            is_subscribed: false,
        }
    }
}

pub static EMAIL_SUBSCRIPTION_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "user_id", "topic", "updated_by", "created_by"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static EMAIL_SUBSCRIPTION_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
/// 如果对应的记录不存在，认为是默认接收提醒邮件的。
/// - trade_order_created
/// - stock_exhibit_develop_approved(应该不需要，现在只发内部管理员)
/// - interflow_thread_created
/// - interflow_thread_recalled
/// - interflow_thread_updated
#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct EmailSubscription {
    pub id: i64,
    pub user_id: i64,
    pub realm_id: Option<i64>,
    pub topic: String,
    pub value: bool,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Serialize, Clone, Debug)]
#[diesel(table_name = email_subscriptions)]
pub struct NewEmailSubscription<'a> {
    pub user_id: i64,
    pub realm_id: Option<i64>,
    pub topic: &'a str,
    pub value: bool,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}

pub static PHONE_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        "id",
        "user_id",
        "value",
        "is_verified",
        "is_master",
        "is_subscribed",
        "updated_by",
        "created_by",
    ]
    .into_iter()
    .map(String::from)
    .collect()
});
pub static PHONE_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct Phone {
    pub id: i64,
    pub user_id: i64,
    pub value: String,
    pub is_verified: bool,
    pub is_master: bool,
    pub is_subscribed: bool,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Serialize, Clone, Debug)]
#[diesel(table_name = phones)]
pub struct NewPhone<'a> {
    pub user_id: i64,
    pub value: &'a str,
    pub is_verified: bool,
    pub is_subscribed: bool,
    pub is_master: bool,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}
#[derive(Deserialize, ToSchema, Debug)]
pub struct PostedPhone {
    #[serde(default)]
    pub value: String,
    #[serde(default = "crate::default_as_false")]
    pub is_subscribed: bool,
}
impl Default for PostedPhone {
    fn default() -> Self {
        PostedPhone {
            value: "".to_owned(),
            is_subscribed: false,
        }
    }
}

pub static ADDRESS_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "realm_id", "country", "state", "city", "line1", "line2", "updated_by", "created_by"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static ADDRESS_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
#[diesel(table_name = addresses)]
pub struct Address {
    pub id: i64,
    pub realm_id: i64,
    pub country: String,
    pub state: Option<String>,
    pub city: Option<String>,
    pub post_code: Option<String>,
    pub line1: Option<String>,
    pub line2: Option<String>,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Serialize, Clone, Debug)]
#[diesel(table_name = addresses)]
pub struct NewAddress<'a> {
    pub realm_id: i64,
    pub country: &'a str,
    pub state: Option<&'a str>,
    pub city: Option<&'a str>,
    pub post_code: Option<&'a str>,
    pub line1: Option<&'a str>,
    pub line2: Option<&'a str>,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}

pub static ACCESS_TOKEN_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "user_id", "name", "kind", "value", "updated_by", "created_by"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static ACCESS_TOKEN_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
// #[belongs_to(User)]
pub struct AccessToken {
    pub id: i64,
    pub user_id: i64,
    pub name: Option<String>,
    pub kind: String,
    pub value: String,
    pub device: Option<String>,
    pub expires_at: DateTime<Utc>,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Serialize, Clone, Debug)]
#[diesel(table_name = access_tokens)]
pub struct NewAccessToken<'a> {
    pub user_id: i64,
    pub name: Option<&'a str>,
    pub kind: &'a str,
    pub value: &'a str,
    pub device: Option<&'a str>,
    pub expires_at: DateTime<Utc>,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}

#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct SecurityCode {
    pub id: i64,
    pub user_id: i64,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub value: String,
    pub send_method: String,
    pub consumed_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = security_codes)]
pub struct NewSecurityCode<'a> {
    pub user_id: i64,
    pub email: Option<&'a str>,
    pub phone: Option<&'a str>,
    pub value: &'a str,
    pub send_method: &'a str,
    pub expires_at: DateTime<Utc>,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}
pub static OAUTH_ACCESS_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "realm_id", "owner_id", "platform", "token_type", "updated_by", "created_by"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static OAUTH_ACCESS_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
#[diesel(table_name = oauth_realm_accesses)]
pub struct OauthRealmAccess {
    pub id: i64,
    pub realm_id: i64,
    pub owner_id: i64,
    pub platform: String,
    pub access_token: String,
    pub token_type: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub shopify_shop: Option<String>,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Debug)]
#[diesel(table_name = oauth_realm_accesses)]
pub struct NewOauthRealmAccess<'a> {
    pub realm_id: i64,
    pub owner_id: i64,
    pub platform: &'a str,
    pub access_token: &'a str,
    pub token_type: &'a str,
    pub refresh_token: Option<&'a str>,
    pub expires_at: DateTime<Utc>,
    pub shopify_shop: Option<&'a str>,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}
pub static OAUTH_USER_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "user_id", "platform", "token_type", "updated_by", "created_by"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static OAUTH_USERS_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
#[diesel(table_name = oauth_users)]
pub struct OauthUser {
    pub id: i64,
    pub user_id: Option<i64>,
    pub platform: String,
    pub me_id: String,
    pub me_full_name: Option<String>,
    pub me_email: Option<String>,
    pub me_phone: Option<String>,
    pub access_token: String,
    pub token_type: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub shopify_shop: Option<String>,

    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, AsChangeset, Clone, Debug)]
#[diesel(table_name = oauth_users)]
pub struct NewOauthUser<'a> {
    pub user_id: Option<i64>,
    pub platform: &'a str,
    pub me_id: &'a str,
    pub me_full_name: Option<&'a str>,
    pub me_email: Option<&'a str>,
    pub me_phone: Option<&'a str>,
    pub access_token: &'a str,
    pub token_type: &'a str,
    pub refresh_token: Option<&'a str>,
    pub expires_at: DateTime<Utc>,
    pub shopify_shop: Option<&'a str>,

    pub created_at: DateTime<Utc>,
}

pub static STEWARD_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        "id",
        "user_id",
        "serviced_count",
        "auto_assignable",
        "available",
        "is_supervisor",
        "updated_by",
        "created_by",
    ]
    .into_iter()
    .map(String::from)
    .collect()
});
pub static STEWARD_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
#[derive(Queryable, Insertable, AsChangeset, Serialize, Deserialize, ToSchema, Clone, Debug)]
#[diesel(table_name = realm_stewards)]
pub struct RealmSteward {
    pub realm_id: i64,
    pub user_id: i64,
}
#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
#[diesel(table_name = stewards)]
pub struct Steward {
    pub id: i64,
    pub user_id: i64,
    pub serviced_count: i64,
    pub auto_assignable: bool,
    pub available: bool,
    pub is_supervisor: bool,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, AsChangeset, Clone, Debug)]
#[diesel(table_name = stewards)]
pub struct NewSteward {
    pub user_id: i64,
    pub auto_assignable: bool,
    pub is_supervisor: bool,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}

pub static NOTIFICATION_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "owner_id", "sender_id", "kind", "is_read", "read_at", "sent_at"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static NOTIFICATION_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
pub static NOTIFICATION_SEARCH_TMPL: &str = r#"id::varchar(255)='{{data}}' or subject ilike E'%{{data}}%' or body ilike E'%{{data}}%'"#;
#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = notifications)]
pub struct Notification {
    pub id: i64,
    pub realm_id: i64,
    pub owner_id: i64,
    pub sender_id: i64,
    pub subject: String,
    pub body: String,
    pub kind: String,
    pub is_read: bool,
    pub stream_id: Option<i64>,
    pub thread_id: Option<i64>,
    pub entity: Option<String>,
    pub record_id: Option<i64>,
    #[salvo(schema(value_type = Option<Object>))]
    pub extra: Option<Value>,

    pub read_at: Option<DateTime<Utc>>,
    pub sent_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize, Clone, Debug)]
#[diesel(table_name = notifications)]
pub struct NewNotification<'a> {
    pub owner_id: i64,
    pub realm_id: i64,
    pub sender_id: i64,
    pub subject: &'a str,
    pub body: &'a str,
    pub kind: &'a str,
    pub stream_id: Option<i64>,
    pub thread_id: Option<i64>,
    pub entity: Option<&'a str>,
    pub record_id: Option<i64>,
    pub extra: Option<Value>,
}
owner_entity_accessible!(Notification);

pub static NOTIFICATION_GROUP_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "realm_id", "owner_id", "latest_sender_id", "latest_sent_at"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static NOTIFICATION_GROUP_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
pub static NOTIFICATION_GROUP_SEARCH_TMPL: &str = r#"subject ilike E'%{{data}}%'"#;
#[derive(Identifiable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = notification_groups)]
pub struct NotificationGroup {
    pub id: i64,
    pub realm_id: i64,
    pub owner_id: i64,
    pub kind: String,
    pub subject: String,
    pub read_count: i64,
    pub unread_count: i64,

    pub latest_id: i64,
    pub latest_sender_id: i64,
    pub latest_sent_at: DateTime<Utc>,
}

pub static EMAIL_MESSAGE_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "kind", "recipients", "subject", "text_body", "html_body", "status"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static EMAIL_MESSAGE_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
pub static EMAIL_MESSAGE_SEARCH_TMPL: &str = "id::varchar(255)='{{data}}' or subject ilike E'%{{data}}%' or recipient_email ilike E'%{{data}}%'";
#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = email_messages)]
pub struct EmailMessage {
    pub id: i64,
    pub kind: String,
    pub thread_id: Option<i64>,
    pub recipient_id: Option<i64>,
    pub recipient_email: String,
    // #[serde(skip)]
    pub reply_token: Option<String>,
    // #[serde(skip)]
    pub view_token: String,
    pub subject: String,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    #[salvo(schema(value_type = Option<Object>))]
    pub attachments: Value,
    pub track_flag: Option<String>,
    pub sent_in: Option<DateTime<Utc>>,
    pub sent_status: String, // "delay", "sending", "sent", "failed"
    pub sent_error: Option<String>,
    pub is_read: bool,
    pub is_replied: bool,

    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize, Clone, Debug)]
#[diesel(table_name = email_messages)]
pub struct NewEmailMessage<'a> {
    pub kind: &'a str,
    pub thread_id: Option<i64>,
    pub recipient_id: Option<i64>,
    pub recipient_email: &'a str,
    pub reply_token: Option<&'a str>,
    pub view_token: &'a str,
    pub subject: &'a str,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub attachments: Value,
    pub track_flag: Option<&'a str>,
    pub sent_in: Option<DateTime<Utc>>,
    pub sent_status: &'a str,
    pub sent_error: Option<&'a str>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EmailAttachment {
    pub path: String,
    // pub key: String,
    // pub hash: String,
}

#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = delay_notifications)]
pub struct DelayNotification {
    pub id: i64,
    pub kind: String,
    pub realm_id: i64,
    pub entity: String,
    pub record_id: i64,
    pub action: String,
    pub sender_id: Option<i64>,
    pub recipient_ids: Vec<i64>,
    #[salvo(schema(value_type = Option<Object>))]
    pub extra: Option<Value>,
    pub sent_in: DateTime<Utc>,
    pub revoke_key: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize, Clone, Debug)]
#[diesel(table_name = delay_notifications)]
pub struct NewDelayNotification<'a> {
    pub kind: &'a str,
    pub realm_id: i64,
    pub entity: &'a str,
    pub record_id: i64,
    pub action: &'a str,
    pub sender_id: Option<i64>,
    pub recipient_ids: Option<Vec<i64>>,
    pub extra: Option<Value>,
    pub sent_in: DateTime<Utc>,
    pub revoke_key: Option<&'a str>,
}

#[derive(QueryableByName, Debug)]
pub struct TableId {
    #[diesel(sql_type = ::diesel::sql_types::BigInt)]
    #[diesel(column_name = id)]
    pub id: i64,
}

entity_accessible!(Role, roles);
entity_accessible!(Realm, realms);

pub static SENTENCE_FILTER_FIELDS: LazyLock<Vec<String>> =
    LazyLock::new(|| vec!["id", "kind", "value", "is_delisted"].into_iter().map(String::from).collect());
pub static SENTENCE_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
pub static SENTENCE_SEARCH_TMPL: &str = "id::varchar(255)='{{data}}' or kind ilike E'%{{data}}%' or EXISTS (
  SELECT 1
  FROM jsonb_array_elements(value->'delta'->'ops') AS op
  WHERE op->>'insert' ilike E'%{{data}}%'
) or exists (select 1 from unnest(keywords) as keyword where keyword ilike E'%{{data}}%')";
#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = sentences)]
pub struct Sentence {
    pub id: i64,
    pub realm_id: i64,
    pub owner_id: i64,
    pub kind: String,
    #[salvo(schema(value_type = Object))]
    pub value: Value,
    pub is_delisted: bool,
    pub sort_rank: i64,
    pub keywords: Vec<String>,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize, Clone, Debug)]
#[diesel(table_name = sentences)]
pub struct NewSentence<'a> {
    pub realm_id: i64,
    pub owner_id: i64,
    pub kind: &'a str,
    pub value: Value,
    pub is_delisted: bool,
    pub sort_rank: i64,
    pub keywords: Vec<String>,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}

pub static ACTION_HISTORY_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["id", "entity", "record_id", "action", "user_id", "old_data", "keywords"]
        .into_iter()
        .map(String::from)
        .collect()
});
pub static ACTION_HISTORY_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
pub static ACTION_HISTORY_SEARCH_TMPL: &str = "record_id::varchar(255)='{{data}}' or entity ilike E'%{{data}}%' or exists (select 1 from unnest(keywords) as keyword where keyword ilike E'%{{data}}%')";
#[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
#[diesel(table_name = action_histories)]
pub struct ActionHistory {
    pub id: i64,
    pub entity: String,
    pub record_id: i64,
    pub action: String,
    pub user_id: i64,
    pub old_data: Option<Value>,
    pub new_data: Option<Value>,
    pub keywords: Vec<String>,
    pub description: Option<String>,

    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize, Clone, Debug)]
#[diesel(table_name = action_histories)]
pub struct NewActionHistory<'a> {
    pub entity: &'a str,
    pub record_id: i64,
    pub action: &'a str,
    pub user_id: i64,
    pub old_data: Option<Value>,
    pub new_data: Option<Value>,
    pub keywords: Vec<String>,
    pub description: Option<&'a str>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ActionRestoreItem {
    pub entity: String,
    pub record: Value,
}
