use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

mod category;
mod definition;
mod dictionary;
mod relation;
mod sentence;
mod form;
mod image;
mod pronunciation;
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
                .get(relation::list_etymology)
                .post(relation::create_etymology),
        )
        .push(
            Router::with_path("words/{id}/images")
                .get(image::list_images)
                .post(image::create_image),
        )
        .push(
            Router::with_path("words/{id}/images/{image_id}")
                .delete(image::delete_image),
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

        let definitions = dict_word_definitions::table
            .filter(dict_word_definitions::word_id.eq(word_id))
            .order(dict_word_definitions::definition_order.asc())
            .load::<WordDefinition>(conn)?;

        let examples = dict_word_sentences::table
            .filter(dict_word_sentences::word_id.eq(word_id))
            .order(dict_word_sentences::priority_order.asc())
            .load::<WordSentence>(conn)?;

        let pronunciations = dict_pronunciations::table
            .filter(dict_pronunciations::word_id.eq(word_id))
            .order(dict_pronunciations::is_primary.desc())
            .load::<Pronunciation>(conn)?;

        let synonyms = synonym_rows
            .into_iter()
            .map(|(link, w)| WordRelation {
                link,
                synonym: WordRef {
                    id: w.id,
                    word: w.word,
                },
            })
            .collect();

        let forms = dict_word_forms::table
            .filter(dict_word_forms::word_id.eq(word_id))
            .load::<WordForm>(conn)?;

        let categories = dict_categories::table
            .filter(
                dict_categories::id.eq_any(
                    dict_word_categories::table
                        .filter(dict_word_categories::word_id.eq(word_id))
                        .select(dict_word_categories::category_id),
                ),
            )
            .load::<Category>(conn)?;

        let images = dict_word_images::table
            .filter(dict_word_images::word_id.eq(word_id))
            .load::<WordImage>(conn)?;

        Ok(WordQueryResponse {
            word: word_record,
            definitions,
            examples,
            pronunciations,
            relations,
            forms,
            categories,
            images,
        })
    })
    .await
    .map_err(|_| StatusError::not_found().brief("word not found"))?;

    json_ok(result)
}
