use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use salvo::http::StatusError;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AppResult;
use crate::db::schema::*;
// use crate::db::url_filter::JoinedOption;

// pub static USER_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
//     vec![
//         "id",
//         "ident_name",
//         "display_name",
//         "in_kernel",
//         "is_limited",
//         "inviter_id",
//         "updated_by",
//         "created_by",
//     ]
//     .into_iter()
//     .map(String::from)
//     .collect()
// });
// pub static USER_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(|| {
//     url_filter_joined_options![
//         "emails", "id"=>"user_id", "e.value"=>"value";
//         "phones", "id"=>"user_id", "p.value"=>"value";
//     ]
// });
// pub static USER_SEARCH_TMPL: &str = "id::varchar(255)='{{data}}' or ident_name ilike E'%{{data}}%' or display_name ilike E'%{{data}}%' or id in (select user_id from emails where emails.value ilike E'%{{data}}%') or id in (select user_id from phones where phones.value ilike E'%{{data}}%')";
#[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct User {
    pub id: i64,

    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,

    pub avatar: Option<String>,
    pub display_name: Option<String>,

    pub verified_at: Option<DateTime<Utc>>,

    pub limited_at: Option<DateTime<Utc>>,
    pub limited_by: Option<i64>,

    pub locked_at: Option<DateTime<Utc>>,
    pub locked_by: Option<i64>,

    pub disabled_at: Option<DateTime<Utc>>,
    pub disabled_by: Option<i64>,

    pub inviter_id: Option<i64>,
    #[salvo(schema(value_type = Object, additional_properties = true))]
    pub profile: Value,

    pub updated_by: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Deserialize, Clone, Debug)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub display_name: Option<String>,
    pub inviter_id: Option<i64>,
    pub profile: Value,

    pub updated_by: Option<i64>,
    pub created_by: Option<i64>,
}

#[derive(Identifiable, Debug, Clone)]
#[diesel(table_name = user_passwords)]
pub struct Password {
    pub id: i64,
    pub user_id: i64,
    pub hash: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Insertable, Queryable, Debug, Clone)]
#[diesel(table_name = user_passwords)]
pub struct NewPassword {
    pub user_id: i64,
    pub hash: String,
    pub created_at: DateTime<Utc>,
}

// pub static ROLE_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
//     vec!["id", "name", "description", "owner_id", "updated_by", "created_by"]
//         .into_iter()
//         .map(String::from)
//         .collect()
// });
// pub static ROLE_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
// #[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
// pub struct Role {
//     pub id: i64,
//     pub code: Option<String>,
//     pub name: String,
//     pub kind: String,
//     pub owner_id: i64,
//     pub description: Option<String>,
//     pub updated_by: Option<i64>,
//     pub updated_at: DateTime<Utc>,
//     pub created_by: Option<i64>,
//     pub created_at: DateTime<Utc>,
// }
// #[derive(Insertable, Debug)]
// #[diesel(table_name = roles)]
// pub struct NewRole<'a> {
//     pub code: Option<&'a str>,
//     pub name: &'a str,
//     pub kind: Option<&'a str>,
//     pub owner_id: i64,
//     pub description: Option<&'a str>,

//     pub updated_by: Option<i64>,
//     pub created_by: Option<i64>,
// }

// #[derive(Queryable, Insertable, Serialize, Debug)]
// pub struct RoleUser {
//     pub role_id: i64,
//     pub user_id: i64,
// }

// #[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
// pub struct Operation {
//     pub id: i64,
//     pub code: String,
//     pub title: String,
//     pub entity: String,
//     pub action: String,
//     pub scope: String,
//     pub filter_name: Option<String>,
//     pub filter_value: Option<String>,
//     #[salvo(schema(value_type = Option<Object>))]
//     pub depends: Option<Value>,
//     #[salvo(schema(value_type = Option<Object>))]
//     pub contains: Option<Value>,
//     pub sort_rank: i64,
//     pub description: Option<String>,
// }

// #[derive(Identifiable, Insertable, Queryable, QueryableByName, Serialize, ToSchema, Clone, Debug)]
// #[diesel(table_name = permissions)]
// pub struct Permission {
//     pub id: i64,
//     pub role_id: i64,
//     pub entity: String,
//     pub action: String,
//     pub scope: String,
//     pub filter_name: String,
//     pub filter_int_value: Option<i64>,
//     pub filter_text_value: Option<String>,
// }
// #[derive(Insertable, Debug)]
// #[diesel(table_name = permissions)]
// pub struct NewPermission<'a> {
//     pub role_id: i64,
//     pub entity: &'a str,
//     pub action: &'a str,
//     pub scope: &'a str,
//     pub filter_name: &'a str,
//     pub filter_int_value: Option<i64>,
//     pub filter_text_value: Option<&'a str>,
// }

