use chrono::Utc;
use diesel::prelude::*;
use salvo::http::StatusCode;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::asset::*;
use crate::models::learn::*;
use crate::{AppResult, DepotExt};

pub fn router() -> Router {
    Router::with_path("asset")
        // Taxonomy
        .push(Router::with_path("domains").get(list_domains))
        .push(
            Router::with_path("categories")
                .get(list_categories)
                .push(Router::with_path("{domain_id}").get(get_categories_by_domain)),
        )
        // Contexts
        .push(
            Router::with_path("contexts")
                .get(list_contexts)
                .push(Router::with_path("{id}").get(get_context)),
        )
        // Stages
        .push(
            Router::with_path("stages")
                .get(list_stages)
                .push(Router::with_path("{id}").get(get_stage))
                .push(Router::with_path("{id}/scripts").get(get_scripts_by_stage)),
        )
        // Scripts (dialogues)
        .push(
            Router::with_path("scripts")
                .get(list_scripts)
                .push(Router::with_path("{script_id}/turns").get(get_script_turns)),
        )
        // Reading subjects
        .push(
            Router::with_path("reading-subjects")
                .get(list_read_subjects)
                .push(Router::with_path("{id}/sentences").get(get_read_sentences)),
        )
}

// ============================================================================
// Taxonomy API
// ============================================================================

