use diesel::prelude::*;

use salvo::oapi::extract::*;
use salvo::prelude::*;

use crate::models::*;
use crate::permission::Accessible;
use crate::db::schema::*;
use crate::{AppResult, DepotExt, JsonResult, db, things, utils};

#[endpoint(tags("user"))]
pub async fn show(
    user_id: PathParam<i64>,
    width: PathParam<i64>,
    height: PathParam<i64>,
    ext: PathParam<String>,
    req: &mut Request,
    res: &mut Response,
) -> AppResult<()> {
    let mut conn = db::conn()?;
    let user = users::table.find(user_id.into_inner()).first::<User>(&mut conn)?;
    drop(conn);

    let file_path = if let Some(avatar) = &user.avatar {
        join_path!(user.avatar_base_dir(false), avatar, format!("{width}x{height}.{ext}"))
    } else {
        join_path!("avatars/defaults", format!("{width}x{height}.webp"))
    };
    utils::fs::send_local_or_s3_file(file_path, req.headers(), res, None).await;
    Ok(())
}

#[endpoint(tags("user"))]
pub async fn upload(user_id: PathParam<i64>, req: &mut Request, depot: &mut Depot) -> JsonResult<User> {
    let cuser = depot.current_user()?;
    let mut conn = db::conn()?;
    let user = users::table.find(user_id.into_inner()).first::<User>(&mut conn)?;
    if !user.permitted(cuser, "edit", &mut conn)? {
        return Err(StatusError::forbidden().into());
    }
    drop(conn);

    let file = req.file("image").await;
    if file.is_none() {
        return Err(StatusError::bad_request().brief("not found file in file field").into());
    }
    let file = file.unwrap();
    let user = things::user::avatar::upload(user.id, file).await?;
    Ok(Json(user))
}

#[endpoint(tags("user"))]
pub async fn delete(user_id: PathParam<i64>, depot: &mut Depot) -> AppResult<()> {
    let cuser = depot.current_user()?;
    let mut conn = db::conn()?;
    let user = users::table.find(user_id.into_inner()).first::<User>(&mut conn)?;
    if !user.permitted(cuser, "edit", &mut conn)? || !cuser.in_kernel {
        return Err(StatusError::forbidden().into());
    }
    drop(conn);

    ::std::fs::remove_dir_all(user.avatar_base_dir(true)).ok();
    if crate::aws_s3_bucket().is_some() {
        let dir_key = user.avatar_base_dir(false);
        if let Err(e) = crate::aws::s3::remove_dir(dir_key).await {
            tracing::error!(error = ?e, "remove avatar from s3 error");
        }
    }
    Ok(())
}
