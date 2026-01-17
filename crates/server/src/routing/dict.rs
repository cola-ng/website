use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

mod antonyms;
mod categories;
mod collocations;
mod definitions;
mod dictionaries;
mod etymology;
mod examples;
mod family;
mod forms;
mod images;
mod phrases;
mod pronunciations;
mod synonyms;
mod usage_notes;
mod word_dictionaries;
mod words;

pub fn router() -> Router {
    Router::with_path("dict")
        .push(Router::with_path("lookup").get(lookup))
        .push(
            Router::with_path("dictionaries")
                .get(dictionaries::list_dictionaries)
                .post(dictionaries::create_dictionary),
        )
        .push(
            Router::with_path("dictionaries/{id}")
                .get(dictionaries::get_dictionary)
                .put(dictionaries::update_dictionary)
                .delete(dictionaries::delete_dictionary),
        )
        .push(Router::with_path("words").get(words::list_words))
        .push(Router::with_path("words").post(words::create_word))
        .push(Router::with_path("words/{id}").put(words::update_word))
        .push(Router::with_path("words/{id}").delete(words::delete_word))
        .push(
            Router::with_path("words/{id}/dictionaries")
                .get(word_dictionaries::list_word_dictionaries)
                .post(word_dictionaries::create_word_dictionary),
        )
        .push(
            Router::with_path("words/{word_id}/dictionaries/{dictionary_id}")
                .delete(word_dictionaries::delete_word_dictionary),
        )
        .push(
            Router::with_path("word-dictionaries/{id}")
                .delete(word_dictionaries::delete_word_dictionary_by_id),
        )
        .push(
            Router::with_path("dictionaries/{id}/words")
                .get(word_dictionaries::list_dictionary_words),
        )
        .push(
            Router::with_path("words/{id}/definitions")
                .get(definitions::list_definitions)
                .post(definitions::create_definition),
        )
        .push(
            Router::with_path("words/{id}/pronunciations")
                .get(pronunciations::list_pronunciations)
                .post(pronunciations::create_pronunciation),
        )
        .push(
            Router::with_path("words/{id}/pronunciations/{pronunciation_id}")
                .delete(pronunciations::delete_pronunciation),
        )
        .push(
            Router::with_path("words/{id}/examples")
                .get(examples::list_examples)
                .post(examples::create_example),
        )
        .push(
            Router::with_path("words/{id}/synonyms")
                .get(synonyms::list_synonyms)
                .post(synonyms::create_synonym),
        )
        .push(
            Router::with_path("words/{id}/antonyms")
                .get(antonyms::list_antonyms)
                .post(antonyms::create_antonym),
        )
        .push(
            Router::with_path("words/{id}/forms")
                .get(forms::list_forms)
                .post(forms::create_form),
        )
        .push(
            Router::with_path("words/{id}/collocations")
                .get(collocations::list_collocations)
                .post(collocations::create_collocation),
        )
        .push(
            Router::with_path("words/{id}/word-families")
                .get(family::list_word_family)
                .post(family::create_word_family),
        )
        .push(
            Router::with_path("words/{id}/phrases")
                .get(phrases::list_phrases)
                .post(phrases::create_phrase),
        )
        .push(
            Router::with_path("words/{id}/idioms")
                .get(phrases::list_idioms)
                .post(phrases::create_idiom),
        )
        .push(
            Router::with_path("words/{id}/categories")
                .get(categories::list_categories)
                .post(categories::create_category),
        )
        .push(
            Router::with_path("words/{id}/tags")
                .get(categories::list_categories)
                .post(categories::create_category),
        )
        .push(
            Router::with_path("words/{id}/etymology")
                .get(etymology::list_etymology)
                .post(etymology::create_etymology),
        )
        .push(
            Router::with_path("words/{id}/usage-notes")
                .get(usage_notes::list_usage_notes)
                .post(usage_notes::create_usage_note),
        )
        .push(
            Router::with_path("words/{id}/images")
                .get(images::list_images)
                .post(images::create_image),
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

        let examples = dict_word_examples::table
            .filter(dict_word_examples::word_id.eq(word_id))
            .order(dict_word_examples::example_order.asc())
            .load::<WordExample>(conn)?;

        let pronunciations = dict_word_pronunciations::table
            .filter(dict_word_pronunciations::word_id.eq(word_id))
            .order(dict_word_pronunciations::is_primary.desc())
            .load::<WordPronunciation>(conn)?;

        let synonym_rows: Vec<(WordSynonym, Word)> = dict_word_synonyms::table
            .inner_join(
                dict_words::table.on(dict_word_synonyms::synonym_word_id.eq(dict_words::id)),
            )
            .filter(dict_word_synonyms::word_id.eq(word_id))
            .load(conn)?;
        let synonyms = synonym_rows
            .into_iter()
            .map(|(link, w)| WordSynonymView {
                link,
                synonym: WordRef {
                    id: w.id,
                    word: w.word,
                },
            })
            .collect();

        let antonym_rows: Vec<(WordAntonym, Word)> = dict_word_antonyms::table
            .inner_join(
                dict_words::table.on(dict_word_antonyms::antonym_word_id.eq(dict_words::id)),
            )
            .filter(dict_word_antonyms::word_id.eq(word_id))
            .load(conn)?;
        let antonyms = antonym_rows
            .into_iter()
            .map(|(link, w)| WordAntonymView {
                link,
                antonym: WordRef {
                    id: w.id,
                    word: w.word,
                },
            })
            .collect();

        let forms = dict_word_forms::table
            .filter(dict_word_forms::word_id.eq(word_id))
            .load::<WordForm>(conn)?;

        let collocations = dict_word_collocations::table
            .filter(dict_word_collocations::word_id.eq(word_id))
            .load::<WordCollocation>(conn)?;

        let family_out: Vec<(WordFamilyLink, Word)> = dict_word_family::table
            .inner_join(dict_words::table.on(dict_word_family::related_word_id.eq(dict_words::id)))
            .filter(dict_word_family::root_word_id.eq(word_id))
            .load(conn)?;
        let family_in: Vec<(WordFamilyLink, Word)> = dict_word_family::table
            .inner_join(dict_words::table.on(dict_word_family::root_word_id.eq(dict_words::id)))
            .filter(dict_word_family::related_word_id.eq(word_id))
            .load(conn)?;

        let mut word_family: Vec<WordFamilyView> = Vec::new();
        word_family.extend(family_out.into_iter().map(|(link, w)| WordFamilyView {
            link,
            related: WordRef {
                id: w.id,
                word: w.word,
            },
        }));
        word_family.extend(family_in.into_iter().map(|(link, w)| WordFamilyView {
            link,
            related: WordRef {
                id: w.id,
                word: w.word,
            },
        }));

        let phrases: Vec<Phrase> = dict_phrases::table
            .inner_join(
                dict_phrase_words::table.on(dict_phrase_words::phrase_id.eq(dict_phrases::id)),
            )
            .filter(dict_phrase_words::word_id.eq(word_id))
            .filter(
                dict_phrases::phrase_type
                    .is_null()
                    .or(dict_phrases::phrase_type.ne(Some("idiom"))),
            )
            .select(dict_phrases::all_columns)
            .distinct()
            .load(conn)?;

        let idioms: Vec<Phrase> = dict_phrases::table
            .inner_join(
                dict_phrase_words::table.on(dict_phrase_words::phrase_id.eq(dict_phrases::id)),
            )
            .filter(dict_phrase_words::word_id.eq(word_id))
            .filter(dict_phrases::phrase_type.eq(Some("idiom")))
            .select(dict_phrases::all_columns)
            .distinct()
            .load(conn)?;

        let categories = dict_categories::table
            .filter(
                dict_categories::id.eq_any(
                    dict_word_categories::table
                        .filter(dict_word_categories::word_id.eq(word_id))
                        .select(dict_word_categories::category_id),
                ),
            )
            .load::<Category>(conn)?;

        let etymology = dict_word_etymology::table
            .filter(dict_word_etymology::word_id.eq(word_id))
            .load::<WordEtymology>(conn)?;

        let usage_notes = dict_word_usage_notes::table
            .filter(dict_word_usage_notes::word_id.eq(word_id))
            .load::<WordUsageNote>(conn)?;

        let images = dict_word_images::table
            .filter(dict_word_images::word_id.eq(word_id))
            .load::<WordImage>(conn)?;

        Ok(WordQueryResponse {
            word: word_record,
            definitions,
            examples,
            pronunciations,
            synonyms,
            antonyms,
            forms,
            collocations,
            word_family,
            phrases,
            idioms,
            categories,
            etymology,
            usage_notes,
            images,
        })
    })
    .await
    .map_err(|_| StatusError::not_found().brief("word not found"))?;

    json_ok(result)
}
