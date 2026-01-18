use diesel::prelude::*;
use itertools::Itertools;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

mod category;
mod definition;
mod dictionary;
mod etymology;
mod form;
mod image;
mod pronunciation;
mod relation;
mod searched_words;
mod sentence;
mod words;

pub fn router() -> Router {
    Router::with_path("dict")
        .push(Router::with_path("lookup").get(lookup))
        .push(
            Router::with_path("dictionaries")
                .get(dictionary::list_dictionaries)
                .post(dictionary::create_dictionary),
        )
        .push(
            Router::with_path("dictionaries/{id}")
                .get(dictionary::get_dictionary)
                .put(dictionary::update_dictionary)
                .delete(dictionary::delete_dictionary),
        )
        .push(Router::with_path("words").get(words::list_words))
        .push(Router::with_path("words").post(words::create_word))
        .push(Router::with_path("words/{id}").put(words::update_word))
        .push(Router::with_path("words/{id}").delete(words::delete_word))
        .push(
            Router::with_path("words/{id}/definitions")
                .get(definition::list_definitions)
                .post(definition::create_definition),
        )
        .push(
            Router::with_path("words/{id}/pronunciations")
                .get(pronunciation::list_pronunciations)
                .post(pronunciation::create_pronunciation),
        )
        .push(
            Router::with_path("words/{id}/pronunciations/{pronunciation_id}")
                .delete(pronunciation::delete_pronunciation),
        )
        .push(
            Router::with_path("words/{id}/examples")
                .get(sentence::list_sentences)
                .post(sentence::create_sentence),
        )
        .push(
            Router::with_path("words/{id}/forms")
                .get(form::list_forms)
                .post(form::create_form),
        )
        .push(
            Router::with_path("words/{id}/categories")
                .get(category::list_categories)
                .post(category::create_category),
        )
        .push(
            Router::with_path("words/{id}/tags")
                .get(category::list_categories)
                .post(category::create_category),
        )
        .push(
            Router::with_path("words/{id}/etymology")
                .get(etymology::list_etymology)
                .post(etymology::create_etymology),
        )
        .push(
            Router::with_path("words/{id}/images")
                .get(image::list_images)
                .post(image::create_image),
        )
        .push(Router::with_path("words/{id}/images/{image_id}").delete(image::delete_image))
        .push(
            Router::with_path("words/searched")
                .get(searched_words::list_searched_words)
                .post(searched_words::create_searched_word)
                .delete(searched_words::clear_searched_words),
        )
}

pub(super) fn get_path_id(req: &Request, key: &str) -> Result<i64, StatusError> {
    let raw = req
        .params()
        .get(key)
        .cloned()
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;
    raw.parse()
        .map_err(|_| StatusError::bad_request().brief("invalid id"))
}

#[handler]
pub async fn lookup(req: &mut Request) -> JsonResult<WordQueryResponse> {
    let word = req
        .query::<String>("word")
        .ok_or_else(|| StatusError::bad_request().brief("missing word query"))?;
    let word_lower_value = word.trim().to_lowercase();
    println!("word_lower_value: {}", word_lower_value);
    if word_lower_value.is_empty() {
        return Err(StatusError::bad_request()
            .brief("missing word parameter")
            .into());
    }

    let result: WordQueryResponse = with_conn(move |conn| {
        let word_record = dict_words::table
            .filter(dict_words::word_lower.eq(&word_lower_value))
            .first::<Word>(conn)?;

        println!("word_record: {:?}", word_record);
        let word_id = word_record.id;

        let definitions = dict_definitions::table
            .filter(dict_definitions::word_id.eq(word_id))
            .order(dict_definitions::definition_order.asc())
            .load::<Definition>(conn)?;

        let sentence_ids = dict_word_sentences::table
            .filter(dict_word_sentences::word_id.eq(word_id))
            .select(dict_word_sentences::sentence_id)
            .order(dict_word_sentences::priority_order.desc())
            .load::<i64>(conn)?;

        let sentences = if sentence_ids.is_empty() {
            Vec::new()
        } else {
            dict_sentences::table
                .filter(dict_sentences::id.eq_any(&sentence_ids))
                .load::<Sentence>(conn)?
                .into_iter()
                .sorted_by_key(|s| {
                    sentence_ids
                        .iter()
                        .position(|&id| id == s.id)
                        .unwrap_or(usize::MAX)
                })
                .collect::<Vec<_>>()
        };

        let pronunciations = dict_pronunciations::table
            .filter(dict_pronunciations::word_id.eq(word_id))
            .order(dict_pronunciations::is_primary.desc())
            .load::<Pronunciation>(conn)?;

        let forms = dict_forms::table
            .filter(dict_forms::word_id.eq(word_id))
            .load::<Form>(conn)?;

        let categories = dict_categories::table
            .filter(
                dict_categories::id.eq_any(
                    dict_word_categories::table
                        .filter(dict_word_categories::word_id.eq(word_id))
                        .select(dict_word_categories::category_id),
                ),
            )
            .load::<Category>(conn)?;

        let images = dict_images::table
            .filter(dict_images::word_id.eq(word_id))
            .load::<Image>(conn)?;

        let relations = dict_relations::table
            .filter(dict_relations::word_id.eq(word_id))
            .load::<Relation>(conn)?;

        let etymology_ids = dict_word_etymologies::table
            .filter(dict_word_etymologies::word_id.eq(word_id))
            .select(dict_word_etymologies::etymology_id)
            .load::<i64>(conn)?;

        let etymologies = if etymology_ids.is_empty() {
            Vec::new()
        } else {
            dict_etymologies::table
                .filter(dict_etymologies::id.eq_any(&etymology_ids))
                .load::<Etymology>(conn)?
        };

        let dictionaries = dict_dictionaries::table
            .filter(
                dict_dictionaries::id.eq_any(
                    dict_word_dictionaries::table
                        .filter(dict_word_dictionaries::word_id.eq(word_id))
                        .select(dict_word_dictionaries::dictionary_id),
                ),
            )
            .load::<Dictionary>(conn)?;

        Ok(WordQueryResponse {
            word: word_record,
            definitions,
            sentences,
            pronunciations,
            dictionaries,
            relations,
            etymologies,
            forms,
            categories,
            images,
        })
    })
    .await
    .map_err(|_| StatusError::not_found().brief("word not found"))?;

    json_ok(result)
}
