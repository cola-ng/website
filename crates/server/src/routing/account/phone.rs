use chrono::Utc;
use std::result::Result;
use horme::prelude::*;

use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db;
use crate::models::*;
use crate::db::schema::*;
use crate::utils::validator;
use crate::{AppError, AppResult, DepotExt, JsonResult, PagedResult, StatusInfo};

#[endpoint(tags("account"))]
pub async fn list(req: &mut Request, depot: &mut Depot) -> PagedResult<Phone> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn().await?;
    let query = phones::table.filter(phones::user_id.eq(cuser.id));
    let data = query_pagation_data!(
        req,
        res,
        Phone,
        query,
        "created_at desc",
        PHONE_FILTER_FIELDS.clone(),
        PHONE_JOINED_OPTIONS.clone(),
        ID_VALUE_SEARCH_TMPL,
        conn
    );
    Ok(Json(data))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct CreateInData {
    #[serde(default)]
    value: String,
}
#[endpoint(tags("account"))]
pub async fn create(pdata: JsonBody<CreateInData>, depot: &mut Depot) -> JsonResult<Phone> {
    let pdata = pdata.into_inner();
    if let Err(msg) = validator::validate_email(&pdata.value) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    let cuser = depot.current_user()?;
    let conn = &mut db::conn().await?;
    let phone = horme::transaction(&mut conn, async |conn| {
        // check_phone_other_taken!(Some(cuser.id), &pdata.value, conn);
        let new_phone = NewPhone {
            user_id: cuser.id,
            value: &pdata.value,
            is_verified: false,
            is_subscribed: false,
            is_master: false, //todo
            updated_by: Some(cuser.id),
            created_by: Some(cuser.id),
        };
        let phone = diesel::insert_into(phones::table).values(&new_phone).get_result::<Phone>(conn).await?;
        Ok(phone)
    }).await?;

    cuser.send_security_code_phone_sms(&phone.value, conn)?;
    Ok(Json(phone))
}
#[endpoint(tags("account"))]
pub async fn set_master(phone_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn().await?;
    let phone = phones::table.find(phone_id.into_inner()).first::<Phone>(conn).await?;
    if phone.user_id != cuser.id {
        return Err(StatusError::forbidden().into());
    }
    if phone.is_master {
        return Ok(StatusInfo::done());
    }
    horme::transaction(&mut conn, async |conn| {
        diesel::update(
            phones::table
                .filter(phones::user_id.eq(cuser.id))
                .filter(phones::id.ne(phone.id))
                .filter(phones::is_master.eq(true)),
        )
        .set((
            phones::is_master.eq(false),
            phones::updated_by.eq(cuser.id),
            phones::updated_at.eq(Utc::now()),
        ))
        .execute(conn).await?;
        diesel::update(&phone)
            .set((
                phones::is_master.eq(true),
                phones::updated_by.eq(cuser.id),
                phones::updated_at.eq(Utc::now()),
            ))
            .execute(conn).await?;
        Ok(())
    }).await?;
    Ok(StatusInfo::done())
}
#[endpoint(tags("account"))]
pub async fn resend_verification(phone_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn().await?;
    let phone = phones::table.find(phone_id.into_inner()).first::<Phone>(conn).await?;
    let user = users::table.find(phone.user_id).first::<User>(conn).await?;
    if phone.user_id != cuser.id {
        return Err(StatusError::forbidden().into());
    }
    if phone.is_verified {
        return Ok(StatusInfo::done());
    }

    user.send_security_code_phone_sms(&phone.value, conn)?;
    Ok(StatusInfo::done().brief("Security code sent."))
}
#[endpoint(tags("account"))]
pub async fn delete(phone_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<Vec<Phone>> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn().await?;
    let phone = phones::table.find(phone_id.into_inner()).first::<Phone>(conn).await?;
    if phone.user_id != cuser.id {
        return Err(StatusError::bad_request().into());
    }
    if phone.is_master {
        return Err(StatusError::bad_request().brief("master phone can not be deleted").into());
    }
    diesel::delete(&phone).execute(conn).await?;
    let phones = phones::table.filter(phones::user_id.eq(cuser.id)).get_results::<Phone>(conn).await?;
    Ok(Json(phones))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct UpdateInData {
    #[serde(default)]
    value: String,
}
#[endpoint(tags("account"))]
pub async fn update(phone_id: PathParam<i64>, pdata: JsonBody<UpdateInData>, depot: &mut Depot) -> JsonResult<Phone> {
    let pdata = pdata.into_inner();
    if let Err(msg) = validator::validate_phone(&pdata.value) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    let cuser = depot.current_user()?;
    let conn = &mut db::conn().await?;
    let phone = phones::table.find(phone_id.into_inner()).first::<Phone>(conn).await?;
    let phone = horme::transaction(&mut conn, async |conn| {
        // let exist_phone = phones::table.filter(phones::value.eq(&pdata.value))
        //     .filter(phones::id.ne(phone_id)).first::<Phone>(conn).await.ok();
        // if let Some(exist_phone) = exist_phone {
        //     if exist_phone.user_id == cuser.id {
        //         return Err(AppError::HttpStatus(Box::new(Conflict("phone conflict", "this phone exist in your account"))));
        //     } else {
        //         return Err(AppError::HttpStatus(Box::new(Conflict("phone conflict", "this phone exist in other account"))));
        //     }
        // }
        let phone = diesel::update(phones::table.filter(phones::id.eq(phone.id)).filter(phones::user_id.eq(cuser.id)))
            .set((
                phones::is_verified.eq(false),
                phones::value.eq(&pdata.value),
                phones::updated_by.eq(cuser.id),
                phones::updated_at.eq(Utc::now()),
            ))
            .get_result::<Phone>(conn).await?;
        Ok(phone)
    }).await?;
    Ok(Json(phone))
}
