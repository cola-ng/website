use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_antonyms(req: &mut Request) -> JsonResult<Vec<DictWordAntonymView>> {
    let word_id = super::get_path_id(req, "id")?;
    let rows: Vec<(DictWordAntonym, DictWord)> = with_conn(move |conn| {
        dict_word_antonyms::table
            .inner_join(
                dict_words::table.on(dict_word_antonyms::antonym_word_id.eq(dict_words::id)),
            )
            .filter(dict_word_antonyms::word_id.eq(word_id))
            .load(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch antonyms"))?;

    json_ok(
        rows.into_iter()
            .map(|(link, w)| DictWordAntonymView {
                link,
                antonym: WordRef {
                    id: w.id,
                    word: w.word,
                },
            })
            .collect(),
    )
}

#[derive(Deserialize)]
pub struct CreateAntonymRequest {
    pub antonym_word_id: i64,
    pub antonym_type: Option<String>,
    pub context: Option<String>,
}

#[handler]
pub async fn create_antonym(req: &mut Request) -> JsonResult<DictWordAntonym> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateAntonymRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let created: DictWordAntonym = with_conn(move |conn| {
        diesel::insert_into(dict_word_antonyms::table)
            .values(&NewDictWordAntonym {
                word_id,
                antonym_word_id: input.antonym_word_id,
                antonym_type: input.antonym_type,
                context: input.context,
            })
            .get_result::<DictWordAntonym>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create antonym"))?;

    json_ok(created)
}
