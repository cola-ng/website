use diesel::prelude::*;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{AppResult, JsonResult, json_ok};

pub fn router() -> Router {
    Router::with_path("dict")
        .push(Router::with_path("words/<word>").get(query_word))
        .push(Router::with_path("words").get(list_words))
        .push(Router::with_path("words").post(create_word))
        .push(Router::with_path("words/<id>").put(update_word))
        .push(Router::with_path("words/<id>").delete(delete_word))
        .push(Router::with_path("words/<id>/definitions").get(list_definitions))
        .push(Router::with_path("words/<id>/definitions").post(create_definition))
        .push(Router::with_path("words/<id>/examples").get(list_examples))
        .push(Router::with_path("words/<id>/examples").post(create_example))
        .push(Router::with_path("words/<id>/synonyms").get(list_synonyms))
        .push(Router::with_path("words/<id>/synonyms").post(create_synonym))
        .push(Router::with_path("words/<id>/antonyms").get(list_antonyms))
        .push(Router::with_path("words/<id>/antonyms").post(create_antonym))
        .push(Router::with_path("words/<id>/forms").get(list_forms))
        .push(Router::with_path("words/<id>/forms").post(create_form))
        .push(Router::with_path("words/<id>/collocations").get(list_collocations))
        .push(Router::with_path("words/<id>/collocations").post(create_collocation))
        .push(Router::with_path("words/<id>/word-families").get(list_word_families))
        .push(Router::with_path("words/<id>/word-families").post(create_word_family))
        .push(Router::with_path("words/<id>/phrases").get(list_phrases))
        .push(Router::with_path("words/<id>/phrases").post(create_phrase))
        .push(Router::with_path("words/<id>/idioms").get(list_idioms))
        .push(Router::with_path("words/<id>/idioms").post(create_idiom))
        .push(Router::with_path("words/<id>/tags").get(list_tags))
        .push(Router::with_path("words/<id>/tags").post(create_tag))
        .push(Router::with_path("words/<id>/related-topics").get(list_related_topics))
        .push(Router::with_path("words/<id>/related-topics").post(create_related_topic))
        .push(Router::with_path("words/<id>/etymology").get(list_etymology))
        .push(Router::with_path("words/<id>/etymology").post(create_etymology))
        .push(Router::with_path("words/<id>/usage-notes").get(list_usage_notes))
        .push(Router::with_path("words/<id>/usage-notes").post(create_usage_note))
        .push(Router::with_path("words/<id>/images").get(list_images))
        .push(Router::with_path("words/<id>/images").post(create_image))
        .push(Router::with_path("words/<id>/videos").get(list_videos))
        .push(Router::with_path("words/<id>/videos").post(create_video))
        .push(Router::with_path("words/<id>/common-errors").get(list_common_errors))
        .push(Router::with_path("words/<id>/common-errors").post(create_common_error))
}

#[derive(Deserialize)]
pub struct ListWordsQuery {
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    offset: Option<i64>,
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    min_difficulty: Option<i32>,
    #[serde(default)]
    max_difficulty: Option<i32>,
}

