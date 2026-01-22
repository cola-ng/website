use diesel::prelude::*;
use salvo::prelude::*;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_categories(req: &mut Request) -> JsonResult<Vec<WordCategory>> {
    let word_id = super::get_path_id(req, "id")?;
    let categories: Vec<WordCategory> = with_conn(move |conn| {
        dict_word_categories::table
            .filter(dict_word_categories::word_id.eq(word_id))
            .load::<WordCategory>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch categories"))?;
    json_ok(categories)
}
