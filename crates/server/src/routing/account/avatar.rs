use std::path::{Path, PathBuf};

use chrono::Utc;
use diesel::prelude::*;
use image::{GenericImageView, ImageFormat};
use salvo::prelude::*;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::User;
use crate::{AppResult, DepotExt, JsonResult, json_ok};

/// Avatar sizes to generate (width x height)
const AVATAR_SIZES: [u32; 3] = [160, 320, 640];
/// Output formats for avatars
const AVATAR_FORMATS: [&str; 2] = ["webp", "png"];

/// Generate all avatar sizes for an image
/// Returns Ok if at least one size was generated
async fn generate_avatar_sizes<P: AsRef<Path>>(
    source_path: P,
    output_dir: P,
) -> AppResult<()> {
    let source = source_path.as_ref().to_path_buf();
    let dir = output_dir.as_ref().to_path_buf();

    tokio::task::spawn_blocking(move || {
        let img = image::open(&source)?;
        let (width, height) = img.dimensions();
        let min_dimension = width.min(height);

        let mut generated_count = 0;

        for size in AVATAR_SIZES {
            // Only generate if source is large enough
            if min_dimension < size {
                continue;
            }

            // Resize to square (thumbnail_exact maintains aspect and crops to square)
            let resized = img.thumbnail_exact(size, size);

            for format in AVATAR_FORMATS {
                let output_file = dir.join(format!("{size}x{size}.{format}"));

                match format {
                    "webp" => {
                        // Use lossless webp (image crate doesn't support lossy quality settings)
                        let file = std::fs::File::create(&output_file)?;
                        let encoder = image::codecs::webp::WebPEncoder::new_lossless(file);
                        resized.write_with_encoder(encoder)?;
                    }
                    "png" => {
                        resized.save_with_format(&output_file, ImageFormat::Png)?;
                    }
                    _ => {
                        resized.save(&output_file)?;
                    }
                }
                generated_count += 1;
            }
        }

        if generated_count == 0 {
            // Image too small, generate at least 160x160 from whatever we have
            let resized = img.thumbnail_exact(160, 160);
            for format in AVATAR_FORMATS {
                let output_file = dir.join(format!("160x160.{format}"));
                match format {
                    "webp" => {
                        let file = std::fs::File::create(&output_file)?;
                        let encoder = image::codecs::webp::WebPEncoder::new_lossless(file);
                        resized.write_with_encoder(encoder)?;
                    }
                    _ => {
                        resized.save_with_format(&output_file, ImageFormat::Png)?;
                    }
                }
            }
        }

        Ok::<(), image::ImageError>(())
    })
    .await
    .map_err(|e| {
        StatusError::internal_server_error()
            .brief(format!("image processing task failed: {}", e))
    })?
    .map_err(|e| {
        StatusError::internal_server_error()
            .brief(format!("image processing failed: {}", e))
    })?;

    Ok(())
}

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

    // Find the best available avatar file (prefer webp, then png)
    let file_path = if let Some(avatar_dir) = avatar {
        let candidates = [
            format!("{}/160x160.webp", avatar_dir),
            format!("{}/160x160.png", avatar_dir),
            format!("{}/320x320.webp", avatar_dir),
            format!("{}/320x320.png", avatar_dir),
        ];

        candidates
            .into_iter()
            .find(|p| PathBuf::from(p).exists())
            .unwrap_or_else(|| format!("{}/160x160.webp", avatar_dir))
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

    // Generate unique avatar directory using UTC timestamp
    let timestamp = Utc::now().timestamp();
    let avatar_dir = format!("uploads/avatars/{}", user_id);
    let store_dir = format!("{}/{}", avatar_dir, timestamp);

    // Create directory and copy original file
    std::fs::create_dir_all(&store_dir)?;
    let origin_file = format!("{}/origin.{}", store_dir, ext);
    std::fs::copy(file.path(), &origin_file)?;

    // Generate all avatar sizes (webp and png)
    if let Err(e) = generate_avatar_sizes(&origin_file, &store_dir).await {
        // Cleanup on failure
        std::fs::remove_dir_all(&store_dir).ok();
        return Err(e.into());
    }

    // Update user avatar in database (store the directory path)
    let avatar_value = store_dir.clone();
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
    if let Some(avatar_dir) = avatar {
        std::fs::remove_dir_all(&avatar_dir).ok();
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
