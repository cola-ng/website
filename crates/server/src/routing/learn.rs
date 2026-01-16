use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use salvo::http::StatusCode;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{AppResult, DepotExt, hoops};

pub fn router() -> Router {
    Router::with_path("learn")
        .hoop(hoops::require_auth)
        .push(
            Router::with_path("issue-words")
                .get(list_issue_words)
                .post(create_issue_word)
                .delete(reset_issue_words),
        )
        .push(
            Router::with_path("sessions")
                .get(list_sessions)
                .post(create_session)
                .delete(reset_sessions)
                .push(Router::with_path("{session_id}").patch(update_session)),
        )
        .push(
            Router::with_path("conversations")
                .get(list_conversations)
                .post(create_conversation)
                .delete(reset_conversations),
        )
        .push(
            Router::with_path("conversation-annotations")
                .get(list_conversation_annotations)
                .post(create_conversation_annotation)
                .delete(reset_conversation_annotations),
        )
        .push(
            Router::with_path("word-practices")
                .get(list_word_practices)
                .post(create_word_practice)
                .delete(reset_word_practices),
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

#[derive(Deserialize)]
pub struct CreateIssueWordRequest {
    word: String,
    issue_type: String,
    description_en: Option<String>,
    description_zh: Option<String>,
    context: Option<String>,
}

#[handler]
pub async fn list_issue_words(
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
pub async fn list_conversations(
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

// ============================================================================
// User Vocabulary API (user-specific)
// ============================================================================

#[derive(Deserialize)]
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

#[derive(Deserialize)]
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
// Conversation Annotations API (user-specific)
// ============================================================================

#[derive(Deserialize)]
pub struct CreateConversationAnnotationRequest {
    conversation_id: i64,
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
pub async fn list_conversation_annotations(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let conversation_id_param = req.query::<i64>("conversation_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let annotations: Vec<ConversationAnnotation> = with_conn(move |conn| {
        let mut query = learn_conversation_annotations::table
            .filter(learn_conversation_annotations::user_id.eq(user_id))
            .order(learn_conversation_annotations::created_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(cid) = conversation_id_param {
            query = query.filter(learn_conversation_annotations::conversation_id.eq(cid));
        }

        query.load::<ConversationAnnotation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list annotations"))?;

    res.render(Json(annotations));
    Ok(())
}

#[handler]
pub async fn create_conversation_annotation(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateConversationAnnotationRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let new_annotation = NewConversationAnnotation {
        user_id,
        conversation_id: input.conversation_id,
        annotation_type: input.annotation_type,
        start_position: input.start_position,
        end_position: input.end_position,
        original_text: input.original_text,
        suggested_text: input.suggested_text,
        description_en: input.description_en,
        description_zh: input.description_zh,
        severity: input.severity,
    };

    let annotation: ConversationAnnotation = with_conn(move |conn| {
        diesel::insert_into(learn_conversation_annotations::table)
            .values(&new_annotation)
            .get_result::<ConversationAnnotation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create annotation"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(annotation));
    Ok(())
}

// ============================================================================
// Word Practices API (user-specific)
// ============================================================================

#[derive(Deserialize)]
pub struct CreateWordPracticeRequest {
    word_id: i64,
    session_id: String,
    success_level: Option<i32>,
    notes: Option<String>,
}

#[handler]
pub async fn list_word_practices(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let word_id_param = req.query::<i64>("word_id");
    let session_id_param = req.query::<String>("session_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let practices: Vec<WordPracticeLog> = with_conn(move |conn| {
        let mut query = learn_word_practices::table
            .filter(learn_word_practices::user_id.eq(user_id))
            .order(learn_word_practices::practiced_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(wid) = word_id_param {
            query = query.filter(learn_word_practices::word_id.eq(wid));
        }
        if let Some(sid) = session_id_param {
            query = query.filter(learn_word_practices::session_id.eq(sid));
        }

        query.load::<WordPracticeLog>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list word practices"))?;

    res.render(Json(practices));
    Ok(())
}

#[handler]
pub async fn create_word_practice(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let input: CreateWordPracticeRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    let new_practice = NewWordPracticeLog {
        user_id,
        word_id: input.word_id,
        session_id: input.session_id,
        success_level: input.success_level,
        notes: input.notes,
    };

    let practice: WordPracticeLog = with_conn(move |conn| {
        diesel::insert_into(learn_word_practices::table)
            .values(&new_practice)
            .get_result::<WordPracticeLog>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to create word practice"))?;

    res.status_code(StatusCode::CREATED);
    res.render(Json(practice));
    Ok(())
}

// ============================================================================
// Reading Practices API (user-specific)
// ============================================================================

#[derive(Deserialize)]
pub struct CreateReadPracticeRequest {
    sentence_id: i64,
    session_id: String,
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
    let session_id_param = req.query::<String>("session_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let practices: Vec<ReadingPracticeAttempt> = with_conn(move |conn| {
        let mut query = learn_read_practices::table
            .filter(learn_read_practices::user_id.eq(user_id))
            .order(learn_read_practices::attempted_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(sid) = sentence_id_param {
            query = query.filter(learn_read_practices::sentence_id.eq(sid));
        }
        if let Some(sess_id) = session_id_param {
            query = query.filter(learn_read_practices::session_id.eq(sess_id));
        }

        query.load::<ReadingPracticeAttempt>(conn)
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

    let new_practice = NewReadingPracticeAttempt {
        user_id,
        sentence_id: input.sentence_id,
        session_id: input.session_id,
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

    let practice: ReadingPracticeAttempt = with_conn(move |conn| {
        diesel::insert_into(learn_read_practices::table)
            .values(&new_practice)
            .get_result::<ReadingPracticeAttempt>(conn)
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

#[derive(Deserialize)]
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
pub async fn reset_sessions(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(learn_sessions::table.filter(learn_sessions::user_id.eq(user_id)))
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset sessions"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_sessions".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_conversations(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(
            learn_conversations::table.filter(learn_conversations::user_id.eq(user_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset conversations"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_conversations".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_conversation_annotations(
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(
            learn_conversation_annotations::table
                .filter(learn_conversation_annotations::user_id.eq(user_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| {
        StatusError::internal_server_error().brief("failed to reset conversation annotations")
    })?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_conversation_annotations".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn reset_word_practices(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let deleted_count = with_conn(move |conn| {
        diesel::delete(
            learn_word_practices::table.filter(learn_word_practices::user_id.eq(user_id)),
        )
        .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to reset word practices"))?;

    res.render(Json(ResetResponse {
        deleted_count,
        table: "learn_word_practices".to_string(),
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

        // Conversation annotations (depends on conversations)
        let count = diesel::delete(
            learn_conversation_annotations::table
                .filter(learn_conversation_annotations::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_conversation_annotations".to_string(),
        });

        // Word practices (depends on issue_words)
        let count = diesel::delete(
            learn_word_practices::table.filter(learn_word_practices::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_word_practices".to_string(),
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

        // Conversations
        let count = diesel::delete(
            learn_conversations::table.filter(learn_conversations::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_conversations".to_string(),
        });

        // Sessions
        let count = diesel::delete(
            learn_sessions::table.filter(learn_sessions::user_id.eq(user_id)),
        )
        .execute(conn)?;
        tables_reset.push(ResetResponse {
            deleted_count: count,
            table: "learn_sessions".to_string(),
        });

        // Issue words
        let count = diesel::delete(
            learn_issue_words::table.filter(learn_issue_words::user_id.eq(user_id)),
        )
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
        let count = diesel::delete(
            learn_daily_stats::table.filter(learn_daily_stats::user_id.eq(user_id)),
        )
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
        let count = diesel::delete(
            learn_suggestions::table.filter(learn_suggestions::user_id.eq(user_id)),
        )
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
