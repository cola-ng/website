use salvo::oapi::extract::*;
use salvo::prelude::*;

use crate::models::User;
use crate::{AppResult, DepotExt, JsonResult, things, utils};

static SCALED_SIZES: [usize; 3] = [1280, 640, 320];

#[endpoint(tags("account"))]
pub async fn show(
    width: PathParam<Option<usize>>,
    height: PathParam<Option<usize>>,
    ext: PathParam<Option<String>>,
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let cuser = depot.current_user()?;
    let width = width.into_inner().unwrap_or(320);
    let height = height.into_inner().unwrap_or(320);
    let ext = ext.into_inner().unwrap_or_else(|| "webp".into());
    let file_path = if let Some(avatar) = &cuser.avatar {
        join_path!(cuser.avatar_base_dir(false), avatar, format!("{width}x{height}.{ext}"))
    } else {
        join_path!("avatars/defaults", format!("{width}x{height}.webp"))
    };

    utils::fs::send_local_or_s3_file(file_path, req.headers(), res, None).await;
    Ok(())
}

#[endpoint(tags("account"))]
pub async fn upload(req: &mut Request, depot: &mut Depot) -> JsonResult<User> {
    let cuser = depot.current_user()?;
    let Some(file) = req.file("image").await else {
        return Err(StatusError::bad_request().brief("not found file in file field").into());
    };
    let user = things::user::avatar::upload(cuser.id, file).await?;
    Ok(Json(user))
}

#[endpoint(tags("account"))]
pub async fn delete(_req: &mut Request, depot: &mut Depot) -> AppResult<()> {
    let cuser = depot.current_user()?;
    ::std::fs::remove_dir_all(cuser.avatar_base_dir(true)).ok();
    if crate::aws_s3_bucket().is_some() {
        let dir_key = cuser.avatar_base_dir(false);
        if let Err(e) = crate::aws::s3::remove_dir(&dir_key).await {
            tracing::error!(dir_key = %dir_key, error = ?e, "remove avatar from s3 error");
        }
    }
    Ok(())
}
