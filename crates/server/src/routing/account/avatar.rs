use std::path::PathBuf;

use diesel::prelude::*;
use salvo::prelude::*;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::User;
use crate::{AppResult, DepotExt, JsonResult, json_ok};

/// Get avatar for current user
#[endpoint(tags("Account"))]
pub async fn show(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;

    // Get user's avatar from database
    let avatar: Option<String> = with_conn(move |conn| {
        base_users::table
            .filter(base_users::id.eq(user_id))
            .select(base_users::avatar)
            .first::<Option<String>>(conn)
    })
    .await
    .ok()
    .flatten();

    let file_path = if let Some(avatar_path) = avatar {
        format!("{}/160x160.webp", avatar_path)
    } else {
        "avatars/defaults/160x160.webp".to_string()
    };

    // Try to send the file
    let path = PathBuf::from(&file_path);
    if path.exists() {
        res.send_file(&path, req.headers()).await;
    } else {
        res.status_code(StatusCode::NOT_FOUND);
    }
    Ok(())
}

/// Upload avatar for current user
#[endpoint(tags("Account"))]
pub async fn upload_avatar(req: &mut Request, depot: &mut Depot) -> JsonResult<User> {
    let user_id = depot.user_id()?;

    let Some(file) = req.file("image").await else {
        return Err(StatusError::bad_request()
            .brief("image file is required")
            .into());
    };

    // Get file extension
    let ext = file
        .path()
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();

    // Validate it's an image
    let valid_exts = ["jpg", "jpeg", "png", "gif", "webp"];
    if !valid_exts.contains(&ext.as_str()) {
        return Err(StatusError::bad_request()
            .brief("unsupported image format")
            .into());
    }

    // Generate unique avatar name
    let uuid_name = uuid::Uuid::new_v4().to_string();
    let avatar_dir = format!("uploads/avatars/{}", user_id);
    let store_dir = format!("{}/{}", avatar_dir, uuid_name);

    // Create directory and copy file
    std::fs::create_dir_all(&store_dir)?;
    let origin_file = format!("{}/origin.{}", store_dir, ext);
    std::fs::copy(file.path(), &origin_file)?;

    // Create a simple copy as 160x160.webp (in production, would resize with image library)
    let resized_file = format!("{}/160x160.webp", store_dir);
    std::fs::copy(&origin_file, &resized_file).ok();

    // Update user avatar in database
    let avatar_value = store_dir;
    let updated: User = with_conn(move |conn| {
        diesel::update(base_users::table.filter(base_users::id.eq(user_id)))
            .set(base_users::avatar.eq(&avatar_value))
            .get_result::<User>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update avatar"))?;

    json_ok(User::from(updated))
}

/// Delete avatar for current user
#[endpoint(tags("Account"))]
pub async fn delete_avatar(depot: &mut Depot) -> JsonResult<User> {
    let user_id = depot.user_id()?;

    // Get current avatar path to delete
    let avatar: Option<String> = with_conn(move |conn| {
        base_users::table
            .filter(base_users::id.eq(user_id))
            .select(base_users::avatar)
            .first::<Option<String>>(conn)
    })
    .await
    .ok()
    .flatten();

    // Remove avatar directory if exists
    if let Some(avatar_path) = avatar {
        std::fs::remove_dir_all(&avatar_path).ok();
    }

    // Clear avatar in database
    let updated: User = with_conn(move |conn| {
        diesel::update(base_users::table.filter(base_users::id.eq(user_id)))
            .set(base_users::avatar.eq::<Option<String>>(None))
            .get_result::<User>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete avatar"))?;

    json_ok(User::from(updated))
}
