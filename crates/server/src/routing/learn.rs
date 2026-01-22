use chrono::{Datelike, Duration, NaiveDate, Utc};
use diesel::prelude::*;
use salvo::oapi::extract::JsonBody;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{AppResult, DepotExt, JsonResult, OkResponse, hoops, json_ok};

pub fn router() -> Router {
    Router::with_path("learn")
        .hoop(hoops::require_auth)
        .push(Router::with_path("summary").get(get_learn_summary))
        .push(
            Router::with_path("issue-words")
                .get(list_issue_words)
                .post(create_issue_word)
                .delete(reset_issue_words),
        )
        .push(
            Router::with_path("chats")
                .get(list_chats)
                .post(create_chat)
                .delete(reset_chats),
        )
        .push(
            Router::with_path("chat-turns")
                .get(list_chat_turns)
                .post(create_chat_turn)
                .delete(reset_chat_turns),
        )
        .push(
            Router::with_path("chat-annotations")
                .get(list_chat_annotations)
                .post(create_chat_annotation)
                .delete(reset_chat_annotations),
        )
        .push(
            Router::with_path("write-practices")
                .get(list_write_practices)
                .post(create_write_practice)
                .delete(reset_write_practices),
        )
        .push(
            Router::with_path("read-practices")
                .get(list_read_practices)
                .post(create_read_practice)
                .delete(reset_read_practices),
        )
        .push(
            Router::with_path("vocabulary")
                .get(list_vocabulary)
                .post(create_vocabulary)
                .delete(reset_vocabulary),
        )
        .push(
            Router::with_path("daily-stats")
                .get(list_daily_stats)
                .post(upsert_daily_stat)
                .delete(reset_daily_stats),
        )
        .push(
            Router::with_path("achievements")
                .get(list_achievements)
                .delete(reset_achievements),
        )
        .push(
            Router::with_path("suggestions")
                .get(list_suggestions)
                .post(create_suggestion)
                .delete(reset_suggestions),
        )
        .push(Router::with_path("reset-all").delete(reset_all_learn_data))
}

// ============================================================================
// Issue Words API (user-specific)
// ============================================================================

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

