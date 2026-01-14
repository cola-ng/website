use chrono::Utc;
use diesel::prelude::*;

use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::models::*;
use crate::permission::Accessible;
use crate::schema::*;
use crate::utils::validator;
use crate::{AppError, DepotExt, JsonResult, db};

#[endpoint(tags("user"))]
pub async fn list(user_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<Vec<Email>> {
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let user = users::table
        .find(user_id.into_inner())
        .first::<User>(conn)?
        .assign_to(cuser, "view", conn)?;

    let uemails = emails::table.filter(emails::user_id.eq(user.id)).get_results::<Email>(conn)?;
    Ok(Json(uemails))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct CreateInData {
    #[serde(default)]
    value: String,
}
#[endpoint(tags("user"))]
pub fn create(user_id: PathParam<i64>, pdata: JsonBody<CreateInData>, depot: &mut Depot) -> JsonResult<Email> {
    let pdata = pdata.into_inner();
    if let Err(msg) = validator::validate_email(&pdata.value) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    let cuser = depot.current_user()?;
    if !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }
    let conn = &mut db::connect()?;
    let user = users::table
        .find(user_id.into_inner())
        .first::<User>(conn)?
        .assign_to(cuser, "edit", conn)?;

    let email = conn.transaction::<Email, AppError, _>(|conn| {
        check_email_other_taken!(None, &pdata.value, conn);
        let has_master = diesel_exists!(emails::table.filter(emails::user_id.eq(user.id)).filter(emails::is_master.eq(true)), conn);
        let new_email = NewEmail {
            user_id: user.id,
            value: &pdata.value,
            is_verified: false,
            is_subscribed: false,
            is_master: !has_master,
            updated_by: Some(cuser.id),
            created_by: Some(cuser.id),
        };
        let email = diesel::insert_into(emails::table).values(&new_email).get_result::<Email>(conn)?;
        Ok(email)
    })?;

    user.send_verification_email(&email.value, conn)?;
    Ok(Json(email))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct TakeInData {
    #[serde(default)]
    value: String,
}
#[endpoint(tags("user"))]
pub fn take(user_id: PathParam<i64>, pdata: JsonBody<TakeInData>, depot: &mut Depot) -> JsonResult<Email> {
    let pdata = pdata.into_inner();
    if let Err(msg) = validator::validate_email(&pdata.value) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let user = users::table
        .find(user_id.into_inner())
        .first::<User>(conn)?
        .assign_to(cuser, "edit", conn)?;

    let email = conn.transaction::<Email, AppError, _>(|conn| {
        check_email_other_taken!(None, &pdata.value, conn);
        let new_email = NewEmail {
            user_id: user.id,
            value: &pdata.value,
            is_verified: false,
            is_subscribed: false,
            is_master: false, //TODO:
            updated_by: Some(cuser.id),
            created_by: Some(cuser.id),
        };
        let cemail = diesel::insert_into(emails::table).values(&new_email).get_result::<Email>(conn)?;
        Ok(cemail)
    })?;

    user.send_verification_email(&email.value, conn)?;
    Ok(Json(email))
}
#[endpoint(tags("user"))]
pub async fn delete(user_id: PathParam<i64>, email_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<Vec<Email>> {
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let user = users::table
        .find(user_id.into_inner())
        .first::<User>(conn)?
        .assign_to(cuser, "edit", conn)?;
    let email = emails::table.find(email_id.into_inner()).first::<Email>(conn)?;
    if email.user_id != user.id {
        return Err(StatusError::bad_request().into());
    }

    if email.is_master {
        return Err(StatusError::bad_request().brief("master email can not be deleted").into());
    }
    diesel::delete(&email).execute(conn)?;
    let emails = emails::table.filter(emails::user_id.eq(cuser.id)).get_results::<Email>(conn)?;
    Ok(Json(emails))
}
// #[endpoint(tags("user"))]
// pub async fn update(user_id: PathParam<i64>, pdata:JsonBody<FindInData>,req: &mut Request, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
//     #[derive(Deserialize, ToSchema, Debug)]
//     struct PostedData {
//         #[serde(default)]
//         value: String,
//     }
//     let email_id = req.param("email_id").unwrap_or(0i64);
//     if email_id <= 0 {
//         return Err(StatusError::bad_request().brief("Error happened when parse posted data.").into());
//     }
// let pdata = pdata.into_inner();
//     if let Err(msg) = validator::validate_email(&pdata.value) {
//         return Err(StatusError::bad_request().brief(&msg).into());
//     }
//     let cuser = depot.current_user()?;
//     let conn = &mut db::connect()?;
//     let user =  users::table.find(user_id.into_inner()).first::< User>(conn)?;

//     user.assign_to(cuser, "edit", conn);

//     let uemail = emails::table.find(email_id).first::<Email>(conn);
//     if uemail.is_err() {
//         return Err(StatusError::bad_request().brief("Error happened when parse posted data.").into());
//     }
//     conn.transaction::<_, AppError, _>(|conn| {
//         let exist_email = emails::table
//             .filter(emails::value.eq(&pdata.value))
//             .filter(emails::id.ne(email_id))
//             .first::<Email>(conn)
//             .ok();
//         if let Some(exist_email) = exist_email {
//             if exist_email.user_id == cuser.id {
//                 return Err(StatusError::conflict().into());
//             } else {
//                 return Err(StatusError::conflict().into());
//             }
//         }
//         let uemail = diesel::update(emails::table.filter(emails::id.eq(email_id).and(emails::user_id.eq(cuser.id))))
//             .set((
//                 emails::is_verified.eq(false),
//                 emails::value.eq(&pdata.value),
//                 emails::updated_by.eq(cuser.id),
//                 emails::updated_at.eq(Utc::now()),
//             ))
//             .get_result::<Email>(conn)?;
//         res.render(Json(uemail));
//         Ok(())
//     })?;
//     Ok(())
// }
#[endpoint(tags("user"))]
pub async fn set_master(user_id: PathParam<i64>, email_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<Email> {
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let user = users::table
        .find(user_id.into_inner())
        .first::<User>(conn)?
        .assign_to(cuser, "edit", conn)?;
    let email = emails::table.find(email_id.into_inner()).first::<Email>(conn)?;

    if email.user_id != user.id {
        return Err(StatusError::bad_request().into());
    }
    if email.is_master {
        return Ok(Json(email));
    }

    let email = conn.transaction::<_, AppError, _>(|conn| {
        diesel::update(emails::table.filter(emails::id.ne(email.id).and(emails::is_master.eq(true))))
            .set((
                emails::is_master.eq(false),
                emails::updated_by.eq(cuser.id),
                emails::updated_at.eq(Utc::now()),
            ))
            .execute(conn)?;
        let uemail = diesel::update(emails::table.find(email.id))
            .set((
                emails::is_master.eq(true),
                emails::updated_by.eq(cuser.id),
                emails::updated_at.eq(Utc::now()),
            ))
            .get_result::<Email>(conn)?;
        Ok(uemail)
    })?;
    Ok(Json(email))
}