// pub static EMAIL_SUBSCRIPTION_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
//     vec!["id", "user_id", "topic", "updated_by", "created_by"]
//         .into_iter()
//         .map(String::from)
//         .collect()
// });
// pub static EMAIL_SUBSCRIPTION_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
// /// 如果对应的记录不存在，认为是默认接收提醒邮件的。
// /// - trade_order_created
// /// - stock_exhibit_develop_approved(应该不需要，现在只发内部管理员)
// /// - interflow_thread_created
// /// - interflow_thread_recalled
// /// - interflow_thread_updated
// #[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
// pub struct EmailSubscription {
//     pub id: i64,
//     pub user_id: i64,
//     pub topic: String,
//     pub value: bool,

//     pub updated_by: Option<i64>,
//     pub updated_at: DateTime<Utc>,
//     pub created_by: Option<i64>,
//     pub created_at: DateTime<Utc>,
// }
// #[derive(Insertable, Serialize, Clone, Debug)]
// #[diesel(table_name = email_subscriptions)]
// pub struct NewEmailSubscription<'a> {
//     pub user_id: i64,
//     pub topic: &'a str,
//     pub value: bool,

//     pub updated_by: Option<i64>,
//     pub created_by: Option<i64>,
// }

// pub static PHONE_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
//     vec![
//         "id",
//         "user_id",
//         "value",
//         "is_verified",
//         "is_master",
//         "is_subscribed",
//         "updated_by",
//         "created_by",
//     ]
//     .into_iter()
//     .map(String::from)
//     .collect()
// });
// pub static PHONE_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
// #[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
// pub struct Phone {
//     pub id: i64,
//     pub user_id: i64,
//     pub value: String,
//     pub is_verified: bool,
//     pub is_master: bool,
//     pub is_subscribed: bool,

//     pub updated_by: Option<i64>,
//     pub updated_at: DateTime<Utc>,
//     pub created_by: Option<i64>,
//     pub created_at: DateTime<Utc>,
// }
// #[derive(Insertable, Serialize, Clone, Debug)]
// #[diesel(table_name = phones)]
// pub struct NewPhone<'a> {
//     pub user_id: i64,
//     pub value: &'a str,
//     pub is_verified: bool,
//     pub is_subscribed: bool,
//     pub is_master: bool,

//     pub updated_by: Option<i64>,
//     pub created_by: Option<i64>,
// }
// #[derive(Deserialize, ToSchema, Debug)]
// pub struct PostedPhone {
//     #[serde(default)]
//     pub value: String,
//     #[serde(default = "crate::default_as_false")]
//     pub is_subscribed: bool,
// }
// impl Default for PostedPhone {
//     fn default() -> Self {
//         PostedPhone {
//             value: "".to_owned(),
//             is_subscribed: false,
//         }
//     }
// }

// pub static ADDRESS_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
//     vec!["id", "country", "state", "city", "line1", "line2", "updated_by", "created_by"]
//         .into_iter()
//         .map(String::from)
//         .collect()
// });
// pub static ADDRESS_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
// #[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
// #[diesel(table_name = addresses)]
// pub struct Address {
//     pub id: i64,
//     pub country: String,
//     pub state: Option<String>,
//     pub city: Option<String>,
//     pub post_code: Option<String>,
//     pub line1: Option<String>,
//     pub line2: Option<String>,

//     pub updated_by: Option<i64>,
//     pub updated_at: DateTime<Utc>,
//     pub created_by: Option<i64>,
//     pub created_at: DateTime<Utc>,
// }
// #[derive(Insertable, Serialize, Clone, Debug)]
// #[diesel(table_name = addresses)]
// pub struct NewAddress<'a> {
//     pub country: &'a str,
//     pub state: Option<&'a str>,
//     pub city: Option<&'a str>,
//     pub post_code: Option<&'a str>,
//     pub line1: Option<&'a str>,
//     pub line2: Option<&'a str>,

//     pub updated_by: Option<i64>,
//     pub created_by: Option<i64>,
// }

pub static ACCESS_TOKEN_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        "id",
        "user_id",
        "name",
        "kind",
        "value",
        "updated_by",
        "created_by",
    ]
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

// #[derive(Identifiable, Insertable, Queryable, Serialize, Deserialize, ToSchema, Clone, Debug)]
// pub struct SecurityCode {
//     pub id: i64,
//     pub user_id: i64,
//     pub email: Option<String>,
//     pub phone: Option<String>,
//     pub value: String,
//     pub send_method: String,
//     pub consumed_at: Option<DateTime<Utc>>,
//     pub expires_at: DateTime<Utc>,

//     pub updated_by: Option<i64>,
//     pub updated_at: DateTime<Utc>,
//     pub created_by: Option<i64>,
//     pub created_at: DateTime<Utc>,
// }

// #[derive(Insertable, Debug)]
// #[diesel(table_name = security_codes)]
// pub struct NewSecurityCode<'a> {
//     pub user_id: i64,
//     pub email: Option<&'a str>,
//     pub phone: Option<&'a str>,
//     pub value: &'a str,
//     pub send_method: &'a str,
//     pub expires_at: DateTime<Utc>,