/// List issue words for current user
#[endpoint(tags("Learn"))]
pub async fn list_issue_words(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<Vec<IssueWord>> {
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

    json_ok(words)
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

// ============================================================================
// Chats API (user-specific)
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct CreateChatRequest {
    title: String,
    duration_ms: Option<i32>,
    pause_count: Option<i32>,
}

#[handler]
pub async fn list_chats(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let chats: Vec<Chat> = with_conn(move |conn| {
        learn_chats::table
            .filter(learn_chats::user_id.eq(user_id))
            .order(learn_chats::created_at.desc())
            .limit(limit)
            .load::<Chat>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list chats"))?;

    res.render(Json(chats));
    Ok(())
}

#[handler]
pub async fn create_chat(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateChatRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.title.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("title is required")
            .into());
    }

    let new_chat = NewChat {
        user_id,
        title: input.title,
        duration_ms: input.duration_ms,
        pause_count: input.pause_count,
    };

    let chat: Chat = with_conn(move |conn| {
        diesel::insert_into(learn_chats::table)
            .values(&new_chat)
            .get_result::<Chat>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create chat"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(chat));
    Ok(())
}

// ============================================================================
// Chat Turns API (user-specific)
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct CreateChatTurnRequest {
    chat_id: String,
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
pub async fn list_chat_turns(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let chat_id_param = req.query::<String>("chat_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let turns: Vec<ChatTurn> = with_conn(move |conn| {
        let mut query = learn_chat_turns::table
            .filter(learn_chat_turns::user_id.eq(user_id))
            .order(learn_chat_turns::created_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(cid) = chat_id_param {
            query = query.filter(learn_chat_turns::chat_id.eq(cid));
        }

        query.load::<ChatTurn>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list chat turns"))?;

    res.render(Json(turns));
    Ok(())
}

#[handler]
pub async fn create_chat_turn(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateChatTurnRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let new_turn = NewChatTurn {
        user_id,
        chat_id: input.chat_id,
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

    let turn: ChatTurn = with_conn(move |conn| {
        diesel::insert_into(learn_chat_turns::table)
            .values(&new_turn)
            .get_result::<ChatTurn>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create chat turn"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(turn));
    Ok(())
}

// ============================================================================
// Chat Annotations API (user-specific)
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct CreateChatAnnotationRequest {
    chat_id: i64,
    chat_turn_id: i64,
    annotation_type: String,
    start_position: Option<i32>,
    end_position: Option<i32>,
    original_text: Option<String>,
    suggested_text: Option<String>,
    description_en: Option<String>,
    description_zh: Option<String>,
    severity: Option<String>,
}

#[handler]
pub async fn list_chat_annotations(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let chat_id_param = req.query::<i64>("chat_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let annotations: Vec<ChatAnnotation> = with_conn(move |conn| {
        let mut query = learn_chat_annotations::table
            .filter(learn_chat_annotations::user_id.eq(user_id))
            .order(learn_chat_annotations::created_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(cid) = chat_id_param {
            query = query.filter(learn_chat_annotations::chat_id.eq(cid));
        }

        query.load::<ChatAnnotation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list annotations"))?;

    res.render(Json(annotations));
    Ok(())
}

#[handler]
pub async fn create_chat_annotation(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateChatAnnotationRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let new_annotation = NewChatAnnotation {
        user_id,
        chat_id: input.chat_id,
        chat_turn_id: input.chat_turn_id,
        annotation_type: input.annotation_type,
        start_position: input.start_position,
        end_position: input.end_position,
        original_text: input.original_text,
        suggested_text: input.suggested_text,
        description_en: input.description_en,
        description_zh: input.description_zh,
        severity: input.severity,
    };

    let annotation: ChatAnnotation = with_conn(move |conn| {
        diesel::insert_into(learn_chat_annotations::table)
            .values(&new_annotation)
            .get_result::<ChatAnnotation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create annotation"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(annotation));
    Ok(())
}

// ============================================================================
// User Vocabulary API (user-specific)
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct CreateVocabularyRequest {
    word: String,
    word_zh: Option<String>,
}

#[handler]
pub async fn list_vocabulary(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let due_only = req.query::<bool>("due_only").unwrap_or(false);
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let vocab: Vec<UserVocabulary> = with_conn(move |conn| {
        let mut query = learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .order(learn_vocabularies::first_seen_at.desc())
            .limit(limit)
            .into_boxed();

        if due_only {
            let now = Utc::now();
            query = query.filter(
                learn_vocabularies::next_review_at
                    .is_null()
                    .or(learn_vocabularies::next_review_at.le(now)),
            );
        }

        query.load::<UserVocabulary>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list vocabulary"))?;

    res.render(Json(vocab));
    Ok(())
}

#[handler]
pub async fn create_vocabulary(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateVocabularyRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.word.trim().is_empty() {
        return Err(StatusError::bad_request().brief("word is required").into());
    }

    let new_vocab = NewUserVocabulary {
        user_id,
        word: input.word,
        word_zh: input.word_zh,
    };

    let vocab: UserVocabulary = with_conn(move |conn| {
        diesel::insert_into(learn_vocabularies::table)
            .values(&new_vocab)
            .get_result::<UserVocabulary>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create vocabulary"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(vocab));
    Ok(())
}

// ============================================================================
// Daily Stats API (user-specific)
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct UpsertDailyStatRequest {
    stat_date: String,
    minutes_studied: Option<i32>,
    words_practiced: Option<i32>,
    sessions_completed: Option<i32>,
    errors_corrected: Option<i32>,
    new_words_learned: Option<i32>,
    review_words_count: Option<i32>,
}

#[handler]
pub async fn list_daily_stats(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let limit = req.query::<i64>("limit").unwrap_or(30).clamp(1, 365);

    let stats: Vec<DailyStat> = with_conn(move |conn| {
        learn_daily_stats::table
            .filter(learn_daily_stats::user_id.eq(user_id))
            .order(learn_daily_stats::stat_date.desc())
            .limit(limit)
            .load::<DailyStat>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list daily stats"))?;

    res.render(Json(stats));
    Ok(())
}

#[handler]
pub async fn upsert_daily_stat(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: UpsertDailyStatRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let date = NaiveDate::parse_from_str(&input.stat_date, "%Y-%m-%d")
        .map_err(|_| StatusError::bad_request().brief("invalid date format, use YYYY-MM-DD"))?;

    let new_stat = NewDailyStat {
        user_id,
        stat_date: date,
        minutes_studied: input.minutes_studied,
        words_practiced: input.words_practiced,
        sessions_completed: input.sessions_completed,
        errors_corrected: input.errors_corrected,
        new_words_learned: input.new_words_learned,
        review_words_count: input.review_words_count,
    };

    let stat: DailyStat = with_conn(move |conn| {
        diesel::insert_into(learn_daily_stats::table)
            .values(&new_stat)
            .on_conflict((learn_daily_stats::user_id, learn_daily_stats::stat_date))
            .do_update()
            .set(&UpdateDailyStat {
                minutes_studied: input.minutes_studied,
                words_practiced: input.words_practiced,
                sessions_completed: input.sessions_completed,
                errors_corrected: input.errors_corrected,
                new_words_learned: input.new_words_learned,
                review_words_count: input.review_words_count,
            })
            .get_result::<DailyStat>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to upsert daily stat"))?;

    res.render(Json(stat));
    Ok(())
}

// ============================================================================
// User Achievements API (user-specific)
// ============================================================================

#[handler]
pub async fn list_achievements(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let achievements: Vec<UserAchievement> = with_conn(move |conn| {
        learn_achievements::table
            .filter(learn_achievements::user_id.eq(user_id))
            .order(learn_achievements::earned_at.desc())
            .load::<UserAchievement>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list achievements"))?;

    res.render(Json(achievements));
    Ok(())
}

// ============================================================================
// Write Practices API (user-specific)
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct CreateWritePracticeRequest {
    word_id: i64,
    practice_id: String,
    success_level: Option<i32>,
    notes: Option<String>,
}

#[handler]
pub async fn list_write_practices(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let word_id_param = req.query::<i64>("word_id");
    let practice_id_param = req.query::<String>("practice_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let practices: Vec<WritePractice> = with_conn(move |conn| {
        let mut query = learn_write_practices::table
            .filter(learn_write_practices::user_id.eq(user_id))
            .order(learn_write_practices::updated_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(wid) = word_id_param {
            query = query.filter(learn_write_practices::word_id.eq(wid));
        }
        if let Some(pid) = practice_id_param {
            query = query.filter(learn_write_practices::practice_id.eq(pid));
        }

        query.load::<WritePractice>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list write practices"))?;

    res.render(Json(practices));
    Ok(())
}

#[handler]
pub async fn create_write_practice(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateWritePracticeRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let new_practice = NewWritePractice {
        user_id,
        word_id: input.word_id,
        practice_id: input.practice_id,
        success_level: input.success_level,
        notes: input.notes,
    };

    let practice: WritePractice = with_conn(move |conn| {
        diesel::insert_into(learn_write_practices::table)
            .values(&new_practice)
            .get_result::<WritePractice>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create write practice"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(practice));
    Ok(())
}

// ============================================================================
// Reading Practices API (user-specific)
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct CreateReadPracticeRequest {
    sentence_id: Option<i64>,
    practice_id: String,
    user_audio_path: Option<String>,
    pronunciation_score: Option<i32>,
    fluency_score: Option<i32>,
    intonation_score: Option<i32>,
    overall_score: Option<i32>,
    detected_errors: Option<serde_json::Value>,
    ai_feedback_en: Option<String>,
    ai_feedback_zh: Option<String>,
    waveform_data: Option<serde_json::Value>,
}

#[handler]
pub async fn list_read_practices(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let sentence_id_param = req.query::<i64>("sentence_id");
    let practice_id_param = req.query::<String>("practice_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let practices: Vec<ReadPractice> = with_conn(move |conn| {
        let mut query = learn_read_practices::table
            .filter(learn_read_practices::user_id.eq(user_id))
            .order(learn_read_practices::created_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(sid) = sentence_id_param {
            query = query.filter(learn_read_practices::sentence_id.eq(sid));
        }
        if let Some(pid) = practice_id_param {
            query = query.filter(learn_read_practices::practice_id.eq(pid));
        }

        query.load::<ReadPractice>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list read practices"))?;

    res.render(Json(practices));
    Ok(())
}

#[handler]
pub async fn create_read_practice(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateReadPracticeRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let new_practice = NewReadPractice {
        user_id,
        sentence_id: input.sentence_id,
        practice_id: input.practice_id,
        user_audio_path: input.user_audio_path,
        pronunciation_score: input.pronunciation_score,
        fluency_score: input.fluency_score,
        intonation_score: input.intonation_score,
        overall_score: input.overall_score,
        detected_errors: input.detected_errors,
        ai_feedback_en: input.ai_feedback_en,
        ai_feedback_zh: input.ai_feedback_zh,
        waveform_data: input.waveform_data,
    };

    let practice: ReadPractice = with_conn(move |conn| {
        diesel::insert_into(learn_read_practices::table)
            .values(&new_practice)
            .get_result::<ReadPractice>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create read practice"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(practice));
    Ok(())
}

// ============================================================================
// Suggestions API (user-specific)
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct CreateSuggestionRequest {
    suggestion_type: Option<String>,
    suggested_text: String,
    was_accepted: Option<bool>,
}

#[handler]
pub async fn list_suggestions(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let suggestion_type_param = req.query::<String>("type");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let suggestions: Vec<Suggestion> = with_conn(move |conn| {
        let mut query = learn_suggestions::table
            .filter(learn_suggestions::user_id.eq(user_id))
            .order(learn_suggestions::created_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(st) = suggestion_type_param {
            query = query.filter(learn_suggestions::suggestion_type.eq(st));
        }

        query.load::<Suggestion>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list suggestions"))?;

    res.render(Json(suggestions));
    Ok(())
}

#[handler]
pub async fn create_suggestion(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateSuggestionRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.suggested_text.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("suggested_text is required")
            .into());
    }

    let new_suggestion = NewSuggestion {
        user_id,
        suggestion_type: input.suggestion_type,
        suggested_text: input.suggested_text,
        was_accepted: input.was_accepted,
    };

    let suggestion: Suggestion = with_conn(move |conn| {
        diesel::insert_into(learn_suggestions::table)
            .values(&new_suggestion)
            .get_result::<Suggestion>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create suggestion"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(suggestion));
    Ok(())
}

// ============================================================================
// Reset APIs - Delete all user data for each table
// ============================================================================

#[derive(Serialize)]
struct ResetResponse {
    deleted_count: usize,
    table: String,
}

#[handler]
pub async fn reset_issue_words(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_issue_words::table.filter(learn_issue_words::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset issue words"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_issue_words".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_chats(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_chats::table.filter(learn_chats::user_id.eq(user_id))).execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset chats"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_chats".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_chat_turns(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_chat_turns::table.filter(learn_chat_turns::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset chat turns"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_chat_turns".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_chat_annotations(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(
            learn_chat_annotations::table.filter(learn_chat_annotations::user_id.eq(user_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset chat annotations"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_chat_annotations".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_write_practices(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(
            learn_write_practices::table.filter(learn_write_practices::user_id.eq(user_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset write practices"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_write_practices".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_read_practices(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(
            learn_read_practices::table.filter(learn_read_practices::user_id.eq(user_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset read practices"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_read_practices".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_vocabulary(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_vocabularies::table.filter(learn_vocabularies::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset vocabulary"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_vocabularies".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_daily_stats(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_daily_stats::table.filter(learn_daily_stats::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset daily stats"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_daily_stats".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_achievements(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_achievements::table.filter(learn_achievements::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset achievements"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_achievements".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_suggestions(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_suggestions::table.filter(learn_suggestions::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset suggestions"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_suggestions".to_string(),
    }));
    Ok(())
}

// ============================================================================
// Reset All - Delete all learning data for the user
// ============================================================================

#[derive(Serialize)]
struct ResetAllResponse {
    tables_reset: Vec<ResetResponse>,
    total_deleted: usize,
}

#[handler]
pub async fn reset_all_learn_data(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let results = with_conn(move |conn| {
        let mut tables_reset = Vec::new();

        // Delete in order to respect foreign key constraints
        // First delete dependent tables, then parent tables

        // Chat annotations (depends on chat turns)
        let count = diesel::delete(
            learn_chat_annotations::table.filter(learn_chat_annotations::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_chat_annotations".to_string(),
        });

        // Chat turns
        let count = diesel::delete(
            learn_chat_turns::table.filter(learn_chat_turns::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_chat_turns".to_string(),
        });

        // Chats
        let count =
            diesel::delete(learn_chats::table.filter(learn_chats::user_id.eq(user_id)))
                .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_chats".to_string(),
        });

        // Write practices
        let count = diesel::delete(
            learn_write_practices::table.filter(learn_write_practices::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_write_practices".to_string(),
        });

        // Read practices
        let count = diesel::delete(
            learn_read_practices::table.filter(learn_read_practices::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_read_practices".to_string(),
        });

        // Issue words
        let count =
            diesel::delete(learn_issue_words::table.filter(learn_issue_words::user_id.eq(user_id)))
                .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_issue_words".to_string(),
        });

        // Vocabulary
        let count = diesel::delete(
            learn_vocabularies::table.filter(learn_vocabularies::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_vocabularies".to_string(),
        });

        // Daily stats
        let count =
            diesel::delete(learn_daily_stats::table.filter(learn_daily_stats::user_id.eq(user_id)))
                .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_daily_stats".to_string(),
        });

        // Achievements
        let count = diesel::delete(
            learn_achievements::table.filter(learn_achievements::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_achievements".to_string(),
        });

        // Suggestions
        let count =
            diesel::delete(learn_suggestions::table.filter(learn_suggestions::user_id.eq(user_id)))
                .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_suggestions".to_string(),
        });

        Ok::<_, diesel::result::Error>(tables_reset)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset all learn data"))?;

    let total_deleted: usize = results.iter().map(|r| r.deleted_count).sum();

    res.render(Json(ResetAllResponse {
        tables_reset: results,
        total_deleted,
    }));
    Ok(())
}

// ============================================================================
// Learn Summary API - Aggregated learning data for dashboard
// ============================================================================

#[derive(Serialize)]
pub struct WeeklyMinutes {
    /// Day of week: 0 = Monday, 6 = Sunday
    pub day: i32,
    /// Date in YYYY-MM-DD format
    pub date: String,
    /// Minutes studied on this day
    pub minutes: i32,
}

#[derive(Serialize)]
pub struct LearnSummary {
    /// Whether the user has any learning data
    pub has_data: bool,
    /// Total conversation minutes this week
    pub weekly_conversation_minutes: i32,
    /// Number of mastered vocabulary words (mastery_level >= 4)
    pub mastered_vocabulary_count: i64,
    /// Number of words due for review
    pub pending_review_count: i64,
    /// Minutes studied per day this week (Monday to Sunday)
    pub weekly_minutes: Vec<WeeklyMinutes>,
}

#[handler]
pub async fn get_learn_summary(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let summary = with_conn(move |conn| {
        let now = Utc::now();
        let today = now.date_naive();

        // Calculate the start of the current week (Monday)
        let days_since_monday = today.weekday().num_days_from_monday() as i64;
        let week_start = today - Duration::days(days_since_monday);
        let week_end = week_start + Duration::days(6);

        // 1. Get weekly conversation minutes from learn_daily_stats
        let weekly_stats: Vec<DailyStat> = learn_daily_stats::table
            .filter(learn_daily_stats::user_id.eq(user_id))
            .filter(learn_daily_stats::stat_date.ge(week_start))
            .filter(learn_daily_stats::stat_date.le(week_end))
            .load::<DailyStat>(conn)?;

        let weekly_conversation_minutes: i32 = weekly_stats
            .iter()
            .map(|s| s.minutes_studied.unwrap_or(0))
            .sum();

        // Build weekly minutes array (Monday to Sunday)
        let mut weekly_minutes = Vec::new();
        for i in 0..7 {
            let date = week_start + Duration::days(i);
            let minutes = weekly_stats
                .iter()
                .find(|s| s.stat_date == date)
                .map(|s| s.minutes_studied.unwrap_or(0))
                .unwrap_or(0);
            weekly_minutes.push(WeeklyMinutes {
                day: i as i32,
                date: date.format("%Y-%m-%d").to_string(),
                minutes,
            });
        }

        // 2. Count mastered vocabulary (mastery_level >= 4)
        let mastered_vocabulary_count: i64 = learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .filter(learn_vocabularies::mastery_level.ge(4))
            .count()
            .get_result(conn)?;

        // 3. Count pending review words
        let pending_review_count: i64 = learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .filter(
                learn_vocabularies::next_review_at
                    .is_null()
                    .or(learn_vocabularies::next_review_at.le(now)),
            )
            .count()
            .get_result(conn)?;

        // 4. Check if user has any learning data
        let has_chats: i64 = learn_chats::table
            .filter(learn_chats::user_id.eq(user_id))
            .count()
            .get_result(conn)?;

        let has_vocab: i64 = learn_vocabularies::table
            .filter(learn_vocabularies::user_id.eq(user_id))
            .count()
            .get_result(conn)?;

        let has_data = has_chats > 0 || has_vocab > 0 || weekly_conversation_minutes > 0;

        Ok::<_, diesel::result::Error>(LearnSummary {
            has_data,
            weekly_conversation_minutes,
            mastered_vocabulary_count,
            pending_review_count,
            weekly_minutes,
        })
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get learn summary"))?;

    res.render(Json(summary));
    Ok(())
}
