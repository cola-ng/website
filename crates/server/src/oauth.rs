use std::collections::HashMap;
use std::env;
use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet, RedirectUrl, Scope, TokenUrl};
use salvo::http::StatusError;

use crate::AppResult;
use crate::models::*;
use crate::db::schema::*;

pub struct OauthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub me_url: String,
    pub redirect_url: String,
}
static ALL_PLATFORM_CONFIGS: LazyLock<HashMap<String, OauthConfig>> = LazyLock::new(|| {
    let mut configs = HashMap::new();
    for platform in &["facebook", "linkedin", "google", "google_drive", "dropbox", "box", "shopify"] {
        configs.insert(
            (*platform).to_string(),
            OauthConfig {
                // name: get_env_var(platform, "name").unwrap(),
                client_id: get_env_var(platform, "client_id").unwrap_or_default(),
                client_secret: get_env_var(platform, "client_secret").unwrap_or_default(),
                auth_url: get_env_var(platform, "auth_url").unwrap_or_default(),
                token_url: get_env_var(platform, "token_url").unwrap_or_default(),
                me_url: get_env_var(platform, "me_url").unwrap_or_default(),
                redirect_url: get_env_var(platform, "redirect_url").unwrap_or_default(),
            },
        );
    }
    configs
});
fn get_env_var(platform: &str, field: &str) -> Result<String, env::VarError> {
    let key = format!("{}_{}", platform, field).to_uppercase();
    match env::var(&key) {
        Ok(value) => Ok(value),
        Err(e) => {
            tracing::error!(key = %key, "oauth env not configed");
            Err(e)
        }
    }
}
pub fn new_client(
    platform: &str,
    shop: Option<String>,
) -> AppResult<BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>> {
    let Some(config) = crate::oauth::get_platform_config(platform) else {
        return Err(StatusError::internal_server_error()
            .brief(format!("Platform {} is not supported", platform))
            .into());
    };
    let auth_url = match &shop {
        Some(shop_name) => {
            format!("https://{}{}", shop_name, config.auth_url.clone())
        }
        None => config.auth_url.clone(),
    };
    let token_url = match &shop {
        Some(shop_name) => {
            format!("https://{}{}", shop_name, config.token_url.clone())
        }
        None => config.token_url.clone(),
    };
    let mut client = BasicClient::new(ClientId::new(config.client_id.clone()))
        .set_client_secret(ClientSecret::new(config.client_secret.clone()))
        .set_auth_uri(AuthUrl::new(auth_url)?)
        .set_token_uri(TokenUrl::new(token_url)?)
        .set_redirect_uri(RedirectUrl::new(config.redirect_url.clone())?);
    if platform == "box" || platform == "shopify" {
        client = client.set_auth_type(oauth2::AuthType::RequestBody);
    }
    Ok(client)
}

pub fn get_platform_config(platform: &str) -> Option<&'static OauthConfig> {
    ALL_PLATFORM_CONFIGS.get(platform)
}

pub async fn authorize_url(platform: &str, shop: Option<String>) -> AppResult<String> {
    let client = crate::oauth::new_client(platform, shop)?;
    let (auth_url, _) = if platform == "google" {
        client
            .authorize_url(CsrfToken::new_random)
            // .add_extra_param("access_type", "offline")
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
            // .set_pkce_challenge(pkce_challenge)
            .url()
    } else if platform == "linkedin" {
        client
            .authorize_url(CsrfToken::new_random)
            // .add_extra_param("access_type", "offline")
            .add_scope(Scope::new("r_liteprofile".to_string()))
            .add_scope(Scope::new("r_emailaddress".to_string()))
            // .set_pkce_challenge(pkce_challenge)
            .url()
    } else if platform == "dropbox" {
        client
            .authorize_url(CsrfToken::new_random)
            .add_extra_param("token_access_type", "offline")
            .url()
    } else {
        client
            .authorize_url(CsrfToken::new_random)
            .add_extra_param("access_type", "offline")
            // .set_pkce_challenge(pkce_challenge)
            .url()
    };
    // println!("================================dddd  {}", &auth_url);
    Ok(auth_url.into())
}

#[derive(Deserialize, Debug)]
pub struct OauthMe {
    pub id: String,
    pub name: Option<String>,
    #[serde(alias = "firstName")]
    pub first_name: Option<String>,
    #[serde(alias = "lastName")]
    pub last_name: Option<String>,
    #[serde(alias = "givenName")]
    pub given_name: Option<String>,
    #[serde(alias = "familyName")]
    pub family_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}
