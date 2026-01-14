use chrono::Utc;
use diesel::prelude::*;
use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db;
use crate::models::*;
use crate::permission::Accessible;
use crate::schema::*;
use crate::utils::validator;
use crate::{AppError, DepotExt, JsonResult};

#[endpoint(tags("user"))]
pub async fn list(user_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<Vec<Phone>> {
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let user = users::table
        .find(user_id.into_inner())
        .first::<User>(conn)?
        .assign_to(cuser, "view", conn)?;

    let uphones = phones::table.filter(phones::user_id.eq(user.id)).get_results::<Phone>(conn)?;
    Ok(Json(uphones))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct CreateInData {
    #[serde(default)]
    value: String,
}
#[endpoint(tags("user"))]
pub async fn create(user_id: PathParam<i64>, pdata: JsonBody<CreateInData>, depot: &mut Depot) -> JsonResult<Phone> {
    let pdata = pdata.into_inner();
    if let Err(msg) = validator::validate_email(&pdata.value) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let user = users::table
        .filter(users::id.eq(user_id.into_inner()))
        .filter(users::is_disabled.eq(false))
        .first::<User>(conn)?
        .assign_to(cuser, "edit", conn)?;

    let phone = conn.transaction::<_, AppError, _>(|conn| {
        // check_phone_other_taken!(Some(cuser.id), &pdata.value, conn);
        let has_master = diesel_exists!(phones::table.filter(phones::user_id.eq(user.id)).filter(phones::is_master.eq(true)), conn);
        let new_phone = NewPhone {
            user_id: user.id,
            value: &pdata.value,
            is_verified: false,
            is_subscribed: false,
            is_master: !has_master,
            updated_by: Some(cuser.id),
            created_by: Some(cuser.id),
        };
        let phone = diesel::insert_into(phones::table).values(&new_phone).get_result::<Phone>(conn)?;
        Ok(phone)
    })?;
    Ok(Json(phone))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct TakeInData {
    #[serde(default)]
    value: String,
}
#[endpoint(tags("user"))]
pub async fn take(user_id: PathParam<i64>, pdata: JsonBody<TakeInData>, depot: &mut Depot) -> JsonResult<Phone> {
    let pdata = pdata.into_inner();
    if let Err(msg) = validator::validate_phone(&pdata.value) {
        return Err(StatusError::bad_request().brief(msg).into());
    }
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let user = users::table
        .filter(users::id.eq(user_id.into_inner()))
        .filter(users::is_disabled.eq(false))
        .first::<User>(conn)?
        .assign_to(cuser, "edit", conn)?;

    conn.transaction::<_, AppError, _>(|conn| {
        let query = phones::table.filter(phones::value.eq(&pdata.value));
        if diesel_exists!(query, conn) {
            return Err(StatusError::conflict().brief("phone conflict").brief("This phone already exist").into());
        }
        let new_phone = NewPhone {
            user_id: user.id,
            value: &pdata.value,
            is_verified: false,
            is_subscribed: false,
            is_master: false, //TODO
            updated_by: Some(cuser.id),
            created_by: Some(cuser.id),
        };
        let phone = diesel::insert_into(phones::table).values(&new_phone).get_result::<Phone>(conn)?;
        Ok(Json(phone))
    })
}
#[endpoint(tags("user"))]
pub async fn delete(user_id: PathParam<i64>, phone_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<Vec<Phone>> {
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let user = users::table
        .find(user_id.into_inner())
        .first::<User>(conn)?
        .assign_to(cuser, "edit", conn)?;
    let phone = phones::table.find(phone_id.into_inner()).first::<Phone>(conn)?;
    if phone.user_id != user.id {
        return Err(StatusError::bad_request().into());
    }

    if phone.is_master {
        return Err(StatusError::bad_request().brief("master phone can not be deleted").into());
    }
    diesel::delete(&phone).execute(conn)?;
    let phones = phones::table.filter(phones::user_id.eq(cuser.id)).get_results::<Phone>(conn)?;
    Ok(Json(phones))
}

//     #[derive(Deserialize, ToSchema, Debug)]
//     struct UpdateResData {
//         #[serde(default)]
//         value: String,
//     }
// #[endpoint(tags("user"))]
// pub async fn update(user_id: PathParam<i64>, pdata:JsonBody<UpdateResData>,req: &mut Request, depot: &mut Depot, res: &mut Response) -> JsonResult<Phone> {
//     let phone_id: i64 = req.param("phone_id").unwrap_or(0);
//     if phone_id <= 0 {
//         return Err(StatusError::bad_request().brief("phone_id param is invalid").into());
//     }
//     let pdata = pdata.into_inner();
//     if let Err(msg) = validator::validate_phone(&pdata.value) {
//         return Err(StatusError::bad_request().brief( &msg).into());
//     }
//     let cuser = depot.current_user()?;
//     let conn = &mut db::connect()?;
//     let user =  users::table.find(phone_id.into_inner()).first::< User>(conn)?;

//     user.assign_to(cuser, "edit", conn);

//     let uphone = phones::table.find(phone_id).first::<Phone>(conn)?;
//     if uphone.is_err() {
//         return Err(StatusError::bad_request().brief("phone is not exist").into());
//     }
//     let phone = conn.transaction::<_, AppError, _>(|| {
//         let exist_phone = phones::table
//             .filter(phones::value.eq(&pdata.value))
//             .filter(phones::id.ne(phone_id))
//             .first::<Phone>(conn)?;
//         if exist_phone.user_id == cuser.id {
//             return Err(StatusError::conflict().into());
//         }
//         let uphone = diesel::update(phones::table.filter(phones::id.eq(phone_id)).filter(phones::user_id.eq(cuser.id)))
//             .set((
//                 phones::is_verified.eq(false),
//                 phones::value.eq(&pdata.value),
//                 phones::updated_by.eq(cuser.id),
//                 phones::updated_at.eq(Utc::now()),
//             ))
//             .get_result::<Phone>(conn)?;
//         res.render();
//         Ok(uphone)
//     })?;
//     Ok(Json(phone))
// }
#[endpoint(tags("user"))]
pub async fn set_master(user_id: PathParam<i64>, phone_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<Phone> {
    let cuser = depot.current_user()?;
    let conn = &mut db::connect()?;
    let user = users::table
        .filter(users::id.eq(user_id.into_inner()))
        .filter(users::is_disabled.eq(false))
        .first::<User>(conn)?
        .assign_to(cuser, "edit", conn)?;
    let phone = phones::table.find(phone_id.into_inner()).first::<Phone>(conn)?;

    if phone.user_id != user.id {
        return Err(StatusError::bad_request().into());
    }
    if phone.is_master {
        return Ok(Json(phone));
    }

    conn.transaction::<_, AppError, _>(|conn| {
        diesel::update(
            phones::table.filter(
                phones::id
                    .ne(phone.id)
                    .and(phones::user_id.eq(phone.user_id))
                    .and(phones::is_master.eq(true)),
            ),
        )
        .set((
            phones::is_master.eq(false),
            phones::updated_by.eq(cuser.id),
            phones::updated_at.eq(Utc::now()),
        ))
        .execute(conn)?;
        let phone = diesel::update(phones::table.find(phone.id))
            .set((
                phones::is_master.eq(true),
                phones::updated_by.eq(cuser.id),
                phones::updated_at.eq(Utc::now()),
            ))
            .get_result::<Phone>(conn)?;
        Ok(Json(phone))
    })
}
