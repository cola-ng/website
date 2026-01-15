use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use salvo::http::StatusCode;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::{schema, with_conn};
use crate::models::learn::*;

// ============================================================================
// Helper functions
// ============================================================================

fn bad_request(message: &str) -> StatusError {
    StatusError::bad_request().brief(message)
}

fn get_user_id(depot: &Depot) -> Result<i64, StatusError> {
    depot
        .get::<i64>("user_id")
        .copied()
        .map_err(|_| StatusError::unauthorized().brief("missing user"))
}

// ============================================================================
// Scenes API (shared content)
// ============================================================================

#[handler]
pub async fn list_scenes(req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
    let category_param = req.query::<String>("category");
    let difficulty = req.query::<String>("difficulty");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let scenes: Vec<Scene> = with_conn(move |conn| {
        use schema::scenes::dsl::*;
        let mut query = scenes
            .filter(is_active.eq(true))
            .order(display_order.asc())
            .limit(limit)
            .into_boxed();

        if let Some(cat) = category_param {
            query = query.filter(category.eq(cat));
        }
        if let Some(diff) = difficulty {
            query = query.filter(difficulty_level.eq(diff));
        }

        query.load::<Scene>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list scenes"))?;

    res.render(Json(scenes));
    Ok(())
}

#[handler]
pub async fn get_scene(req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
    let scene_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| bad_request("missing id"))?;

    let scene: Scene = with_conn(move |conn| {
        use schema::scenes::dsl::*;
        scenes.filter(id.eq(scene_id)).first::<Scene>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("scene not found"))?;

    res.render(Json(scene));
    Ok(())
}

#[handler]
pub async fn get_asset_dialogues(
    req: &mut Request,
    res: &mut Response,
) -> Result<(), StatusError> {
    let scene_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| bad_request("missing id"))?;

    let dialogues: Vec<SceneDialogue> = with_conn(move |conn| {
        use schema::asset_dialogues::dsl::*;
        asset_dialogues
            .filter(scene_id.eq(scene_id))
            .load::<SceneDialogue>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list dialogues"))?;

    res.render(Json(dialogues));
    Ok(())
}

#[handler]
pub async fn get_asset_dialogue_turns(req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
    let dialogue_id: i64 = req
        .param::<i64>("dialogue_id")
        .ok_or_else(|| bad_request("missing dialogue_id"))?;

    let turns: Vec<DialogueTurn> = with_conn(move |conn| {
        use schema::asset_dialogue_turns::dsl::*;
        asset_dialogue_turns
            .filter(dialogue_id.eq(dialogue_id))
            .order(turn_number.asc())
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
pub async fn list_classic_sources(
    req: &mut Request,
    res: &mut Response,
) -> Result<(), StatusError> {
    let source_type_param = req.query::<String>("type");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let sources: Vec<ClassicDialogueSource> = with_conn(move |conn| {
        use schema::asset_classic_sources::dsl::*;
        let mut query = asset_classic_sources.limit(limit).into_boxed();

        if let Some(st) = source_type_param {
            query = query.filter(source_type.eq(st));
        }

        query.load::<ClassicDialogueSource>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list sources"))?;

    res.render(Json(sources));
    Ok(())
}

#[handler]
pub async fn list_classic_clips(req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
    let source_id_param = req.query::<i64>("source_id");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let clips: Vec<ClassicDialogueClip> = with_conn(move |conn| {
        use schema::asset_classic_clips::dsl::*;
        let mut query = asset_classic_clips
            .order(popularity_score.desc())
            .limit(limit)
            .into_boxed();

        if let Some(sid) = source_id_param {
            query = query.filter(source_id.eq(sid));
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
pub async fn list_asset_read_exercises(
    req: &mut Request,
    res: &mut Response,
) -> Result<(), StatusError> {
    let difficulty = req.query::<String>("difficulty");
    let exercise_type_param = req.query::<String>("type");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let exercises: Vec<ReadingExercise> = with_conn(move |conn| {
        use schema::asset_read_exercises::dsl::*;
        let mut query = asset_read_exercises.limit(limit).into_boxed();

        if let Some(diff) = difficulty {
            query = query.filter(difficulty_level.eq(diff));
        }
        if let Some(et) = exercise_type_param {
            query = query.filter(exercise_type.eq(et));
        }

        query.load::<ReadingExercise>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list exercises"))?;

    res.render(Json(exercises));
    Ok(())
}

#[handler]
pub async fn get_asset_read_sentences(
    req: &mut Request,
    res: &mut Response,
) -> Result<(), StatusError> {
    let exercise_id_param: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| bad_request("missing id"))?;

    let sentences: Vec<ReadingSentence> = with_conn(move |conn| {
        use schema::asset_read_sentences::dsl::*;
        asset_read_sentences
            .filter(exercise_id.eq(exercise_id_param))
            .order(sentence_order.asc())
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
pub async fn list_asset_phrases(req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
    let category_param = req.query::<String>("category");
    let formality = req.query::<String>("formality");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let phrases: Vec<KeyPhrase> = with_conn(move |conn| {
        use schema::asset_phrases::dsl::*;
        let mut query = asset_phrases.limit(limit).into_boxed();

        if let Some(cat) = category_param {
            query = query.filter(category.eq(cat));
        }
        if let Some(form) = formality {
            query = query.filter(formality_level.eq(form));
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
) -> Result<(), StatusError> {
    let user_id = get_user_id(depot)?;
    let due_only = req.query::<bool>("due_only").unwrap_or(false);
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let words: Vec<IssueWord> = with_conn(move |conn| {
        use schema::learn_issue_words::dsl::*;
        let mut query = learn_issue_words
            .filter(user_id.eq(user_id))
            .order(created_at.desc())
            .limit(limit)
            .into_boxed();

        if due_only {
            let now = Utc::now();
            query = query.filter(
                next_review_at
                    .is_null()
                    .or(next_review_at.le(now)),
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
) -> Result<(), StatusError> {
    let user_id = get_user_id(depot)?;
    let input: CreateIssueWordRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;

    if input.word.trim().is_empty() || input.issue_type.trim().is_empty() {
        return Err(bad_request("word and issue_type are required"));
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
        use schema::learn_issue_words::dsl::*;
        diesel::insert_into(learn_issue_words)
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
) -> Result<(), StatusError> {
    let user_id = get_user_id(depot)?;
    let session_type_param = req.query::<String>("type");
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let sessions: Vec<LearningSession> = with_conn(move |conn| {
        use schema::learn_sessions::dsl::*;
        let mut query = learn_sessions
            .filter(user_id.eq(user_id))
            .order(started_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(st) = session_type_param {
            query = query.filter(session_type.eq(st));
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
) -> Result<(), StatusError> {
    let user_id = get_user_id(depot)?;
    let input: CreateSessionRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;

    if input.session_id.trim().is_empty() {
        return Err(bad_request("session_id is required"));
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
        use schema::learn_sessions::dsl::*;
        diesel::insert_into(learn_sessions)
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
) -> Result<(), StatusError> {
    let user_id = get_user_id(depot)?;
    let session_id_param: String = req
        .param::<String>("session_id")
        .ok_or_else(|| bad_request("missing session_id"))?;

    let input: UpdateSessionRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;

    let ended_at_parsed = input
        .ended_at
        .map(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)))
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
        use schema::learn_sessions::dsl::*;
        diesel::update(
            learn_sessions
                .filter(session_id.eq(session_id_param))
                .filter(user_id.eq(user_id)),
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
) -> Result<(), StatusError> {
    let user_id = get_user_id(depot)?;
    let session_id_param = req.query::<String>("session_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let convos: Vec<Conversation> = with_conn(move |conn| {
        use schema::learn_conversations::dsl::*;
        let mut query = learn_conversations
            .filter(user_id.eq(user_id))
            .order(created_at.desc())
            .limit(limit)
            .into_boxed();

        if let Some(sid) = session_id_param {
            query = query.filter(session_id.eq(sid));
        }

        query.load::<Conversation>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list learn_conversations"))?;

    res.render(Json(convos));
    Ok(())
}

#[handler]
pub async fn create_conversation(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let user_id = get_user_id(depot)?;
    let input: CreateConversationRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;

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
        use schema::learn_conversations::dsl::*;
        diesel::insert_into(learn_conversations)
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
) -> Result<(), StatusError> {
    let user_id = get_user_id(depot)?;
    let due_only = req.query::<bool>("due_only").unwrap_or(false);
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let vocab: Vec<UserVocabulary> = with_conn(move |conn| {
        use schema::learn_vocabularies::dsl::*;
        let mut query = learn_vocabularies
            .filter(user_id.eq(user_id))
            .order(first_seen_at.desc())
            .limit(limit)
            .into_boxed();

        if due_only {
            let now = Utc::now();
            query = query.filter(
                next_review_at
                    .is_null()
                    .or(next_review_at.le(now)),
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
) -> Result<(), StatusError> {
    let user_id = get_user_id(depot)?;
    let input: CreateVocabularyRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;

    if input.word.trim().is_empty() {
        return Err(bad_request("word is required"));
    }

    let new_vocab = NewUserVocabulary {
        user_id,
        word: input.word,
        word_zh: input.word_zh,
    };

    let vocab: UserVocabulary = with_conn(move |conn| {
        use schema::learn_vocabularies::dsl::*;
        diesel::insert_into(learn_vocabularies)
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
pub async fn list_learn_daily_stats(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let user_id = get_user_id(depot)?;
    let limit = req.query::<i64>("limit").unwrap_or(30).clamp(1, 365);

    let stats: Vec<DailyStat> = with_conn(move |conn| {
        use schema::learn_daily_stats::dsl::*;
        learn_daily_stats
            .filter(user_id.eq(user_id))
            .order(stat_date.desc())
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
) -> Result<(), StatusError> {
    let user_id = get_user_id(depot)?;
    let input: UpsertDailyStatRequest = req
        .parse_json()
        .await
        .map_err(|_| bad_request("invalid json"))?;

    let date = NaiveDate::parse_from_str(&input.stat_date, "%Y-%m-%d")
        .map_err(|_| bad_request("invalid date format, use YYYY-MM-DD"))?;

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
        use schema::learn_daily_stats::dsl::*;
        diesel::insert_into(learn_daily_stats)
            .values(&new_stat)
            .on_conflict((user_id, stat_date))
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
pub async fn list_achievements(
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let user_id = get_user_id(depot)?;

    let achievements: Vec<UserAchievement> = with_conn(move |conn| {
        use schema::learn_achievements::dsl::*;
        learn_achievements
            .filter(user_id.eq(user_id))
            .order(earned_at.desc())
            .load::<UserAchievement>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list achievements"))?;

    res.render(Json(achievements));
    Ok(())
}

// ============================================================================
// Router
// ============================================================================

pub fn router(auth_hoop: impl Handler) -> Router {
    // Shared content routes (no auth required for reading)
    let shared = Router::new()
        .push(
            Router::with_path("scenes")
                .get(list_scenes)
                .push(Router::with_path("{id}").get(get_scene))
                .push(Router::with_path("{id}/dialogues").get(get_asset_dialogues)),
        )
        .push(Router::with_path("dialogues/{dialogue_id}/turns").get(get_asset_dialogue_turns))
        .push(Router::with_path("classic-sources").get(list_classic_sources))
        .push(Router::with_path("classic-clips").get(list_classic_clips))
        .push(
            Router::with_path("reading-exercises")
                .get(list_asset_read_exercises)
                .push(Router::with_path("{id}/sentences").get(get_asset_read_sentences)),
        )
        .push(Router::with_path("key-phrases").get(list_asset_phrases));

    // User-specific routes (auth required)
    let user_learning = Router::with_path("learning")
        .hoop(auth_hoop)
        .push(
            Router::with_path("issue-words")
                .get(list_learn_issue_words)
                .post(create_issue_word),
        )
        .push(
            Router::with_path("sessions")
                .get(list_sessions)
                .post(create_session)
                .push(Router::with_path("{session_id}").patch(update_session)),
        )
        .push(
            Router::with_path("learn_conversations")
                .get(list_learn_conversations)
                .post(create_conversation),
        )
        .push(
            Router::with_path("vocabulary")
                .get(list_vocabulary)
                .post(create_vocabulary),
        )
        .push(
            Router::with_path("daily-stats")
                .get(list_learn_daily_stats)
                .post(upsert_daily_stat),
        )
        .push(Router::with_path("achievements").get(list_achievements));

    Router::new().push(shared).push(user_learning)
}
