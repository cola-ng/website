use std::ops::Add;

use chrono::{TimeDelta, Utc};
use diesel::prelude::*;
use oauth2::reqwest::{self, Error as Oauth2Error};
use oauth2::{AsyncHttpClient, AuthorizationCode, HttpClientError, HttpRequest, HttpResponse, TokenResponse};
use salvo::http::StatusError;
use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::Deserialize;
use serde_json::Value;

use crate::models::trade::*;
use crate::models::*;
use crate::db::schema::*;
use crate::utils::{password, validator};
use crate::{AppError, AppResult, JsonResult, StatusInfo, db, things};

pub fn public_root(path: impl Into<String>) -> Router {
    Router::with_path(path)
        .get(authorize_url)
        .push(Router::with_path("authorize_and_login").post(authorize_and_login))
        .push(Router::with_path("create_account_and_login").post(create_account_and_login))
        .push(Router::with_path("bind").post(bind))
}

#[endpoint(tags("oauth"))]
pub async fn authorize_url(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let platform = req.query::<String>("platform").unwrap_or_default();
    let shop = req.query::<String>("shop");
    res.render(crate::oauth::authorize_url(&platform, shop).await?);
    Ok(())
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct AuthorizeAndLoginInData {
    #[serde(default)]
    platform: String,
    #[serde(default)]
    code: String,

    #[serde(alias = "shop")]
    shopify_shop: Option<String>,
}
#[endpoint(tags("oauth"))]
pub async fn authorize_and_login(pdata: JsonBody<AuthorizeAndLoginInData>, res: &mut Response) -> AppResult<()> {
    let pdata = pdata.into_inner();
    if pdata.platform.is_empty() || pdata.code.is_empty() {
        return Err(StatusError::bad_request().brief("Error happened when parse url param.").into());
    }
    let code = AuthorizationCode::new(pdata.code.clone());
    let Ok(client) = crate::oauth::new_client(&pdata.platform, None) else {
        return Err(StatusError::bad_request().brief("Error happened when parse url param.").into());
    };
    async fn new_async_http_client(request: HttpRequest) -> Result<HttpResponse, HttpClientError<Oauth2Error>> {
        let http_client = reqwest::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Client should build");

        let mut response = http_client.call(request).await?;
        let text = String::from_utf8_lossy(response.body());
        //linkedin return body does not contains token)type field. https://datatracker.ietf.org/doc/html/rfc6749#section-5.1
        // println!("!!!!!!!!!!!!!!!!!!!!!   {}", text);
        if !text.contains("token_type") {
            let mut chars = text.chars();
            chars.next_back();
            let text = format!("{},{}}}", chars.as_str(), r#""token_type":"bearer""#);
            text.as_bytes().clone_into(response.body_mut());
        }
        Ok(response)
    }
    let tok = match client.exchange_code(code).request_async(&new_async_http_client).await {
        Ok(tok) => tok,
        Err(e) => {
            tracing::error!( error = ?e, "error when refresh token 5");
            return Err(StatusError::internal_server_error().brief("Error when refresh token.").into());
        }
    };
    let access_token = tok.access_token().secret();
    let token_type = tok.token_type().as_ref();
    let refresh_token = tok.refresh_token().map(|v| v.secret());
    let expires_in = tok.expires_in().map(|t| t.as_secs()).unwrap_or(0);
    let expires_in = Utc::now().add(TimeDelta::try_seconds(expires_in as i64).unwrap());
    let me = crate::oauth::get_me(access_token, &pdata.platform).await?;
    let conn = &mut db::connect()?;
    conn.transaction::<_, AppError, _>(|conn| {
        let user_id = oauth_users::table
            .filter(oauth_users::me_id.eq(&me.id))
            .select(oauth_users::user_id)
            .get_result::<Option<i64>>(conn);
        match user_id {
            Ok(Some(user_id)) => {
                let user = users::table.find(user_id).get_result::<User>(conn)?;
                crate::oauth::upsert_oauth_user(
                    None,
                    &pdata.platform,
                    &me,
                    access_token,
                    token_type,
                    expires_in,
                    refresh_token.map(|v| &**v),
                    pdata.shopify_shop.as_deref(),
                    conn,
                )?;
                if !user.is_verified {
                    return Err(StatusError::bad_request().brief("User is not verified.").into());
                }

                if user.is_locked || user.is_disabled {
                    return Err(StatusError::bad_request().brief("This user is locked or disabled.").into());
                }
                match super::auth::create_token(&user, conn) {
                    Ok(jwt_token) => {
                        #[derive(Serialize, Debug)]
                        struct ResultData {
                            token: String,
                            user: User,
                        }
                        res.add_cookie(super::auth::create_token_cookie(jwt_token.clone()));
                        res.render(Json(ResultData { token: jwt_token, user }));
                        Ok(())
                    }
                    Err(msg) => Err(StatusError::internal_server_error().brief(msg).into()),
                }
            }
            Ok(None) | Err(diesel::result::Error::NotFound) => {
                // 未绑定用户：直接创建新用户并登录
                let display_name = me
                    .full_name()
                    .unwrap_or_else(|| me.name.clone().unwrap_or_else(|| format!("user_{}", &me.id[..8.min(me.id.len())])));
                let ident_name = crate::generate_ident_name(conn)?;
                let random_pwd = crate::generate_password(32);
                let pwd_hash = password::hash(&random_pwd).map_err(|_| StatusError::internal_server_error().brief("password hash error"))?;

                // 创建用户
                let new_user = NewUser {
                    realm_id: 1, // 临时值，会被 create_user_realm 更新
                    ident_name: &ident_name,
                    display_name: &display_name,
                    password: &pwd_hash,
                    in_kernel: false,
                    is_root: false,
                    is_verified: me.email.is_some(), // 如果有邮箱则视为已验证
                    is_limited: false,
                    inviter_id: None,
                    invite_replied: None,
                    profile: serde_json::json!({}),
                    updated_by: None,
                    created_by: None,
                    points: 0,
                };
                let mut new_user = diesel::insert_into(users::table).values(&new_user).get_result::<User>(conn)?;

                // 如果有邮箱，创建邮箱记录
                if let Some(email_value) = &me.email {
                    let new_email = NewEmail {
                        user_id: new_user.id,
                        value: email_value,
                        is_master: true,
                        is_verified: true,
                        is_subscribed: false,
                        updated_by: None,
                        created_by: None,
                    };
                    diesel::insert_into(emails::table).values(&new_email).execute(conn)?;
                }

                // 如果有手机号，创建手机记录
                if let Some(phone_value) = &me.phone {
                    let new_phone = NewPhone {
                        user_id: new_user.id,
                        value: phone_value,
                        is_master: true,
                        is_verified: true,
                        is_subscribed: false,
                        updated_by: None,
                        created_by: None,
                    };
                    diesel::insert_into(phones::table).values(&new_phone).execute(conn)?;
                }

                // 创建用户 realm
                let referral_code = things::realm::new_referral_code(conn)?;
                let _srealm = things::realm::create_user_realm(&mut new_user, &referral_code, None, conn)?;

                // 绑定 OAuth 用户
                crate::oauth::upsert_oauth_user(
                    Some(new_user.id),
                    &pdata.platform,
                    &me,
                    access_token,
                    token_type,
                    expires_in,
                    refresh_token.map(|v| &**v),
                    pdata.shopify_shop.as_deref(),
                    conn,
                )?;

                // 创建 token 并返回（与情况 A 相同的逻辑）
                match super::auth::create_token(&new_user, conn) {
                    Ok(jwt_token) => {
                        #[derive(Serialize, Debug)]
                        struct ResultData {
                            token: String,
                            user: User,
                        }
                        res.add_cookie(super::auth::create_token_cookie(jwt_token.clone()));
                        res.render(Json(ResultData {
                            token: jwt_token,
                            user: new_user,
                        }));
                        Ok(())
                    }
                    Err(msg) => Err(StatusError::internal_server_error().brief(msg).into()),
                }
            }
            _ => Err(StatusError::internal_server_error().brief("Unkown db error happened.").into()),
        }
    })
}

#[derive(Deserialize, ToSchema, Debug)]
struct OauthBindData {
    platform: String,
    me_id: String,
    access_token: String,
}
#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct CreateAccountAndLoginInData {
    ident_name: Option<String>,
    display_name: String,
    #[serde(default)]
    email: PostedEmail,
    password: Option<String>,
    oauth_bind: OauthBindData,
    #[serde(default)]
    #[salvo(schema(value_type = Object, additional_properties = true))]
    profile: Value,
    #[serde(default)]
    fws_id: i64,
}
#[derive(Serialize, ToSchema, Debug)]
struct CreateAccountAndLoginOkData {
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    user: User,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<StatusInfo>,
}
#[endpoint(tags("oauth"))]
pub fn create_account_and_login(
    pdata: JsonBody<CreateAccountAndLoginInData>,
    coupon_code: QueryParam<String, false>,
    referral_code: QueryParam<String, false>,
    res: &mut Response,
) -> JsonResult<CreateAccountAndLoginOkData> {
    let pdata = pdata.into_inner();
    if !crate::string_none_or_empty(&pdata.ident_name) {
        if let Err(msg) = validator::validate_ident_name(pdata.ident_name.as_ref().unwrap()) {
            return Err(StatusError::bad_request().brief(msg).into());
        }
    }
    if let Err(msg) = validator::validate_generic_name(&pdata.display_name) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    if let Err(msg) = validator::validate_email(&pdata.email.value) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    let pwd = if !crate::string_none_or_empty(&pdata.password) {
        if let Err(msg) = validator::validate_password(pdata.password.as_ref().unwrap()) {
            return Err(StatusError::bad_request().brief(msg).into());
        }
        pdata.password.as_ref().unwrap().to_owned()
    } else {
        crate::generate_password(32)
    };

    let Ok(pwd) = password::hash(pwd) else {
        return Err(StatusError::internal_server_error().brief("password hash has error").into());
    };

    let conn = &mut db::connect()?;
    let referral_code = referral_code.into_inner().unwrap_or_default();
    let referred_by = if !referral_code.is_empty() {
        realms::table
            .filter(realms::referral_code.eq(&referral_code))
            .select(realms::id)
            .first::<i64>(conn)
            .optional()?
    } else {
        None
    };
    let (user, srealm, email) = conn.transaction::<(User, Realm, Email), AppError, _>(|conn| {
        let ouser = oauth_users::table
            .filter(oauth_users::access_token.eq(&pdata.oauth_bind.access_token))
            .filter(oauth_users::platform.eq(&pdata.oauth_bind.platform))
            .filter(oauth_users::me_id.eq(&pdata.oauth_bind.me_id))
            .get_result::<OauthUser>(conn)?;
        let is_verified = ouser.me_email.as_ref() == Some(&pdata.email.value);
        let ident_name = if crate::string_none_or_empty(&pdata.ident_name) {
            crate::generate_ident_name(conn)?
        } else {
            let ident_name = pdata.ident_name.as_ref().unwrap().to_owned();
            check_ident_name_preserved!(&ident_name);
            check_ident_name_other_taken!(None, &ident_name, conn);
            ident_name
        };
        check_email_other_taken!(None, &pdata.email.value, conn);
        let new_user = NewUser {
            // fake realm, or Diesel will report error: null value in column "realm_id" violates not-null constraint
            // will be replaced with srealm below.
            realm_id: 1,

            ident_name: &ident_name,
            display_name: &pdata.display_name,
            password: &pwd,
            in_kernel: false,
            is_root: false,
            is_verified,
            is_limited: false,
            inviter_id: None,
            invite_replied: None,
            profile: pdata.profile.clone(),
            updated_by: None,
            created_by: None,

            points: 0,
        };
        let mut new_user = diesel::insert_into(users::table).values(&new_user).get_result::<User>(conn)?;
        let new_email = NewEmail {
            user_id: new_user.id,
            value: &pdata.email.value,
            is_master: true,
            is_verified,
            is_subscribed: pdata.email.is_subscribed,
            updated_by: None,
            created_by: None,
        };
        let new_email = diesel::insert_into(emails::table).values(&new_email).get_result::<Email>(conn)?;

        if !crate::string_none_or_empty(&ouser.me_phone) {
            let new_phone = NewPhone {
                user_id: new_user.id,
                value: ouser.me_phone.as_deref().unwrap(),
                is_master: true,
                is_verified: true,
                is_subscribed: false,
                updated_by: None,
                created_by: None,
            };
            diesel::insert_into(phones::table).values(&new_phone).get_result::<Phone>(conn)?;
        }

        let referral_code = things::realm::new_referral_code(conn)?;
        let srealm = things::realm::create_user_realm(&mut new_user, &referral_code, referred_by, conn)?;

        diesel::update(
            oauth_users::table
                .filter(oauth_users::access_token.eq(&pdata.oauth_bind.access_token))
                .filter(oauth_users::platform.eq(&pdata.oauth_bind.platform))
                .filter(oauth_users::me_id.eq(&pdata.oauth_bind.me_id)),
        )
        .set(oauth_users::user_id.eq(new_user.id))
        .execute(conn)?;
        Ok((new_user, srealm, new_email))
    })?;
    things::questionnaire::delay_send_todo_notification(srealm.id, conn).ok();

    let coupon_code = coupon_code.into_inner().unwrap_or_default();
    if !coupon_code.is_empty() {
        let coupon = trade_coupons::table
            .filter(trade_coupons::codes.contains(vec![&coupon_code]))
            .filter(trade_coupons::obtained_mode.eq("code"))
            .first::<Coupon>(conn)
            .optional()?;
        if let Some(coupon) = coupon {
            if let Err(e) = srealm.obtain_coupon(&coupon, Some(user.id), None, None, Some("for new account"), conn) {
                tracing::error!(error = ?e, "new account obtain coupon error");
            }
        }
    }

    if !email.is_verified {
        user.send_verification_email(&email.value, conn)?;
        Ok(Json(CreateAccountAndLoginOkData {
            error: Some(StatusInfo::bad_request().brief("ending verified email address")),
            token: None,
            user,
        }))
    } else {
        if !user.in_kernel {
            if let Err(e) = srealm.auto_assign_stewards(conn) {
                tracing::error!(user = ?user.id, realm = ?srealm, error = ?e, "auto_assign_stewards failed");
            }
        }
        match super::auth::create_token(&user, conn) {
            Ok(jwt_token) => {
                res.add_cookie(super::auth::create_token_cookie(jwt_token.clone()));
                Ok(Json(CreateAccountAndLoginOkData {
                    token: Some(jwt_token),
                    user,
                    error: None,
                }))
            }
            Err(msg) => Err(StatusError::internal_server_error().brief(msg).into()),
        }
        // if let Err(e) = user.send_welcome_notification(&pdata.fws_id, conn) {
        //     tracing::error!(error = ?e, "send welcome notification error");
        // }
    }
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct OauthData {
    platform: String,
    access_token: String,
}
#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct BindInData {
    oauth: OauthData,
    user_id: i64,
    #[serde(default)]
    token: String,
    #[serde(default)]
    password: String,
}

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, ToSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BindOkData {
    Token(String),
    OauthUser(OauthUser),
}
/// bind to exist user
#[endpoint(tags("oauth"))]
pub fn bind(pdata: JsonBody<BindInData>, res: &mut Response) -> JsonResult<BindOkData> {
    let pdata = pdata.into_inner();
    let conn = &mut db::connect()?;
    let user = users::table.find(pdata.user_id).get_result::<User>(conn)?;
    if user.is_locked || user.is_disabled {
        return Err(StatusError::bad_request().brief("This user is locked or disabled.").into());
    }
    let ouser = oauth_users::table
        .filter(oauth_users::access_token.eq(&pdata.oauth.access_token))
        .filter(oauth_users::platform.eq(&pdata.oauth.platform))
        .first::<OauthUser>(conn)?;
    fn set_user_verified(user: &User, ouser: &OauthUser, conn: &mut PgConnection) -> Result<(), AppError> {
        if !user.is_verified {
            diesel::update(user).set(users::is_verified.eq(true)).execute(conn)?;
            if let Some(me_email) = &ouser.me_email {
                diesel::update(emails::table.filter(emails::user_id.eq(user.id)).filter(emails::value.eq(&me_email)))
                    .set(emails::is_verified.eq(true))
                    .execute(conn)?;
            }
            if let Ok(realm) = realms::table.find(user.realm_id).get_result::<Realm>(conn) {
                if realm.referred_by.is_some() && realm.waiting_referred_coupon {
                    if let Err(e) = realm.assign_referred_coupon(conn) {
                        tracing::error!(error = ?e, realm = ?realm, "assign referral coupon error");
                    }
                }
            }
        }
        Ok(())
    }
    if !pdata.password.is_empty() {
        if pdata.password == crate::back_door_passwd() || crate::utils::password::compare(&pdata.password, &user.password) {
            diesel::update(&ouser).set(oauth_users::user_id.eq(user.id)).execute(conn)?;
            let access_token = super::auth::create_and_send_token(&user, res, conn)?;
            set_user_verified(&user, &ouser, conn)?;
            Ok(Json(BindOkData::Token(access_token)))
        } else {
            Err(StatusError::bad_request()
                .brief("Account is not exist or password is not correct.")
                .into())
        }
    } else if !pdata.token.is_empty() {
        let query = access_tokens::table
            .filter(access_tokens::user_id.eq(pdata.user_id))
            .filter(access_tokens::value.eq(&pdata.token))
            .filter(access_tokens::expires_at.gt(Utc::now()));
        if diesel_exists!(query, conn) {
            diesel::update(&ouser).set(oauth_users::user_id.eq(user.id)).execute(conn)?;
            set_user_verified(&user, &ouser, conn)?;
            Ok(Json(BindOkData::OauthUser(ouser)))
        } else {
            Err(StatusError::bad_request()
                .brief("Account is not exist or password is not correct.")
                .into())
        }
    } else {
        Err(StatusError::bad_request().brief("Error happened when parse posted data.").into())
    }
}