#[handler]
pub async fn list_domains(_req: &mut Request, res: &mut Response) -> AppResult<()> {
    let domains: Vec<TaxonDomain> = with_conn(move |conn| {
        taxon_domains::table.load::<TaxonDomain>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list domains"))?;

    res.render(Json(domains));
    Ok(())
}

#[handler]
pub async fn list_categories(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let domain_id = req.query::<i64>("domain_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let categories: Vec<TaxonCategory> = with_conn(move |conn| {
        let mut query = taxon_categories::table.limit(limit).into_boxed();

        if let Some(did) = domain_id {
            query = query.filter(taxon_categories::domain_id.eq(did));
        }

        query.load::<TaxonCategory>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list categories"))?;

    res.render(Json(categories));
    Ok(())
}

#[handler]
pub async fn get_categories_by_domain(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let domain_id: i64 = req
        .param::<i64>("domain_id")
        .ok_or_else(|| StatusError::bad_request().brief("missing domain_id"))?;

    let categories: Vec<TaxonCategory> = with_conn(move |conn| {
        taxon_categories::table
            .filter(taxon_categories::domain_id.eq(domain_id))
            .load::<TaxonCategory>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list categories"))?;

    res.render(Json(categories));
    Ok(())
}

// ============================================================================
// Contexts API
// ============================================================================

#[handler]
pub async fn list_contexts(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let difficulty = req.query::<i16>("difficulty");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let contexts: Vec<Context> = with_conn(move |conn| {
        let mut query = asset_contexts::table
            .filter(asset_contexts::is_active.eq(true))
            .order(asset_contexts::display_order.asc())
            .limit(limit)
            .into_boxed();

        if let Some(diff) = difficulty {
            query = query.filter(asset_contexts::difficulty.eq(diff));
        }

        query.load::<Context>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list contexts"))?;

    res.render(Json(contexts));
    Ok(())
}

#[handler]
pub async fn get_context(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let context_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let context: Context = with_conn(move |conn| {
        asset_contexts::table
            .filter(asset_contexts::id.eq(context_id))
            .first::<Context>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("context not found"))?;

    res.render(Json(context));
    Ok(())
}

// ============================================================================
// Stages API
// ============================================================================

#[handler]
pub async fn list_stages(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let difficulty = req.query::<i16>("difficulty");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let stages: Vec<Stage> = with_conn(move |conn| {
        let mut query = asset_stages::table
            .filter(asset_stages::is_active.eq(true))
            .order(asset_stages::display_order.asc())
            .limit(limit)
            .into_boxed();

        if let Some(diff) = difficulty {
            query = query.filter(asset_stages::difficulty.eq(diff));
        }

        query.load::<Stage>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list stages"))?;

    res.render(Json(stages));
    Ok(())
}

#[handler]
pub async fn get_stage(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let stage_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let stage: Stage = with_conn(move |conn| {
        asset_stages::table
            .filter(asset_stages::id.eq(stage_id))
            .first::<Stage>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("stage not found"))?;

    res.render(Json(stage));
    Ok(())
}

#[handler]
pub async fn get_scripts_by_stage(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let stage_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let scripts: Vec<Script> = with_conn(move |conn| {
        asset_scripts::table
            .filter(asset_scripts::stage_id.eq(stage_id))
            .load::<Script>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list scripts"))?;

    res.render(Json(scripts));
    Ok(())
}

// ============================================================================
// Scripts API (dialogues)
// ============================================================================

#[handler]
pub async fn list_scripts(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let stage_id = req.query::<i64>("stage_id");
    let difficulty = req.query::<i16>("difficulty");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let scripts: Vec<Script> = with_conn(move |conn| {
        let mut query = asset_scripts::table.limit(limit).into_boxed();

        if let Some(sid) = stage_id {
            query = query.filter(asset_scripts::stage_id.eq(sid));
        }
        if let Some(diff) = difficulty {
            query = query.filter(asset_scripts::difficulty.eq(diff));
        }

        query.load::<Script>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list scripts"))?;

    res.render(Json(scripts));
    Ok(())
}

#[handler]
pub async fn get_script_turns(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let script_id: i64 = req
        .param::<i64>("script_id")
        .ok_or_else(|| StatusError::bad_request().brief("missing script_id"))?;

    let turns: Vec<ScriptTurn> = with_conn(move |conn| {
        asset_script_turns::table
            .filter(asset_script_turns::script_id.eq(script_id))
            .order(asset_script_turns::turn_number.asc())
            .load::<ScriptTurn>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list turns"))?;

    res.render(Json(turns));
    Ok(())
}

// ============================================================================
// Reading Subjects API
// ============================================================================

#[handler]
pub async fn list_read_subjects(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let difficulty = req.query::<i16>("difficulty");
    let subject_type = req.query::<String>("type");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let subjects: Vec<ReadSubject> = with_conn(move |conn| {
        let mut query = asset_read_subjects::table.limit(limit).into_boxed();

        if let Some(diff) = difficulty {
            query = query.filter(asset_read_subjects::difficulty.eq(diff));
        }
        if let Some(st) = subject_type {
            query = query.filter(asset_read_subjects::subject_type.eq(st));
        }

        query.load::<ReadSubject>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list subjects"))?;

    res.render(Json(subjects));
    Ok(())
}

#[handler]
pub async fn get_read_sentences(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let subject_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let sentences: Vec<ReadSentence> = with_conn(move |conn| {
        asset_read_sentences::table
            .filter(asset_read_sentences::subject_id.eq(subject_id))
            .order(asset_read_sentences::sentence_order.asc())
            .load::<ReadSentence>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list sentences"))?;

    res.render(Json(sentences));
    Ok(())
}

// ============================================================================
// Issue Words API (user-specific)
// ============================================================================

#[derive(Deserialize)]
pub struct CreateIssueWordRequest {
    word: String,
    issue_type: String,
    description_en: Option<String>,
    description_zh: Option<String>,
    context: Option<String>,
}

#[handler]
pub async fn list_learn_issue_words(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let due_only = req.query::<bool>("due_only").unwrap_or(false);
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let words: Vec<IssueWord> = with_conn(move |conn| {
        let mut query = learn_issue_words::table
            .filter(learn_issue_words::user_id.eq(user_id))
            .order(learn_issue_words::created_at.desc())
            .limit(limit)
            .into_boxed();

        if due_only {
            let now = Utc::now();
            query = query.filter(
                learn_issue_words::next_review_at
                    .is_null()
                    .or(learn_issue_words::next_review_at.le(now)),
            );
        }

        query.load::<IssueWord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list issue words"))?;

    res.render(Json(words));
    Ok(())
}

#[handler]
pub async fn create_issue_word(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateIssueWordRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.word.trim().is_empty() || input.issue_type.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("word and issue_type are required")
            .into());
    }

    let new_word = NewIssueWord {
        user_id,
        word: input.word,
        issue_type: input.issue_type,
        description_en: input.description_en,
        description_zh: input.description_zh,
        context: input.context,
    };

    let word: IssueWord = with_conn(move |conn| {
        diesel::insert_into(learn_issue_words::table)
            .values(&new_word)
            .get_result::<IssueWord>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create issue word"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(word));
    Ok(())
}
