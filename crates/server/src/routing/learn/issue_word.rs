use chrono::Utc;
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::oapi::extract::JsonBody;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{DepotExt, JsonResult, json_ok};

#[derive(Deserialize, ToSchema)]
pub struct CreateIssueWordRequest {
    /// Word with issue
    word: String,
    /// Type of issue (pronunciation, grammar, etc.)
    issue_type: String,
    /// English description
    description_en: Option<String>,
    /// Chinese description
    description_zh: Option<String>,
    /// Context where the issue was found
    context: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct PaginatedIssueWords {
    pub items: Vec<IssueWord>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/// List issue words for current user
#[endpoint(tags("Learn"))]
pub async fn list_issue_words(req: &mut Request, depot: &mut Depot) -> JsonResult<PaginatedIssueWords> {
    let user_id = depot.user_id()?;
    let due_only = req.query::<bool>("due_only").unwrap_or(false);
    let page = req.query::<i64>("page").unwrap_or(1).max(1);
    let per_page = req.query::<i64>("per_page").unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let (items, total): (Vec<IssueWord>, i64) = with_conn(move |conn| {
        let mut query = learn_issue_words::table
            .filter(learn_issue_words::user_id.eq(user_id))
            .into_boxed();

        let mut count_query = learn_issue_words::table
            .filter(learn_issue_words::user_id.eq(user_id))
            .into_boxed();

        if due_only {
            let now = Utc::now();
            query = query.filter(
                learn_issue_words::next_review_at
                    .is_null()
                    .or(learn_issue_words::next_review_at.le(now)),
            );
            count_query = count_query.filter(
                learn_issue_words::next_review_at
                    .is_null()
                    .or(learn_issue_words::next_review_at.le(now)),
            );
        }

        let total: i64 = count_query.count().get_result(conn)?;

        let items = query
            .order(learn_issue_words::created_at.desc())
            .offset(offset)
            .limit(per_page)
            .load::<IssueWord>(conn)?;

        Ok((items, total))
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list issue words"))?;

    json_ok(PaginatedIssueWords {
        items,
        total,
        page,
        per_page,
    })
}

/// Create a new issue word
#[endpoint(tags("Learn"))]
pub async fn create_issue_word(
    input: JsonBody<CreateIssueWordRequest>,
    depot: &mut Depot,
) -> JsonResult<IssueWord> {
    let user_id = depot.user_id()?;

    if input.word.trim().is_empty() || input.issue_type.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("word and issue_type are required")
            .into());
    }

    let new_word = NewIssueWord {
        user_id,
        word: input.word.clone(),
        issue_type: input.issue_type.clone(),
        description_en: input.description_en.clone(),
        description_zh: input.description_zh.clone(),
        context: input.context.clone(),
    };

    let word: IssueWord = with_conn(move |conn| {
        diesel::insert_into(learn_issue_words::table)
            .values(&new_word)
            .get_result::<IssueWord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create issue word"))?;

    json_ok(word)
}
