use std::fs::create_dir_all;

use diesel::prelude::*;
use salvo::http::form::FilePart;

use crate::models::*;
use crate::schema::*;
use crate::{AppError, AppResult, db, utils};

pub static SCALED_SIZES: [usize; 3] = [1280, 640, 320];

pub async fn upload(user_id: i64, file: &FilePart) -> AppResult<User> {
    let uuid_name = utils::uuid_string();
    let store_dir = join_path!(super::avatar_base_dir(user_id, true), &uuid_name);
    let ext = utils::fs::get_file_ext(file.path());
    if !utils::fs::is_image_ext(&ext) {
        return Err(AppError::Public("unsupported image format".into()));
    }
    let origin_file = join_path!(&store_dir, format!("origin.{}", ext));
    create_dir_all(&store_dir)?;
    std::fs::copy(file.path(), &origin_file)?;
    let metadata = utils::media::get_image_info(&origin_file).await?;
    for size in [1280, 640, 320] {
        if metadata.width >= size && metadata.height >= size {
            let resized_file = join_path!(&store_dir, format!("{}x{}.webp", size, size));
            if let Err(e) = utils::media::resize_image(Some(size), Some(size), &origin_file, &resized_file).await {
                tracing::error!(error = ?e, "resize image failed");
            }
        }
    }
    if crate::aws_s3_bucket().is_some() {
        if let Err(e) = utils::fs::move_dir_to_s3(store_dir.clone(), None).await {
            tracing::error!(store_dir =  %store_dir, error = ?e, "move avatar to s3 error");
        }
    }
    let conn = &mut db::connect()?;
    let user = diesel::update(users::table.find(user_id))
        .set(users::avatar.eq(&*uuid_name))
        .get_result::<User>(conn)?;

    Ok(user)
}
