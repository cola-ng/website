use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_word_family(req: &mut Request) -> JsonResult<Vec<DictWordFamilyView>> {
    let word_id = super::get_path_id(req, "id")?;
    let rows_out: Vec<(DictWordFamilyLink, DictWord)> = with_conn(move |conn| {
        dict_word_family::table
            .inner_join(dict_words::table.on(dict_word_family::related_word_id.eq(dict_words::id)))
            .filter(dict_word_family::root_word_id.eq(word_id))
            .load(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch word family"))?;

    json_ok(
        rows_out
            .into_iter()
            .map(|(link, w)| DictWordFamilyView {
                link,
                related: WordRef { id: w.id, word: w.word },
            })
            .collect(),
    )
}

#[derive(Deserialize)]
pub struct CreateWordFamilyRequest {
    pub related_word_id: i64,
    pub relationship_type: Option<String>,
    pub morpheme: Option<String>,
}

#[handler]
pub async fn create_word_family(req: &mut Request) -> JsonResult<DictWordFamilyLink> {
    let root_word_id = super::get_path_id(req, "id")?;
    let input: CreateWordFamilyRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let created: DictWordFamilyLink = with_conn(move |conn| {
        diesel::insert_into(dict_word_family::table)
            .values(&NewDictWordFamilyLink {
                root_word_id,
                related_word_id: input.related_word_id,
                relationship_type: input.relationship_type,
                morpheme: input.morpheme,
            })
            .get_result::<DictWordFamilyLink>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create word family"))?;

    json_ok(created)
}

