-- Migration: Learning Content
-- Based on desktop learning_companion.db structure
-- All user-specific tables include user_id foreign key

-- ============================================================================
-- SCENARIOS & SCENES (Shared content - no user_id)
-- ============================================================================

CREATE TABLE IF NOT EXISTS scenarios (
    id BIGSERIAL PRIMARY KEY,
    name_en TEXT NOT NULL,
    name_zh TEXT NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    icon_emoji TEXT,
    difficulty_level TEXT CHECK(difficulty_level IN ('beginner', 'intermediate', 'advanced')) DEFAULT 'intermediate',
    category TEXT,
    display_order INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(name_en)
);

CREATE INDEX IF NOT EXISTS idx_scenarios_active ON scenarios(is_active, display_order);

CREATE TABLE IF NOT EXISTS scene_dialogues (
    id BIGSERIAL PRIMARY KEY,
    scenario_id BIGINT NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    title_en TEXT NOT NULL,
    title_zh TEXT NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    total_turns INTEGER DEFAULT 0,
    estimated_duration_seconds INTEGER,
    difficulty_level TEXT CHECK(difficulty_level IN ('beginner', 'intermediate', 'advanced')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_scene_dialogues_scenario ON scene_dialogues(scenario_id);

CREATE TABLE IF NOT EXISTS dialogue_turns (
    id BIGSERIAL PRIMARY KEY,
    scene_dialogue_id BIGINT NOT NULL REFERENCES scene_dialogues(id) ON DELETE CASCADE,
    turn_number INTEGER NOT NULL,
    speaker_role TEXT NOT NULL,
    speaker_name TEXT,
    content_en TEXT NOT NULL,
    content_zh TEXT NOT NULL,
    audio_path TEXT,
    phonetic_transcription TEXT,
    key_phrases JSONB,
    notes TEXT,
    UNIQUE(scene_dialogue_id, turn_number)
);

CREATE INDEX IF NOT EXISTS idx_dialogue_turns_scene ON dialogue_turns(scene_dialogue_id, turn_number);

-- ============================================================================
-- CLASSIC DIALOGUES (Shared content - no user_id)
-- ============================================================================

CREATE TABLE IF NOT EXISTS classic_dialogue_sources (
    id BIGSERIAL PRIMARY KEY,
    source_type TEXT NOT NULL CHECK(source_type IN ('movie', 'tv_show', 'ted_talk', 'documentary', 'other')),
    title TEXT NOT NULL,
    year INTEGER,
    description_en TEXT,
    description_zh TEXT,
    thumbnail_url TEXT,
    imdb_id TEXT,
    difficulty_level TEXT CHECK(difficulty_level IN ('beginner', 'intermediate', 'advanced')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(source_type, title)
);

CREATE INDEX IF NOT EXISTS idx_classic_sources_type ON classic_dialogue_sources(source_type);

CREATE TABLE IF NOT EXISTS classic_dialogue_clips (
    id BIGSERIAL PRIMARY KEY,
    source_id BIGINT NOT NULL REFERENCES classic_dialogue_sources(id) ON DELETE CASCADE,
    clip_title_en TEXT NOT NULL,
    clip_title_zh TEXT NOT NULL,
    start_time_seconds INTEGER,
    end_time_seconds INTEGER,
    video_url TEXT,
    transcript_en TEXT NOT NULL,
    transcript_zh TEXT NOT NULL,
    key_vocabulary JSONB,
    cultural_notes TEXT,
    grammar_points JSONB,
    difficulty_vocab INTEGER DEFAULT 3 CHECK(difficulty_vocab BETWEEN 1 AND 5),
    difficulty_speed INTEGER DEFAULT 3 CHECK(difficulty_speed BETWEEN 1 AND 5),
    difficulty_slang INTEGER DEFAULT 3 CHECK(difficulty_slang BETWEEN 1 AND 5),
    popularity_score INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_classic_clips_source ON classic_dialogue_clips(source_id);
CREATE INDEX IF NOT EXISTS idx_classic_clips_popularity ON classic_dialogue_clips(popularity_score DESC);

-- ============================================================================
-- READING PRACTICE (Shared content - no user_id)
-- ============================================================================

CREATE TABLE IF NOT EXISTS reading_exercises (
    id BIGSERIAL PRIMARY KEY,
    title_en TEXT NOT NULL,
    title_zh TEXT NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    difficulty_level TEXT CHECK(difficulty_level IN ('beginner', 'intermediate', 'advanced')),
    exercise_type TEXT CHECK(exercise_type IN ('sentence', 'paragraph', 'dialogue', 'tongue_twister')) DEFAULT 'sentence',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS reading_sentences (
    id BIGSERIAL PRIMARY KEY,
    exercise_id BIGINT NOT NULL REFERENCES reading_exercises(id) ON DELETE CASCADE,
    sentence_order INTEGER NOT NULL,
    content_en TEXT NOT NULL,
    content_zh TEXT NOT NULL,
    phonetic_transcription TEXT,
    native_audio_path TEXT,
    focus_sounds JSONB,
    common_mistakes JSONB,
    UNIQUE(exercise_id, sentence_order)
);

CREATE INDEX IF NOT EXISTS idx_reading_sentences_exercise ON reading_sentences(exercise_id, sentence_order);

-- ============================================================================
-- KEY PHRASES (Shared content - no user_id)
-- ============================================================================

CREATE TABLE IF NOT EXISTS key_phrases (
    id BIGSERIAL PRIMARY KEY,
    phrase_en TEXT NOT NULL,
    phrase_zh TEXT NOT NULL,
    phonetic_transcription TEXT,
    usage_context TEXT,
    example_sentence_en TEXT,
    example_sentence_zh TEXT,
    category TEXT,
    formality_level TEXT CHECK(formality_level IN ('casual', 'neutral', 'formal')),
    frequency_score INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(phrase_en)
);

CREATE INDEX IF NOT EXISTS idx_key_phrases_category ON key_phrases(category);

-- ============================================================================
-- USER-SPECIFIC TABLES (All include user_id)
-- ============================================================================

-- Table: issue_words - Words that the user has problems with
CREATE TABLE IF NOT EXISTS issue_words (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    word TEXT NOT NULL,
    issue_type TEXT NOT NULL CHECK(issue_type IN ('pronunciation', 'usage', 'unfamiliar', 'grammar')),
    description_en TEXT,
    description_zh TEXT,
    last_picked_at TIMESTAMPTZ,
    pick_count INTEGER NOT NULL DEFAULT 0,
    next_review_at TIMESTAMPTZ,
    review_interval_days INTEGER DEFAULT 1,
    difficulty_level INTEGER DEFAULT 1 CHECK(difficulty_level BETWEEN 1 AND 5),
    context TEXT,
    audio_timestamp INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, word, issue_type)
);

CREATE INDEX IF NOT EXISTS idx_issue_words_user ON issue_words(user_id, next_review_at);
CREATE INDEX IF NOT EXISTS idx_issue_words_review ON issue_words(next_review_at) WHERE next_review_at IS NOT NULL;

-- Table: learning_sessions - Track overall learning sessions
CREATE TABLE IF NOT EXISTS learning_sessions (
    id BIGSERIAL PRIMARY KEY,
    session_id TEXT NOT NULL UNIQUE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_type TEXT CHECK(session_type IN ('free_talk', 'scenario', 'classic_dialogue', 'reading', 'review', 'assistant')),
    scenario_id BIGINT REFERENCES scenarios(id) ON DELETE SET NULL,
    scene_dialogue_id BIGINT REFERENCES scene_dialogues(id) ON DELETE SET NULL,
    classic_clip_id BIGINT REFERENCES classic_dialogue_clips(id) ON DELETE SET NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ended_at TIMESTAMPTZ,
    duration_seconds INTEGER,
    total_words_spoken INTEGER DEFAULT 0,
    average_wpm REAL,
    error_count INTEGER DEFAULT 0,
    correction_count INTEGER DEFAULT 0,
    notes TEXT,
    ai_summary_en TEXT,
    ai_summary_zh TEXT
);

CREATE INDEX IF NOT EXISTS idx_learning_sessions_user ON learning_sessions(user_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_learning_sessions_type ON learning_sessions(session_type, started_at);

-- Table: conversations - Stores all conversation history
CREATE TABLE IF NOT EXISTS conversations (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    speaker TEXT NOT NULL CHECK(speaker IN ('user', 'teacher')),
    use_lang TEXT NOT NULL CHECK(use_lang IN ('en', 'zh')),
    content_en TEXT NOT NULL,
    content_zh TEXT NOT NULL,
    audio_path TEXT,
    duration_ms INTEGER,
    words_per_minute REAL,
    pause_count INTEGER,
    hesitation_count INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_conversations_user_session ON conversations(user_id, session_id, created_at);
CREATE INDEX IF NOT EXISTS idx_conversations_created ON conversations(created_at);

-- Table: conversation_annotations - Stores annotations and issues found in conversations
CREATE TABLE IF NOT EXISTS conversation_annotations (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    annotation_type TEXT NOT NULL CHECK(
        annotation_type IN ('pronunciation_error', 'grammar_error', 'word_choice',
                           'fluency_issue', 'suggestion', 'correction')
    ),
    start_position INTEGER,
    end_position INTEGER,
    original_text TEXT,
    suggested_text TEXT,
    description_en TEXT,
    description_zh TEXT,
    severity TEXT CHECK(severity IN ('low', 'medium', 'high')) DEFAULT 'medium',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_annotations_conversation ON conversation_annotations(conversation_id);
CREATE INDEX IF NOT EXISTS idx_annotations_user ON conversation_annotations(user_id, annotation_type);

-- Table: word_practice_log - Logs each time a word is practiced
CREATE TABLE IF NOT EXISTS word_practice_log (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    word_id BIGINT NOT NULL REFERENCES issue_words(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    success_level INTEGER CHECK(success_level BETWEEN 1 AND 5),
    notes TEXT,
    practiced_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_practice_log_word ON word_practice_log(word_id, practiced_at);
CREATE INDEX IF NOT EXISTS idx_practice_log_user_session ON word_practice_log(user_id, session_id);

-- Table: reading_practice_attempts - Log of user's reading practice attempts
CREATE TABLE IF NOT EXISTS reading_practice_attempts (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sentence_id BIGINT NOT NULL REFERENCES reading_sentences(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    user_audio_path TEXT,
    pronunciation_score INTEGER CHECK(pronunciation_score BETWEEN 0 AND 100),
    fluency_score INTEGER CHECK(fluency_score BETWEEN 0 AND 100),
    intonation_score INTEGER CHECK(intonation_score BETWEEN 0 AND 100),
    overall_score INTEGER CHECK(overall_score BETWEEN 0 AND 100),
    detected_errors JSONB,
    ai_feedback_en TEXT,
    ai_feedback_zh TEXT,
    waveform_data JSONB,
    attempted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_reading_attempts_sentence ON reading_practice_attempts(sentence_id, attempted_at);
CREATE INDEX IF NOT EXISTS idx_reading_attempts_user_session ON reading_practice_attempts(user_id, session_id);

-- Table: user_achievements - Track user achievements and milestones
CREATE TABLE IF NOT EXISTS user_achievements (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    achievement_type TEXT NOT NULL,
    achievement_name TEXT NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    metadata JSONB,
    earned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, achievement_type, achievement_name)
);

CREATE INDEX IF NOT EXISTS idx_achievements_user ON user_achievements(user_id, earned_at DESC);

-- Table: daily_stats - Daily learning statistics
CREATE TABLE IF NOT EXISTS daily_stats (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stat_date DATE NOT NULL,
    minutes_studied INTEGER DEFAULT 0,
    words_practiced INTEGER DEFAULT 0,
    sessions_completed INTEGER DEFAULT 0,
    errors_corrected INTEGER DEFAULT 0,
    new_words_learned INTEGER DEFAULT 0,
    review_words_count INTEGER DEFAULT 0,
    UNIQUE(user_id, stat_date)
);

CREATE INDEX IF NOT EXISTS idx_daily_stats_user_date ON daily_stats(user_id, stat_date DESC);

-- Table: user_vocabulary - Track user's vocabulary mastery
CREATE TABLE IF NOT EXISTS user_vocabulary (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    word TEXT NOT NULL,
    word_zh TEXT,
    mastery_level INTEGER DEFAULT 1 CHECK(mastery_level BETWEEN 1 AND 5),
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_practiced_at TIMESTAMPTZ,
    practice_count INTEGER DEFAULT 0,
    correct_count INTEGER DEFAULT 0,
    next_review_at TIMESTAMPTZ,
    UNIQUE(user_id, word)
);

CREATE INDEX IF NOT EXISTS idx_user_vocab_user ON user_vocabulary(user_id, next_review_at);
CREATE INDEX IF NOT EXISTS idx_user_vocab_review ON user_vocabulary(next_review_at) WHERE next_review_at IS NOT NULL;

-- Table: assistant_conversations - Track real-time assistant usage
CREATE TABLE IF NOT EXISTS assistant_conversations (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    external_app TEXT,
    message_count INTEGER DEFAULT 0,
    ai_suggestions_count INTEGER DEFAULT 0,
    grammar_corrections_count INTEGER DEFAULT 0,
    translations_count INTEGER DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ended_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_assistant_convos_user ON assistant_conversations(user_id, started_at DESC);

-- Table: assistant_suggestions - Log AI suggestions made during real-time assistance
CREATE TABLE IF NOT EXISTS assistant_suggestions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conversation_id BIGINT NOT NULL REFERENCES assistant_conversations(id) ON DELETE CASCADE,
    suggestion_type TEXT CHECK(suggestion_type IN ('response', 'correction', 'translation', 'vocabulary')),
    suggested_text TEXT NOT NULL,
    was_accepted BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_assistant_suggestions_convo ON assistant_suggestions(conversation_id, created_at);
