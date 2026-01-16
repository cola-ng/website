use salvo::prelude::*;

mod antonyms;
mod categories;
mod collocations;
mod definitions;
mod etymology;
mod examples;
mod family;
mod forms;
mod images;
mod phrases;
mod related_topics;
mod synonyms;
mod usage_notes;
mod words;

pub fn router() -> Router {
    Router::with_path("dict")
        .push(Router::with_path("lookup/<word>").get(words::lookup_word))
        .push(Router::with_path("words").get(words::list_words))
        .push(Router::with_path("words").post(words::create_word))
        .push(Router::with_path("words/<id>").put(words::update_word))
        .push(Router::with_path("words/<id>").delete(words::delete_word))
        .push(
            Router::with_path("words/<id>/definitions")
                .get(definitions::list_definitions)
                .post(definitions::create_definition),
        )
        .push(
            Router::with_path("words/<id>/examples")
                .get(examples::list_examples)
                .post(examples::create_example),
        )
        .push(
            Router::with_path("words/<id>/synonyms")
                .get(synonyms::list_synonyms)
                .post(synonyms::create_synonym),
        )
        .push(
            Router::with_path("words/<id>/antonyms")
                .get(antonyms::list_antonyms)
                .post(antonyms::create_antonym),
        )
        .push(
            Router::with_path("words/<id>/forms")
                .get(forms::list_forms)
                .post(forms::create_form),
        )
        .push(
            Router::with_path("words/<id>/collocations")
                .get(collocations::list_collocations)
                .post(collocations::create_collocation),
        )
        .push(
            Router::with_path("words/<id>/word-families")
                .get(family::list_word_family)
                .post(family::create_word_family),
        )
        .push(
            Router::with_path("words/<id>/phrases")
                .get(phrases::list_phrases)
                .post(phrases::create_phrase),
        )
        .push(
            Router::with_path("words/<id>/idioms")
                .get(phrases::list_idioms)
                .post(phrases::create_idiom),
        )
        .push(
            Router::with_path("words/<id>/categories")
                .get(categories::list_categories)
                .post(categories::create_category),
        )
        .push(
            Router::with_path("words/<id>/tags")
                .get(categories::list_categories)
                .post(categories::create_category),
        )
        .push(
            Router::with_path("words/<id>/related-topics")
                .get(related_topics::list_related_topics)
                .post(related_topics::create_related_topic),
        )
        .push(
            Router::with_path("words/<id>/etymology")
                .get(etymology::list_etymology)
                .post(etymology::create_etymology),
        )
        .push(
            Router::with_path("words/<id>/usage-notes")
                .get(usage_notes::list_usage_notes)
                .post(usage_notes::create_usage_note),
        )
        .push(
            Router::with_path("words/<id>/images")
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