//     pub updated_by: Option<i64>,
//     pub created_by: Option<i64>,
// }
// pub static OAUTH_ACCESS_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
//     vec!["id", "owner_id", "platform", "token_type", "updated_by", "created_by"]
//         .into_iter()
//         .map(String::from)
//         .collect()
// });
// pub static OAUTH_ACCESS_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);

// pub static NOTIFICATION_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
//     vec!["id", "owner_id", "sender_id", "kind", "is_read", "read_at", "sent_at"]
//         .into_iter()
//         .map(String::from)
//         .collect()
// });
// pub static NOTIFICATION_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
// pub static NOTIFICATION_SEARCH_TMPL: &str = r#"id::varchar(255)='{{data}}' or subject ilike E'%{{data}}%' or body ilike E'%{{data}}%'"#;
// #[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
// #[diesel(table_name = notifications)]
// pub struct Notification {
//     pub id: i64,
//     pub owner_id: i64,
//     pub sender_id: i64,
//     pub subject: String,
//     pub body: String,
//     pub kind: String,
//     pub is_read: bool,
//     pub stream_id: Option<i64>,
//     pub thread_id: Option<i64>,
//     pub entity: Option<String>,
//     pub record_id: Option<i64>,
//     #[salvo(schema(value_type = Option<Object>))]
//     pub extra: Option<Value>,

//     pub read_at: Option<DateTime<Utc>>,
//     pub sent_at: DateTime<Utc>,
// }

// #[derive(Insertable, Deserialize, Clone, Debug)]
// #[diesel(table_name = notifications)]
// pub struct NewNotification<'a> {
//     pub owner_id: i64,
//     pub sender_id: i64,
//     pub subject: &'a str,
//     pub body: &'a str,
//     pub kind: &'a str,
//     pub stream_id: Option<i64>,
//     pub thread_id: Option<i64>,
//     pub entity: Option<&'a str>,
//     pub record_id: Option<i64>,
//     pub extra: Option<Value>,
// }
// owner_entity_accessible!(Notification);

// pub static EMAIL_MESSAGE_FILTER_FIELDS: LazyLock<Vec<String>> = LazyLock::new(|| {
//     vec!["id", "kind", "recipients", "subject", "text_body", "html_body", "status"]
//         .into_iter()
//         .map(String::from)
//         .collect()
// });
// pub static EMAIL_MESSAGE_JOINED_OPTIONS: LazyLock<Vec<JoinedOption>> = LazyLock::new(Vec::new);
// pub static EMAIL_MESSAGE_SEARCH_TMPL: &str = "id::varchar(255)='{{data}}' or subject ilike E'%{{data}}%' or recipient_email ilike E'%{{data}}%'";
// #[derive(Identifiable, Insertable, Queryable, Serialize, ToSchema, Clone, Debug)]
// #[diesel(table_name = email_messages)]
// pub struct EmailMessage {
//     pub id: i64,
//     pub kind: String,
//     pub thread_id: Option<i64>,
//     pub recipient_id: Option<i64>,
//     pub recipient_email: String,
//     // #[serde(skip)]
//     pub reply_token: Option<String>,
//     // #[serde(skip)]
//     pub view_token: String,
//     pub subject: String,
//     pub text_body: Option<String>,
//     pub html_body: Option<String>,
//     #[salvo(schema(value_type = Option<Object>))]
//     pub attachments: Value,
//     pub track_flag: Option<String>,
//     pub sent_in: Option<DateTime<Utc>>,
//     pub sent_status: String, // "delay", "sending", "sent", "failed"
//     pub sent_error: Option<String>,
//     pub is_read: bool,
//     pub is_replied: bool,

//     pub updated_at: DateTime<Utc>,
//     pub created_at: DateTime<Utc>,
// }

// #[derive(Insertable, Deserialize, Clone, Debug)]
// #[diesel(table_name = email_messages)]
// pub struct NewEmailMessage<'a> {
//     pub kind: &'a str,
//     pub thread_id: Option<i64>,
//     pub recipient_id: Option<i64>,
//     pub recipient_email: &'a str,
//     pub reply_token: Option<&'a str>,
//     pub view_token: &'a str,
//     pub subject: &'a str,
//     pub text_body: Option<String>,
//     pub html_body: Option<String>,
//     pub attachments: Value,
//     pub track_flag: Option<&'a str>,
//     pub sent_in: Option<DateTime<Utc>>,
//     pub sent_status: &'a str,
//     pub sent_error: Option<&'a str>,
// }
// #[derive(Serialize, Deserialize, Clone, Debug)]
// pub struct EmailAttachment {
//     pub path: String,
//     // pub key: String,
//     // pub hash: String,
// }

#[derive(QueryableByName, Debug)]
pub struct TableId {
    #[diesel(sql_type = ::diesel::sql_types::BigInt)]
    #[diesel(column_name = id)]
    pub id: i64,
}
