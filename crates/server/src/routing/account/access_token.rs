use chrono::{TimeDelta, Utc};
use diesel::prelude::*;
use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

use crate::db;
use crate::models::*;
use crate::permission::Accessible;
use crate::db::schema::*;
use crate::utils::validator;
use crate::{AppError, AppResult, DepotExt, ErrorItem, JsonResult, StatusInfo};

#[endpoint(tags("account"))]
pub fn delete(token_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;

    let access_token = access_tokens::table.find(token_id.into_inner()).first::<AccessToken>(conn)?;
    users::table
        .find(access_token.user_id)
        .first::<User>(conn)?
        .assign_to(cuser, "edit", conn)?;
    db::delete_access_token(access_token.id, cuser.id, conn)?;
    Ok(StatusInfo::done())
}
#[endpoint(tags("account"))]
pub async fn bulk_delete(req: &mut Request, depot: &mut Depot) -> AppResult<StatusInfo> {
    let conn = &mut db::conn()?;
    let info = bulk_delete_records!(
        req,
        depot,
        res,
        access_tokens,
        AccessToken,
        db::delete_access_token,
        users,
        User,
        user_id,
        "edit",
        conn
    );

    Ok(info)
}

#[endpoint(tags("account"))]
pub fn list(_req: &mut Request, depot: &mut Depot) -> JsonResult<Vec<AccessToken>> {
    let cuser = depot.current_user()?;
    let query = access_tokens::table
        .filter(access_tokens::user_id.eq(cuser.id))
        .filter(access_tokens::kind.eq("api"));
    let conn = &mut db::conn()?;
    Ok(Json(query.get_results::<AccessToken>(conn)?))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct CreateInData {
    user_id: i64,
    #[serde(default)]
    name: String,
    // #[serde(default)]
    // pub kind: String,
    #[serde(default)]
    value: String,
    device: Option<String>,
}

#[derive(Serialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct CreateOkData {
    value: String,
}
#[endpoint(tags("account"))]
pub fn create(pdata: JsonBody<CreateInData>, depot: &mut Depot) -> JsonResult<CreateOkData> {
    let pdata = pdata.into_inner();
    if pdata.name.is_empty() {
        return Err(StatusError::bad_request().brief("name is not provider").into());
    }
    if let Err(e) = validator::validate_generic_name(&pdata.name) {
        return Err(StatusError::bad_request().brief(e).into());
    }
    let cuser = depot.current_user()?;
    let exp = Utc::now() + TimeDelta::try_days(7).unwrap();
    let Ok(jwt_token) = crate::create_jwt_token(cuser, &exp) else {
        return Err(StatusError::internal_server_error().brief("create jwt token error").into());
    };
    let conn = &mut db::conn()?;
    let output = conn.transaction::<_, AppError, _>(|conn| {
        let query = access_tokens::table
            .filter(access_tokens::user_id.eq(cuser.id))
            .filter(access_tokens::name.eq(&pdata.name));
        if diesel_exists!(query, conn) {
            return Err(StatusError::conflict()
                .brief("token conflict")
                .brief("This name is already taken, please try another.")
                .into());
        }
        let token = NewAccessToken {
            user_id: cuser.id,
            name: Some(&pdata.name),
            value: jwt_token.split('.').collect::<Vec<&str>>()[2],
            kind: "api",
            device: None,
            expires_at: exp,
            updated_by: Some(cuser.id),
            created_by: Some(cuser.id),
        };
        diesel::insert_into(access_tokens::table).values(&token).execute(conn)?;
        Ok(CreateOkData {
            value: token.value.to_owned(),
        })
    })?;
    Ok(Json(output))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct UpdateInData {
    user_id: i64,
    #[serde(default)]
    name: String,
    // #[serde(default)]
    // pub kind: String,
    #[serde(default)]
    value: String,
    device: Option<String>,
}
#[endpoint(tags("account"))]
pub fn update(token_id: PathParam<i64>, pdata: JsonBody<UpdateInData>, depot: &mut Depot) -> JsonResult<AccessToken> {
    let pdata = pdata.into_inner();
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let exist_token = access_tokens::table.find(token_id.into_inner()).first::<AccessToken>(conn)?;
    if exist_token.user_id != cuser.id {
        return Err(StatusError::bad_request().brief("access token is not correct").into());
    }
    if pdata.name.is_empty() {
        return Err(StatusError::bad_request().brief("access token's name is not provide").into());
    }
    let token = conn.transaction::<AccessToken, AppError, _>(|conn| {
        let query = access_tokens::table
            .filter(access_tokens::user_id.eq(cuser.id))
            .filter(access_tokens::id.ne(exist_token.id))
            .filter(access_tokens::name.eq(&pdata.name));
        if diesel_exists!(query, conn) {
            return Err(StatusError::conflict()
                .brief("token conflict")
                .brief("This name is already taken, please try another.")
                .into());
        }
        let token = diesel::update(&exist_token)
            .set((
                access_tokens::name.eq(&pdata.name),
                access_tokens::updated_by.eq(cuser.id),
                access_tokens::updated_at.eq(Utc::now()),
            ))
            .get_result::<AccessToken>(conn)?;
        Ok(token)
    })?;
    Ok(Json(token))
}
