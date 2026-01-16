use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::dict::*;
use crate::{JsonResult, json_ok};

#[handler]
pub async fn list_related_topics(req: &mut Request) -> JsonResult<Vec<DictRelatedTopic>> {
    let word_id = super::get_path_id(req, "id")?;
    let related_topics: Vec<DictRelatedTopic> = with_conn(move |conn| {
        dict_related_topics::table
            .filter(dict_related_topics::word_id.eq(word_id))
            .load::<DictRelatedTopic>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch related topics"))?;
    json_ok(related_topics)
}

#[derive(Deserialize)]
pub struct CreateRelatedTopicRequest {
    pub topic_name: String,
    pub topic_category: Option<String>,
    pub relevance_score: Option<f32>,
}

#[handler]
pub async fn create_related_topic(req: &mut Request) -> JsonResult<DictRelatedTopic> {
    let word_id = super::get_path_id(req, "id")?;
    let input: CreateRelatedTopicRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;
    if input.topic_name.trim().is_empty() {
        return Err(StatusError::bad_request().brief("topic_name is required").into());
    }
    let created: DictRelatedTopic = with_conn(move |conn| {
        diesel::insert_into(dict_related_topics::table)
            .values(&NewDictRelatedTopic {
                word_id,
                topic_name: input.topic_name.trim().to_string(),
                topic_category: input.topic_category,
                relevance_score: input.relevance_score,
            })
            .get_result::<DictRelatedTopic>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create related topic"))?;
    json_ok(created)
}

