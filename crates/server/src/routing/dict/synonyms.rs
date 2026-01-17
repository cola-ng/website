use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_synonyms(req: &mut Request) -> JsonResult<Vec<WordSynonymView>> {
    let word_id = super::get_path_id(req, "id")?;
    let rows: Vec<(WordSynonym, Word)> = with_conn(move |conn| {
        dict_word_synonyms::table
            .inner_join(dict_words::table.on(dict_word_synonyms::synonym_word_id.eq(dict_words::id)))
            .filter(dict_word_synonyms::word_id.eq(word_id))
            .load(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch synonyms"))?;

    json_ok(
        rows.into_iter()
            .map(|(link, w)| WordSynonymView {
                link,
                synonym: WordRef { id: w.id, word: w.word },
            })
            .collect(),
    )
}

#[derive(Deserialize)]
pub struct CreateSynonymRequest {
    pub synonym_word_id: i64,
    pub similarity_score: Option<f32>,
    pub context: Option<String>,
    pub nuance_notes: Option<String>,
}

#[handler]
pub async fn create_synonym(req: &mut Request) -> JsonResult<WordSynonym> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateSynonymRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let created: WordSynonym = with_conn(move |conn| {
        diesel::insert_into(dict_word_synonyms::table)
            .values(&NewWordSynonym {
                word_id,
                synonym_word_id: input.synonym_word_id,
                similarity_score: input.similarity_score,
                context: input.context,
                nuance_notes: input.nuance_notes,
            })
            .get_result::<WordSynonym>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create synonym"))?;

    json_ok(created)
}

