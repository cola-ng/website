use chrono::Utc;
use diesel::prelude::*;
use salvo::http::StatusCode;
use salvo::oapi::extract::JsonBody;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::asset::*;
use crate::models::learn::*;
use crate::{AppResult, DepotExt, JsonResult, json_ok};

pub fn router() -> Router {
    Router::with_path("asset")
        .push(
            Router::with_path("scenes")
                .get(list_scenes)
                .push(Router::with_path("{id}").get(get_scene))
                .push(Router::with_path("{id}/dialogues").get(get_dialogues)),
        )
        .push(Router::with_path("dialogues/{dialogue_id}/turns").get(get_dialogue_turns))
        .push(Router::with_path("classic-sources").get(list_classic_sources))
        .push(Router::with_path("classic-clips").get(list_classic_clips))
        .push(
            Router::with_path("reading-exercises")
                .get(list_read_exercises)
                .push(Router::with_path("{id}/sentences").get(get_read_sentences)),
        )
        .push(Router::with_path("key-phrases").get(list_phrases))
}

// ============================================================================
// Scenes API (shared content)
// ============================================================================

#[handler]
pub async fn list_scenes(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let category_param = req.query::<String>("category");
    let difficulty = req.query::<String>("difficulty");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let scenes: Vec<Scene> = with_conn(move |conn| {
        let mut query = asset_scenes::table
            .filter(asset_scenes::is_active.eq(true))
            .order(asset_scenes::display_order.asc())
            .limit(limit)
            .into_boxed();

        if let Some(cat) = category_param {
            query = query.filter(asset_scenes::category.eq(cat));
        }
        if let Some(diff) = difficulty {
            query = query.filter(asset_scenes::difficulty.eq(diff));
        }

        query.load::<Scene>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list scenes"))?;

    res.render(Json(scenes));
    Ok(())
}

#[handler]
pub async fn get_scene(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let scene_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let scene: Scene = with_conn(move |conn| {
        asset_scenes::table
            .filter(asset_scenes::id.eq(scene_id))
            .first::<Scene>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("scene not found"))?;

    res.render(Json(scene));
    Ok(())
}

#[handler]
pub async fn get_dialogues(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let scene_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let dialogues: Vec<Dialogue> = with_conn(move |conn| {
        asset_dialogues::table
            .filter(asset_dialogues::scene_id.eq(scene_id))
            .load::<Dialogue>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list dialogues"))?;

    res.render(Json(dialogues));
    Ok(())
}

#[handler]
pub async fn get_dialogue_turns(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let dialogue_id: i64 = req
        .param::<i64>("dialogue_id")
        .ok_or_else(|| StatusError::bad_request().brief("missing dialogue_id"))?;

    let turns: Vec<DialogueTurn> = with_conn(move |conn| {
        asset_dialogue_turns::table
            .filter(asset_dialogue_turns::dialogue_id.eq(dialogue_id))
            .order(asset_dialogue_turns::turn_number.asc())
            .load::<DialogueTurn>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list turns"))?;

    res.render(Json(turns));
    Ok(())
}

// ============================================================================
// Classic Dialogues API (shared content)
// ============================================================================

#[handler]
pub async fn list_classic_sources(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let source_type_param = req.query::<String>("type");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let sources: Vec<ClassicDialogueSource> = with_conn(move |conn| {
        let mut query = asset_classic_sources::table.limit(limit).into_boxed();

        if let Some(st) = source_type_param {
            query = query.filter(asset_classic_sources::source_type.eq(st));
        }

        query.load::<ClassicDialogueSource>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list sources"))?;

    res.render(Json(sources));
    Ok(())
}

#[handler]
pub async fn list_classic_clips(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let source_id_param = req.query::<i64>("source_id");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let clips: Vec<ClassicDialogueClip> = with_conn(move |conn| {
        let mut query = asset_classic_clips::table
            .order(asset_classic_clips::popularity_score.desc())
            .limit(limit)
            .into_boxed();

        if let Some(sid) = source_id_param {
            query = query.filter(asset_classic_clips::source_id.eq(sid));
        }

        query.load::<ClassicDialogueClip>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list clips"))?;

    res.render(Json(clips));
    Ok(())
}

// ============================================================================
// Reading Exercises API (shared content)
// ============================================================================

#[handler]
pub async fn list_read_exercises(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let difficulty = req.query::<String>("difficulty");
    let exercise_type_param = req.query::<String>("type");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let exercises: Vec<ReadingExercise> = with_conn(move |conn| {
        let mut query = asset_read_exercises::table.limit(limit).into_boxed();

        if let Some(diff) = difficulty {
            query = query.filter(asset_read_exercises::difficulty.eq(diff));
        }
        if let Some(et) = exercise_type_param {
            query = query.filter(asset_read_exercises::exercise_type.eq(et));
        }

        query.load::<ReadingExercise>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list exercises"))?;

    res.render(Json(exercises));
    Ok(())
}

#[handler]
pub async fn get_read_sentences(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let exercise_id_param: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let sentences: Vec<ReadingSentence> = with_conn(move |conn| {
        asset_read_sentences::table
            .filter(asset_read_sentences::exercise_id.eq(exercise_id_param))
            .order(asset_read_sentences::sentence_order.asc())
            .load::<ReadingSentence>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list sentences"))?;

    res.render(Json(sentences));
    Ok(())
}

// ============================================================================
// Key Phrases API (shared content)
// ============================================================================

#[handler]
pub async fn list_phrases(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let category_param = req.query::<String>("category");
    let formality = req.query::<String>("formality");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let phrases: Vec<KeyPhrase> = with_conn(move |conn| {
        let mut query = asset_phrases::table.limit(limit).into_boxed();

        if let Some(cat) = category_param {
            query = query.filter(asset_phrases::category.eq(cat));
        }
        if let Some(form) = formality {
            query = query.filter(asset_phrases::formality_level.eq(form));
        }

        query.load::<KeyPhrase>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list phrases"))?;

    res.render(Json(phrases));
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

// ============================================================================
// Learning Sessions API (user-specific)
// ============================================================================

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    session_id: String,
    session_type: Option<String>,
    scene_id: Option<i64>,
    dialogue_id: Option<i64>,
    classic_clip_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct UpdateSessionRequest {
    ended_at: Option<String>,
    duration_seconds: Option<i32>,
    total_words_spoken: Option<i32>,
    average_wpm: Option<f32>,
    error_count: Option<i32>,
    correction_count: Option<i32>,
    notes: Option<String>,
    ai_summary_en: Option<String>,
    ai_summary_zh: Option<String>,
}

#[handler]
pub async fn list_sessions(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let session_type_param = req.query::<String>("type");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let sessions: Vec<LearningSession> = with_conn(move |conn| {
        let mut query = learn_sessions::table
            .filter(learn_sessions::user_id.eq(user_id))
            .order(learn_sessions::started_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(st) = session_type_param {
            query = query.filter(learn_sessions::session_type.eq(st));
        }

        query.load::<LearningSession>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list sessions"))?;

    res.render(Json(sessions));
    Ok(())
}

#[handler]
pub async fn create_session(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateSessionRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.session_id.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("session_id is required")
            .into());
    }

    let new_session = NewLearningSession {
        session_id: input.session_id,
        user_id,
        session_type: input.session_type,
        scene_id: input.scene_id,
        dialogue_id: input.dialogue_id,
        classic_clip_id: input.classic_clip_id,
    };

    let session: LearningSession = with_conn(move |conn| {
        diesel::insert_into(learn_sessions::table)
            .values(&new_session)
            .get_result::<LearningSession>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create session"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(session));
    Ok(())
}

#[handler]
pub async fn update_session(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let session_id_param: String = req
        .param::<String>("session_id")
        .ok_or_else(|| StatusError::bad_request().brief("missing session_id"))?;

    let input: UpdateSessionRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let ended_at_parsed = input
        .ended_at
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        })
        .flatten();

    let update = UpdateLearningSession {
        ended_at: ended_at_parsed,
        duration_seconds: input.duration_seconds,
        total_words_spoken: input.total_words_spoken,
        average_wpm: input.average_wpm,
        error_count: input.error_count,
        correction_count: input.correction_count,
        notes: input.notes,
        ai_summary_en: input.ai_summary_en,
        ai_summary_zh: input.ai_summary_zh,
    };

    let session: LearningSession = with_conn(move |conn| {
        diesel::update(
            learn_sessions::table
                .filter(learn_sessions::session_id.eq(session_id_param))
                .filter(learn_sessions::user_id.eq(user_id)),
        )
        .set(&update)
        .get_result::<LearningSession>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("session not found"))?;

    res.render(Json(session));
    Ok(())
}

// ============================================================================
// Conversations API (user-specific)
// ============================================================================

#[derive(Deserialize)]
pub struct CreateConversationRequest {
    session_id: String,
    speaker: String,
    use_lang: String,
    content_en: String,
    content_zh: String,
    audio_path: Option<String>,
    duration_ms: Option<i32>,
    words_per_minute: Option<f32>,
    pause_count: Option<i32>,
    hesitation_count: Option<i32>,
}

#[handler]
pub async fn list_learn_conversations(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let session_id_param = req.query::<String>("session_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let convos: Vec<Conversation> = with_conn(move |conn| {
        let mut query = learn_conversations::table
            .filter(learn_conversations::user_id.eq(user_id))
            .order(learn_conversations::created_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(sid) = session_id_param {
            query = query.filter(learn_conversations::session_id.eq(sid));
        }

        query.load::<Conversation>(conn)
    })
    .await
    .map_err(|_| {
        StatusError::internal_server_error().brief("failed to list learn_conversations")
    })?;

    res.render(Json(convos));
    Ok(())
}

#[handler]
pub async fn create_conversation(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateConversationRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let new_convo = NewConversation {
        user_id,
        session_id: input.session_id,
        speaker: input.speaker,
        use_lang: input.use_lang,
        content_en: input.content_en,
        content_zh: input.content_zh,
        audio_path: input.audio_path,
        duration_ms: input.duration_ms,
        words_per_minute: input.words_per_minute,
        pause_count: input.pause_count,
        hesitation_count: input.hesitation_count,
    };

    let convo: Conversation = with_conn(move |conn| {
        diesel::insert_into(learn_conversations::table)
            .values(&new_convo)
            .get_result::<Conversation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create conversation"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(convo));
    Ok(())
}