#[handler]
pub async fn list_words(req: &mut Request) -> JsonResult<Vec<DictWord>> {
    let query: ListWordsQuery = req
        .parse_query()
        .unwrap_or_else(|_| ListWordsQuery {
            limit: None,
            offset: None,
            search: None,
            min_difficulty: None,
            max_difficulty: None,
        });

    let search_term = query.search;
    let min_diff = query.min_difficulty;
    let max_diff = query.max_difficulty;
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let words: Vec<DictWord> = with_conn(move |conn| {
        let mut query = dict_words::table
            .order(dict_words::word.asc())
            .limit(limit)
            .offset(offset)
            .into_boxed();

        if let Some(term) = search_term {
            query = query.filter(dict_words::word.ilike(format!("%{}%", term)));
        }

        if let Some(min_d) = min_diff {
            query = query.filter(dict_words::difficulty_level.ge(min_d));
        }

        if let Some(max_d) = max_diff {
            query = query.filter(dict_words::difficulty_level.le(max_d));
        }

        query.load::<DictWord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch words"))?;

    json_ok(words)
}

#[handler]
pub async fn create_word(req: &mut Request) -> JsonResult<DictWord> {
    let input: NewDictWord = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.word.trim().is_empty() {
        return Err(StatusError::bad_request().brief("word is required").into());
    }

    let word = input.clone();
    let created_word: DictWord = with_conn(move |conn| {
        diesel::insert_into(dict_words::table)
            .values(&word)
            .get_result::<DictWord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create word"))?;

    json_ok(created_word)
}

fn get_path_id(req: &Request, key: &str) -> Result<i64, StatusError> {
    let raw = req
        .params()
        .get(key)
        .cloned()
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;
    raw.parse()
        .map_err(|_| StatusError::bad_request().brief("invalid id"))
}

#[handler]
pub async fn update_word(req: &mut Request) -> JsonResult<DictWord> {
    let id = get_path_id(req, "id")?;
    let input: NewDictWord = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.word.trim().is_empty() {
        return Err(StatusError::bad_request().brief("word is required").into());
    }

    let word = input;
    let updated_word: DictWord = with_conn(move |conn| {
        diesel::update(dict_words::table.filter(dict_words::id.eq(id)))
            .set((
                dict_words::word.eq(word.word),
                dict_words::phonetic_us.eq(word.phonetic_us),
                dict_words::phonetic_uk.eq(word.phonetic_uk),
                dict_words::audio_path.eq(word.audio_path),
                dict_words::audio_url.eq(word.audio_url),
                dict_words::difficulty_level.eq(word.difficulty_level),
                dict_words::word_frequency_rank.eq(word.word_frequency_rank),
                dict_words::is_primary.eq(word.is_primary),
            ))
            .get_result::<DictWord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update word"))?;

    json_ok(updated_word)
}

#[handler]
pub async fn delete_word(req: &mut Request) -> JsonResult<serde_json::Value> {
    let id = get_path_id(req, "id")?;

    let _ = with_conn(move |conn| {
        diesel::delete(dict_words::table.filter(dict_words::id.eq(id))).execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete word"))?;

    json_ok(serde_json::json!({ "deleted": true }))
}

#[handler]
pub async fn query_word(req: &mut Request) -> JsonResult<WordQueryResponse> {
    let word = req
        .param::<String>("word")
        .ok_or_else(|| StatusError::bad_request().brief("missing word parameter"))?;

    let word_lower = word.to_lowercase();
    let word_clone = word_lower.clone();

    let result: WordQueryResponse = with_conn(move |conn| {
        let word_record = dict_words::table
            .filter(dict_words::word.eq(&word_clone))
            .first::<DictWord>(conn)
            .optional()?
            .ok_or_else(|| diesel::result::Error::NotFound)?;

        let word_id = word_record.id;

        let definitions = dict_word_definitions::table
            .filter(dict_word_definitions::word_id.eq(word_id))
            .order(dict_word_definitions::definition_order.asc())
            .load::<DictWordDefinition>(conn)?;

        let examples = dict_word_examples::table
            .filter(dict_word_examples::word_id.eq(word_id))
            .order(dict_word_examples::example_order.asc())
            .load::<DictWordExample>(conn)?;

        let synonyms = dict_synonyms::table
            .filter(dict_synonyms::word_id.eq(word_id))
            .load::<DictSynonym>(conn)?;

        let antonyms = dict_antonyms::table
            .filter(dict_antonyms::word_id.eq(word_id))
            .load::<DictAntonym>(conn)?;

        let forms = dict_word_forms::table
            .filter(dict_word_forms::word_id.eq(word_id))
            .load::<DictWordForm>(conn)?;

        let collocations = dict_collocations::table
            .filter(dict_collocations::word_id.eq(word_id))
            .load::<DictCollocation>(conn)?;

        let word_families = dict_word_families::table
            .filter(dict_word_families::word_id.eq(word_id))
            .load::<DictWordFamily>(conn)?;

        let phrases = dict_phrases::table
            .filter(dict_phrases::word_id.eq(word_id))
            .load::<DictPhrase>(conn)?;

        let idioms = dict_idioms::table
            .filter(dict_idioms::word_id.eq(word_id))
            .load::<DictIdiom>(conn)?;

        let tags = dict_word_tags::table
            .filter(dict_word_tags::word_id.eq(word_id))
            .load::<DictWordTag>(conn)?;

        let related_topics = dict_related_topics::table
            .filter(dict_related_topics::word_id.eq(word_id))
            .load::<DictRelatedTopic>(conn)?;

        let etymology = dict_etymology::table
            .filter(dict_etymology::word_id.eq(word_id))
            .load::<DictEtymology>(conn)?;

        let usage_notes = dict_usage_notes::table
            .filter(dict_usage_notes::word_id.eq(word_id))
            .load::<DictUsageNote>(conn)?;

        let images = dict_word_images::table
            .filter(dict_word_images::word_id.eq(word_id))
            .load::<DictWordImage>(conn)?;

        let videos = dict_word_videos::table
            .filter(dict_word_videos::word_id.eq(word_id))
            .load::<DictWordVideo>(conn)?;

        let common_errors = dict_common_errors::table
            .filter(dict_common_errors::word_id.eq(word_id))
            .load::<DictCommonError>(conn)?;

        Ok(WordQueryResponse {
            word: word_record,
            definitions,
            examples,
            synonyms,
            antonyms,
            forms,
            collocations,
            word_families,
            phrases,
            idioms,
            tags,
            related_topics,
            etymology,
            usage_notes,
            images,
            videos,
            common_errors,
        })
    })
    .await
    .map_err(|_| StatusError::not_found().brief("word not found"))?;

    json_ok(result)
}

#[handler]
pub async fn list_definitions(req: &mut Request) -> JsonResult<Vec<DictWordDefinition>> {
    let word_id = get_path_id(req, "id")?;

    let definitions: Vec<DictWordDefinition> = with_conn(move |conn| {
        dict_word_definitions::table
            .filter(dict_word_definitions::word_id.eq(word_id))
            .order(dict_word_definitions::definition_order.asc())
            .load::<DictWordDefinition>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch definitions"))?;

    json_ok(definitions)
}

#[handler]
pub async fn create_definition(req: &mut Request) -> JsonResult<DictWordDefinition> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictWordDefinition = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let definition = input.clone();
    let created_definition: DictWordDefinition = with_conn(move |conn| {
        diesel::insert_into(dict_word_definitions::table)
            .values(NewDictWordDefinition {
                word_id,
                ..definition
            })
            .get_result::<DictWordDefinition>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create definition"))?;

    json_ok(created_definition)
}

#[handler]
pub async fn list_examples(req: &mut Request) -> JsonResult<Vec<DictWordExample>> {
    let word_id = get_path_id(req, "id")?;

    let examples: Vec<DictWordExample> = with_conn(move |conn| {
        dict_word_examples::table
            .filter(dict_word_examples::word_id.eq(word_id))
            .order(dict_word_examples::example_order.asc())
            .load::<DictWordExample>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch examples"))?;

    json_ok(examples)
}

#[handler]
pub async fn create_example(req: &mut Request) -> JsonResult<DictWordExample> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictWordExample = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let example = input.clone();
    let created_example: DictWordExample = with_conn(move |conn| {
        diesel::insert_into(dict_word_examples::table)
            .values(NewDictWordExample {
                word_id,
                ..example
            })
            .get_result::<DictWordExample>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create example"))?;

    json_ok(created_example)
}

#[handler]
pub async fn list_synonyms(req: &mut Request) -> JsonResult<Vec<DictSynonym>> {
    let word_id = get_path_id(req, "id")?;

    let synonyms: Vec<DictSynonym> = with_conn(move |conn| {
        dict_synonyms::table
            .filter(dict_synonyms::word_id.eq(word_id))
            .load::<DictSynonym>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch synonyms"))?;

    json_ok(synonyms)
}

#[handler]
pub async fn create_synonym(req: &mut Request) -> JsonResult<DictSynonym> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictSynonym = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let synonym = input.clone();
    let created_synonym: DictSynonym = with_conn(move |conn| {
        diesel::insert_into(dict_synonyms::table)
            .values(NewDictSynonym {
                word_id,
                ..synonym
            })
            .get_result::<DictSynonym>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create synonym"))?;

    json_ok(created_synonym)
}

#[handler]
pub async fn list_antonyms(req: &mut Request) -> JsonResult<Vec<DictAntonym>> {
    let word_id = get_path_id(req, "id")?;

    let antonyms: Vec<DictAntonym> = with_conn(move |conn| {
        dict_antonyms::table
            .filter(dict_antonyms::word_id.eq(word_id))
            .load::<DictAntonym>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch antonyms"))?;

    json_ok(antonyms)
}

#[handler]
pub async fn create_antonym(req: &mut Request) -> JsonResult<DictAntonym> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictAntonym = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let antonym = input.clone();
    let created_antonym: DictAntonym = with_conn(move |conn| {
        diesel::insert_into(dict_antonyms::table)
            .values(NewDictAntonym {
                word_id,
                ..antonym
            })
            .get_result::<DictAntonym>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create antonym"))?;

    json_ok(created_antonym)
}

#[handler]
pub async fn list_forms(req: &mut Request) -> JsonResult<Vec<DictWordForm>> {
    let word_id = get_path_id(req, "id")?;

    let forms: Vec<DictWordForm> = with_conn(move |conn| {
        dict_word_forms::table
            .filter(dict_word_forms::word_id.eq(word_id))
            .load::<DictWordForm>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch forms"))?;

    json_ok(forms)
}

#[handler]
pub async fn create_form(req: &mut Request) -> JsonResult<DictWordForm> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictWordForm = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let form = input.clone();
    let created_form: DictWordForm = with_conn(move |conn| {
        diesel::insert_into(dict_word_forms::table)
            .values(NewDictWordForm {
                word_id,
                ..form
            })
            .get_result::<DictWordForm>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create form"))?;

    json_ok(created_form)
}

#[handler]
pub async fn list_collocations(req: &mut Request) -> JsonResult<Vec<DictCollocation>> {
    let word_id = get_path_id(req, "id")?;

    let collocations: Vec<DictCollocation> = with_conn(move |conn| {
        dict_collocations::table
            .filter(dict_collocations::word_id.eq(word_id))
            .load::<DictCollocation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch collocations"))?;

    json_ok(collocations)
}

#[handler]
pub async fn create_collocation(req: &mut Request) -> JsonResult<DictCollocation> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictCollocation = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let collocation = input.clone();
    let created_collocation: DictCollocation = with_conn(move |conn| {
        diesel::insert_into(dict_collocations::table)
            .values(NewDictCollocation {
                word_id,
                ..collocation
            })
            .get_result::<DictCollocation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create collocation"))?;

    json_ok(created_collocation)
}

#[handler]
pub async fn list_word_families(req: &mut Request) -> JsonResult<Vec<DictWordFamily>> {
    let word_id = get_path_id(req, "id")?;

    let word_families: Vec<DictWordFamily> = with_conn(move |conn| {
        dict_word_families::table
            .filter(dict_word_families::word_id.eq(word_id))
            .load::<DictWordFamily>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch word families"))?;

    json_ok(word_families)
}

#[handler]
pub async fn create_word_family(req: &mut Request) -> JsonResult<DictWordFamily> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictWordFamily = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let word_family = input.clone();
    let created_word_family: DictWordFamily = with_conn(move |conn| {
        diesel::insert_into(dict_word_families::table)
            .values(NewDictWordFamily {
                word_id,
                ..word_family
            })
            .get_result::<DictWordFamily>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create word family"))?;

    json_ok(created_word_family)
}

#[handler]
pub async fn list_phrases(req: &mut Request) -> JsonResult<Vec<DictPhrase>> {
    let word_id = get_path_id(req, "id")?;

    let phrases: Vec<DictPhrase> = with_conn(move |conn| {
        dict_phrases::table
            .filter(dict_phrases::word_id.eq(word_id))
            .load::<DictPhrase>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch phrases"))?;

    json_ok(phrases)
}

#[handler]
pub async fn create_phrase(req: &mut Request) -> JsonResult<DictPhrase> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictPhrase = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let phrase = input.clone();
    let created_phrase: DictPhrase = with_conn(move |conn| {
        diesel::insert_into(dict_phrases::table)
            .values(NewDictPhrase {
                word_id,
                ..phrase
            })
            .get_result::<DictPhrase>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create phrase"))?;

    json_ok(created_phrase)
}

#[handler]
pub async fn list_idioms(req: &mut Request) -> JsonResult<Vec<DictIdiom>> {
    let word_id = get_path_id(req, "id")?;

    let idioms: Vec<DictIdiom> = with_conn(move |conn| {
        dict_idioms::table
            .filter(dict_idioms::word_id.eq(word_id))
            .load::<DictIdiom>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch idioms"))?;

    json_ok(idioms)
}

#[handler]
pub async fn create_idiom(req: &mut Request) -> JsonResult<DictIdiom> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictIdiom = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let idiom = input.clone();
    let created_idiom: DictIdiom = with_conn(move |conn| {
        diesel::insert_into(dict_idioms::table)
            .values(NewDictIdiom {
                word_id,
                ..idiom
            })
            .get_result::<DictIdiom>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create idiom"))?;

    json_ok(created_idiom)
}

#[handler]
pub async fn list_tags(req: &mut Request) -> JsonResult<Vec<DictWordTag>> {
    let word_id = get_path_id(req, "id")?;

    let tags: Vec<DictWordTag> = with_conn(move |conn| {
        dict_word_tags::table
            .filter(dict_word_tags::word_id.eq(word_id))
            .load::<DictWordTag>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch tags"))?;

    json_ok(tags)
}

#[handler]
pub async fn create_tag(req: &mut Request) -> JsonResult<DictWordTag> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictWordTag = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let tag = input.clone();
    let created_tag: DictWordTag = with_conn(move |conn| {
        diesel::insert_into(dict_word_tags::table)
            .values(NewDictWordTag {
                word_id,
                ..tag
            })
            .get_result::<DictWordTag>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create tag"))?;

    json_ok(created_tag)
}

#[handler]
pub async fn list_related_topics(req: &mut Request) -> JsonResult<Vec<DictRelatedTopic>> {
    let word_id = get_path_id(req, "id")?;

    let related_topics: Vec<DictRelatedTopic> = with_conn(move |conn| {
        dict_related_topics::table
            .filter(dict_related_topics::word_id.eq(word_id))
            .load::<DictRelatedTopic>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch related topics"))?;

    json_ok(related_topics)
}

#[handler]
pub async fn create_related_topic(req: &mut Request) -> JsonResult<DictRelatedTopic> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictRelatedTopic = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let related_topic = input.clone();
    let created_related_topic: DictRelatedTopic = with_conn(move |conn| {
        diesel::insert_into(dict_related_topics::table)
            .values(NewDictRelatedTopic {
                word_id,
                ..related_topic
            })
            .get_result::<DictRelatedTopic>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create related topic"))?;

    json_ok(created_related_topic)
}

#[handler]
pub async fn list_etymology(req: &mut Request) -> JsonResult<Vec<DictEtymology>> {
    let word_id = get_path_id(req, "id")?;

    let etymology: Vec<DictEtymology> = with_conn(move |conn| {
        dict_etymology::table
            .filter(dict_etymology::word_id.eq(word_id))
            .load::<DictEtymology>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch etymology"))?;

    json_ok(etymology)
}

#[handler]
pub async fn create_etymology(req: &mut Request) -> JsonResult<DictEtymology> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictEtymology = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let etymology = input.clone();
    let created_etymology: DictEtymology = with_conn(move |conn| {
        diesel::insert_into(dict_etymology::table)
            .values(NewDictEtymology {
                word_id,
                ..etymology
            })
            .get_result::<DictEtymology>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create etymology"))?;

    json_ok(created_etymology)
}

#[handler]
pub async fn list_usage_notes(req: &mut Request) -> JsonResult<Vec<DictUsageNote>> {
    let word_id = get_path_id(req, "id")?;

    let usage_notes: Vec<DictUsageNote> = with_conn(move |conn| {
        dict_usage_notes::table
            .filter(dict_usage_notes::word_id.eq(word_id))
            .load::<DictUsageNote>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch usage notes"))?;

    json_ok(usage_notes)
}

#[handler]
pub async fn create_usage_note(req: &mut Request) -> JsonResult<DictUsageNote> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictUsageNote = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let usage_note = input.clone();
    let created_usage_note: DictUsageNote = with_conn(move |conn| {
        diesel::insert_into(dict_usage_notes::table)
            .values(NewDictUsageNote {
                word_id,
                ..usage_note
            })
            .get_result::<DictUsageNote>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create usage note"))?;

    json_ok(created_usage_note)
}

#[handler]
pub async fn list_images(req: &mut Request) -> JsonResult<Vec<DictWordImage>> {
    let word_id = get_path_id(req, "id")?;

    let images: Vec<DictWordImage> = with_conn(move |conn| {
        dict_word_images::table
            .filter(dict_word_images::word_id.eq(word_id))
            .load::<DictWordImage>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch images"))?;

    json_ok(images)
}

#[handler]
pub async fn create_image(req: &mut Request) -> JsonResult<DictWordImage> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictWordImage = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let image = input.clone();
    let created_image: DictWordImage = with_conn(move |conn| {
        diesel::insert_into(dict_word_images::table)
            .values(NewDictWordImage {
                word_id,
                ..image
            })
            .get_result::<DictWordImage>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create image"))?;

    json_ok(created_image)
}

#[handler]
pub async fn list_videos(req: &mut Request) -> JsonResult<Vec<DictWordVideo>> {
    let word_id = get_path_id(req, "id")?;

    let videos: Vec<DictWordVideo> = with_conn(move |conn| {
        dict_word_videos::table
            .filter(dict_word_videos::word_id.eq(word_id))
            .load::<DictWordVideo>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch videos"))?;

    json_ok(videos)
}

#[handler]
pub async fn create_video(req: &mut Request) -> JsonResult<DictWordVideo> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictWordVideo = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let video = input.clone();
    let created_video: DictWordVideo = with_conn(move |conn| {
        diesel::insert_into(dict_word_videos::table)
            .values(NewDictWordVideo {
                word_id,
                ..video
            })
            .get_result::<DictWordVideo>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create video"))?;

    json_ok(created_video)
}

#[handler]
pub async fn list_common_errors(req: &mut Request) -> JsonResult<Vec<DictCommonError>> {
    let word_id = get_path_id(req, "id")?;

    let common_errors: Vec<DictCommonError> = with_conn(move |conn| {
        dict_common_errors::table
            .filter(dict_common_errors::word_id.eq(word_id))
            .load::<DictCommonError>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch common errors"))?;

    json_ok(common_errors)
}

#[handler]
pub async fn create_common_error(req: &mut Request) -> JsonResult<DictCommonError> {
    let word_id = get_path_id(req, "id")?;
    let input: NewDictCommonError = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let common_error = input.clone();
    let created_common_error: DictCommonError = with_conn(move |conn| {
        diesel::insert_into(dict_common_errors::table)
            .values(NewDictCommonError {
                word_id,
                ..common_error
            })
            .get_result::<DictCommonError>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create common error"))?;

    json_ok(created_common_error)
}
