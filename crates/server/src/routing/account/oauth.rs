use chrono::{TimeDelta, Utc};
use diesel::prelude::*;
use oauth2::{AuthorizationCode, TokenResponse, reqwest};

use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::Deserialize;
use std::ops::Add;

use crate::models::*;
use crate::db::schema::*;
use crate::{AppResult, DepotExt, JsonResult, PagedResult, StatusInfo, db};

#[endpoint(tags("account"))]
pub async fn list(req: &mut Request, depot: &mut Depot) -> PagedResult<OauthUser> {
    let conn = &mut db::conn()?;
    let cuser = depot.current_user()?;
    let query = oauth_users::table.filter(oauth_users::user_id.eq(cuser.id));
    let data = query_pagation_data!(
        req,
        res,
        OauthUser,
        query,
        "created_at desc",
        OAUTH_USER_FILTER_FIELDS.clone(),
        OAUTH_USERS_JOINED_OPTIONS.clone(),
        "",
        conn
    );
    Ok(Json(data))
}

#[endpoint(tags("account"))]
pub async fn remove(oauth_user_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let conn = &mut db::conn()?;
    let cuser = depot.current_user()?;
    diesel::delete(
        oauth_users::table
            .filter(oauth_users::user_id.eq(cuser.id))
            .filter(oauth_users::id.eq(oauth_user_id.into_inner())),
    )
    .execute(conn)?;
    Ok(StatusInfo::done())
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct AuthorizeAndBindInData {
    #[serde(default)]
    platform: String,
    #[serde(default)]
    code: String,

    #[serde(alias = "shop")]
    shopify_shop: Option<String>,
}
#[endpoint(tags("account"))]
pub async fn authorize_and_bind(pdata: JsonBody<AuthorizeAndBindInData>, depot: &mut Depot) -> JsonResult<OauthUser> {
    let pdata = pdata.into_inner();
    if pdata.platform.is_empty() || pdata.code.is_empty() {
        return Err(StatusError::not_found().into());
    }
    let cuser = depot.current_user()?;
    let mut conn = db::conn()?;
    let ouser = oauth_users::table
        .filter(oauth_users::user_id.eq(cuser.id))
        .filter(oauth_users::platform.eq(&pdata.platform))
        .get_result::<OauthUser>(&mut conn);
    drop(conn);

    if let Ok(ouser) = ouser {
        return Ok(Json(ouser));
    }

    let code = AuthorizationCode::new(pdata.code.clone());
    let client = match crate::oauth::new_client(&pdata.platform, None) {
        Err(e) => {
            tracing::error!( error = ?e, "error when refresh token 3");
            return Err(StatusError::internal_server_error().brief("Error when refresh token.").into());
        }
        Ok(client) => client,
    };
    let http_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");
    let tok = match client.exchange_code(code).request_async(&http_client).await {
        Err(e) => {
            tracing::error!( error = ?e, "error when refresh token 4");
            return Err(StatusError::internal_server_error().brief("Error when refresh token.").into());
        }
        Ok(tok) => tok,
    };
    let access_token = tok.access_token().secret();
    let token_type = tok.token_type().as_ref();
    let refresh_token = tok.refresh_token().map(|v| v.secret());
    let expires_in = tok.expires_in().map(|t| t.as_secs()).unwrap_or(0);
    let expires_in = Utc::now().add(TimeDelta::try_seconds(expires_in as i64).expect("time delta should be valid"));
    let me = crate::oauth::get_me(access_token, &pdata.platform).await?;
    let ouser = crate::oauth::upsert_oauth_user(
        Some(cuser.id),
        &pdata.platform,
        &me,
        access_token,
        token_type,
        expires_in,
        refresh_token.map(|v| &**v),
        pdata.shopify_shop.as_deref(),
        &mut *db::conn()?,
    )?;
    Ok(Json(ouser))
}
