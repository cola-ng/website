use chrono::Utc;
use diesel::prelude::*;

use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::{self, lower};
use crate::models::*;
use crate::db::schema::*;
use crate::utils::validator;
use crate::{AppError, AppResult, DepotExt, JsonResult, PagedResult, StatusInfo};

#[endpoint(tags("account"))]
pub fn list(req: &mut Request, depot: &mut Depot) -> PagedResult<Email> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let query = emails::table.filter(emails::user_id.eq(cuser.id));
    let data = query_pagation_data!(
        req,
        res,
        Email,
        query,
        "created_at desc",
        EMAIL_FILTER_FIELDS.clone(),
        EMAIL_JOINED_OPTIONS.clone(),
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
pub fn create(pdata: JsonBody<CreateInData>, depot: &mut Depot) -> JsonResult<Email> {
    let pdata = pdata.into_inner();
    let value = pdata.value.to_lowercase();
    if let Err(msg) = validator::validate_email(&pdata.value) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let email = conn.transaction::<Email, AppError, _>(|conn| {
        check_email_other_taken!(None, &value, conn);
        let new_email = NewEmail {
            user_id: cuser.id,
            value: &value,
            is_verified: false,
            is_subscribed: false,
            is_master: false,
            updated_by: Some(cuser.id),
            created_by: Some(cuser.id),
        };
        let email = diesel::insert_into(emails::table).values(&new_email).get_result::<Email>(conn)?;
        Ok(email)
    })?;

    cuser.send_verification_email(&email.value, conn)?;
    Ok(Json(email))
}
#[endpoint(tags("account"))]
pub fn set_master(email_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let email = emails::table.find(email_id.into_inner()).first::<Email>(conn)?;
    if email.user_id != cuser.id {
        return Err(StatusError::forbidden().into());
    }
    if email.is_master {
        return Ok(StatusInfo::done());
    }
    conn.transaction::<_, AppError, _>(|conn| {
        diesel::update(
            emails::table
                .filter(emails::user_id.eq(cuser.id))
                .filter(emails::id.ne(email.id))
                .filter(emails::is_master.eq(true)),
        )
        .set((
            emails::is_master.eq(false),
            emails::updated_by.eq(cuser.id),
            emails::updated_at.eq(Utc::now()),
        ))
        .execute(conn)?;
        diesel::update(&email)
            .set((
                emails::is_master.eq(true),
                emails::updated_by.eq(cuser.id),
                emails::updated_at.eq(Utc::now()),
            ))
            .execute(conn)?;
        Ok(())
    })?;
    Ok(StatusInfo::done())
}

#[endpoint(tags("account"))]
pub fn resend_verification(email_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let email = emails::table.find(email_id.into_inner()).first::<Email>(conn)?;
    let user = users::table.find(email.user_id).first::<User>(conn)?;

    if email.user_id != cuser.id && !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }
    if email.is_verified {
        return Err(StatusError::not_found().brief("email is verified already").into());
    }

    user.send_verification_email(&email.value, conn)?;
    Ok(StatusInfo::done().brief("security code sent"))
}

#[endpoint(tags("account"))]
pub fn delete(email_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<Vec<Email>> {
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let email = emails::table.find(email_id.into_inner()).first::<Email>(conn)?;
    if email.user_id != cuser.id {
        return Err(StatusError::bad_request().into());
    }
    if email.is_master {
        return Err(StatusError::bad_request().brief("master email can not be deleted").into());
    }
    diesel::delete(&email).execute(conn)?;
    let emails = emails::table.filter(emails::user_id.eq(cuser.id)).get_results::<Email>(conn)?;
    Ok(Json(emails))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct UpdateInData {
    #[serde(default)]
    value: String,
}
#[endpoint(tags("account"))]
pub fn update(email_id: PathParam<i64>, pdata: JsonBody<UpdateInData>, depot: &mut Depot) -> JsonResult<Email> {
    let UpdateInData { value } = pdata.into_inner();
    let value = value.to_lowercase();
    if let Err(msg) = validator::validate_email(&value) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    let cuser = depot.current_user()?;
    let conn = &mut db::conn()?;
    let email = emails::table.find(email_id.into_inner()).first::<Email>(conn)?;
    let email = conn.transaction::<_, AppError, _>(|conn| {
        let exist_email = emails::table
            .filter(lower(emails::value).eq(&value))
            .filter(emails::id.ne(email.id))
            .first::<Email>(conn)
            .ok();
        if let Some(exist_email) = exist_email {
            if exist_email.user_id == cuser.id {
                return Err(StatusError::conflict()
                    .brief("email conflict")
                    .brief("This email is token in your account")
                    .into());
            } else {
                return Err(StatusError::conflict()
                    .brief("email conflict")
                    .brief("This email is token in other account")
                    .into());
            }
        }
        let email = diesel::update(emails::table.filter(emails::id.eq(email.id).and(emails::user_id.eq(cuser.id))))
            .set((
                emails::is_verified.eq(false),
                emails::value.eq(&value),
                emails::updated_by.eq(cuser.id),
                emails::updated_at.eq(Utc::now()),
            ))
            .get_result::<Email>(conn)?;
        Ok(email)
    })?;
    Ok(Json(email))
}
