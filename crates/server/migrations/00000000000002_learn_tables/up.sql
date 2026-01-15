-- ============================================================================
-- USER-SPECIFIC TABLES (All include user_id)
-- ============================================================================

-- Table: learn_issue_words - Words that the user has problems with
CREATE TABLE IF NOT EXISTS learn_issue_words (
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

CREATE INDEX IF NOT EXISTS idx_learn_issue_words_user ON learn_issue_words(user_id, next_review_at);
CREATE INDEX IF NOT EXISTS idx_learn_issue_words_review ON learn_issue_words(next_review_at) WHERE next_review_at IS NOT NULL;

-- Table: learn_sessions - Track overall learning sessions
CREATE TABLE IF NOT EXISTS learn_sessions (
    id BIGSERIAL PRIMARY KEY,
    session_id TEXT NOT NULL UNIQUE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_type TEXT CHECK(session_type IN ('free_talk', 'scene', 'classic_dialogue', 'reading', 'review', 'assistant')),
    scene_id BIGINT REFERENCES scenes(id) ON DELETE SET NULL,
    dialogue_id BIGINT REFERENCES asset_dialogues(id) ON DELETE SET NULL,
    classic_clip_id BIGINT REFERENCES asset_classic_clips(id) ON DELETE SET NULL,
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

CREATE INDEX IF NOT EXISTS idx_learn_sessions_user ON learn_sessions(user_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_learn_sessions_type ON learn_sessions(session_type, started_at);

-- Table: learn_conversations - Stores all conversation history
CREATE TABLE IF NOT EXISTS learn_conversations (
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

CREATE INDEX IF NOT EXISTS idx_learn_conversations_user_session ON learn_conversations(user_id, session_id, created_at);
CREATE INDEX IF NOT EXISTS idx_learn_conversations_created ON learn_conversations(created_at);

-- Table: learn_conversation_annotations - Stores annotations and issues found in learn_conversations
CREATE TABLE IF NOT EXISTS learn_conversation_annotations (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conversation_id BIGINT NOT NULL REFERENCES learn_conversations(id) ON DELETE CASCADE,
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

CREATE INDEX IF NOT EXISTS idx_annotations_conversation ON learn_conversation_annotations(conversation_id);
CREATE INDEX IF NOT EXISTS idx_annotations_user ON learn_conversation_annotations(user_id, annotation_type);

-- Table: learn_word_practices - Logs each time a word is practiced
CREATE TABLE IF NOT EXISTS learn_word_practices (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    word_id BIGINT NOT NULL REFERENCES learn_issue_words(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    success_level INTEGER CHECK(success_level BETWEEN 1 AND 5),
    notes TEXT,
    practiced_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_learn_practices_word ON learn_word_practices(word_id, practiced_at);
CREATE INDEX IF NOT EXISTS idx_learn_practices_user_session ON learn_word_practices(user_id, session_id);

-- Table: learn_read_practices - Log of user's reading practice attempts
CREATE TABLE IF NOT EXISTS learn_read_practices (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sentence_id BIGINT NOT NULL REFERENCES asset_read_sentences(id) ON DELETE CASCADE,
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

CREATE INDEX IF NOT EXISTS idx_read_ractices_sentence ON learn_read_practices(sentence_id, attempted_at);
CREATE INDEX IF NOT EXISTS idx_read_ractices_user_session ON learn_read_practices(user_id, session_id);

-- Table: learn_achievements - Track user achievements and milestones
CREATE TABLE IF NOT EXISTS learn_achievements (
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

CREATE INDEX IF NOT EXISTS idx_achievements_user ON learn_achievements(user_id, earned_at DESC);

-- Table: learn_daily_stats - Daily learning statistics
CREATE TABLE IF NOT EXISTS learn_daily_stats (
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

CREATE INDEX IF NOT EXISTS idx_learn_daily_stats_user_date ON learn_daily_stats(user_id, stat_date DESC);

-- Table: learn_vocabularies - Track user's vocabulary mastery
CREATE TABLE IF NOT EXISTS learn_vocabularies (
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

CREATE INDEX IF NOT EXISTS idx_user_vocab_user ON learn_vocabularies(user_id, next_review_at);
CREATE INDEX IF NOT EXISTS idx_user_vocab_review ON learn_vocabularies(next_review_at) WHERE next_review_at IS NOT NULL;

-- Table: learn_suggestions - Log AI suggestions made during real-time assistance
CREATE TABLE IF NOT EXISTS learn_suggestions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conversation_id BIGINT NOT NULL REFERENCES assistant_learn_conversations(id) ON DELETE CASCADE,
    suggestion_type TEXT CHECK(suggestion_type IN ('response', 'correction', 'translation', 'vocabulary')),
    suggested_text TEXT NOT NULL,
    was_accepted BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_learn_suggestions_convo ON learn_suggestions(conversation_id, created_at);