impl OauthMe {
    pub fn full_name(&self) -> Option<String> {
        if let Some(name) = &self.name {
            return Some(name.clone());
        }
        let first_part = self.first_name.as_deref().or(self.given_name.as_deref()).unwrap_or_default();
        let last_part = self.last_name.as_deref().or(self.last_name.as_deref()).unwrap_or_default();
        if first_part.is_empty() && last_part.is_empty() {
            None
        } else {
            Some(format!("{} {}", first_part, last_part).trim().to_string())
        }
    }
}
pub async fn get_me(access_token: &str, platform: &str) -> AppResult<OauthMe> {
    match platform {
        "facebook" => {
            let client = reqwest::Client::new();
            let config = crate::oauth::get_platform_config("facebook").unwrap();
            Ok(client
                .get(&config.me_url)
                .query(&[
                    ("fields", "id,name,email"),
                    ("format", "json"),
                    ("method", "get"),
                    ("access_token", access_token),
                ])
                .send()
                .await?
                .json::<OauthMe>()
                .await?)
        }
        "google" => {
            let client = reqwest::Client::new();
            let config = crate::oauth::get_platform_config("google").unwrap();
            // println!(
            //     "me_url:{}\n access_token:{} \nresponse: {}",
            //     &config.me_url,
            //     access_token,
            //     client
            //         .get(&config.me_url)
            //         .header("accept", "application/json")
            //         .bearer_auth(access_token)
            //         .send()
            //         .await?
            //         .text()
            //         .await?
            // );
            Ok(client
                .get(&config.me_url)
                .header("accept", "application/json")
                .bearer_auth(access_token)
                .send()
                .await?
                .json::<OauthMe>()
                .await?)
        }
        "linkedin" => {
            #[derive(Deserialize, Debug)]
            pub struct LinkedinMe {
                pub id: String,
                #[serde(alias = "localizedFirstName")]
                pub first_name: Option<String>,
                #[serde(alias = "localizedLastName")]
                pub last_name: Option<String>,
            }
            let client = reqwest::Client::new();
            let config = crate::oauth::get_platform_config("linkedin").expect("oauth config for linkedin failed");
            println!(
                "me_url:{}\n access_token:{} \nresponse: {}",
                &config.me_url,
                access_token,
                client
                    .get(&config.me_url)
                    .header("accept", "application/json")
                    .bearer_auth(access_token)
                    .send()
                    .await?
                    .text()
                    .await?
            );
            let LinkedinMe { id, first_name, last_name } = client
                .get(&config.me_url)
                .header("accept", "application/json")
                .bearer_auth(access_token)
                .send()
                .await?
                .json::<LinkedinMe>()
                .await?;
            #[derive(Deserialize, Debug)]
            pub struct ResponseData {
                elements: Vec<ElementInfo>,
            }
            #[derive(Deserialize, Debug)]
            pub struct ElementInfo {
                #[serde(alias = "handle~")]
                pub email: EmailInfo,
            }
            #[derive(Deserialize, Debug)]
            pub struct EmailInfo {
                #[serde(alias = "emailAddress")]
                pub address: String,
            }
            let result = client
                .get("https://api.linkedin.com/v2/emailAddress?q=members&projection=(elements*(handle~))")
                .header("accept", "application/json")
                .bearer_auth(access_token)
                .send()
                .await?
                .json::<ResponseData>()
                .await?;
            let email = result.elements[0].email.address.clone();
            Ok(OauthMe {
                id,
                name: None,
                first_name,
                last_name,
                given_name: None,
                family_name: None,
                phone: None,
                email: Some(email),
            })
        }
        _ => Err(crate::AppError::Internal("not support platform".into())),
    }
}
#[allow(clippy::too_many_arguments)]
pub fn upsert_oauth_user(
    user_id: Option<i64>,
    platform: &str,
    me: &OauthMe,
    access_token: &str,
    token_type: &str,
    expires_at: DateTime<Utc>,
    refresh_token: Option<&str>,
    shopify_shop: Option<&str>,
    conn: &mut PgConnection,
) -> Result<OauthUser, diesel::result::Error> {
    let ouser = NewOauthUser {
        user_id,
        platform,
        me_id: &me.id,
        me_full_name: me.name.as_deref(),
        me_email: me.email.as_deref(),
        me_phone: me.phone.as_deref(),
        access_token,
        token_type,
        expires_at,
        refresh_token,
        shopify_shop,
        created_at: Utc::now(),
    };
    diesel::insert_into(oauth_users::table)
        .values(&ouser)
        .on_conflict((oauth_users::me_id, oauth_users::platform))
        .do_update()
        .set(&ouser)
        .get_result(conn)
}
