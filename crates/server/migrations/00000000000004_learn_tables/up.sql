-- ============================================================================
-- USER-SPECIFIC TABLES (All include user_id)
-- ============================================================================

-- Table: learn_issue_words - Words that the user has problems with
CREATE TABLE IF NOT EXISTS learn_issue_words (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    word TEXT NOT NULL,
    issue_type TEXT NOT NULL CHECK(issue_type IN ('pronunciation', 'usage', 'unfamiliar', 'grammar')),
    description_en TEXT,
    description_zh TEXT,
    last_picked_at TIMESTAMPTZ,
    pick_count INTEGER NOT NULL DEFAULT 0,
    next_review_at TIMESTAMPTZ,
    review_interval_days INTEGER DEFAULT 1,
    difficulty INTEGER DEFAULT 1 CHECK(difficulty BETWEEN 1 AND 5),
    context TEXT,
    audio_timestamp INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, word, issue_type)
);

CREATE INDEX IF NOT EXISTS idx_learn_issue_words_user ON learn_issue_words(user_id, next_review_at);
CREATE INDEX IF NOT EXISTS idx_learn_issue_words_review ON learn_issue_words(next_review_at) WHERE next_review_at IS NOT NULL;


CREATE TABLE IF NOT EXISTS learn_chats (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    title TEXT NOT NULL,
    context_id BIGINT,
    duration_ms INTEGER,
    pause_count INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_learn_chats_user ON learn_chats(user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_learn_chats_created ON learn_chats(created_at);


-- Table: learn_chat_turns - Stores all chat history
CREATE TABLE IF NOT EXISTS learn_chat_turns (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    chat_id BIGINT NOT NULL,
    speaker TEXT NOT NULL,
    use_lang TEXT NOT NULL CHECK(use_lang IN ('en', 'zh')),
    content_en TEXT NOT NULL,
    content_zh TEXT NOT NULL,
    audio_path TEXT,
    duration_ms INTEGER,
    words_per_minute REAL,
    pause_count INTEGER,
    hesitation_count INTEGER,
    status TEXT NOT NULL DEFAULT 'completed', -- 'pending', 'processing', 'completed', 'error'
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_learn_chat_turns_user_chat ON learn_chat_turns(user_id, chat_id, created_at);
CREATE INDEX IF NOT EXISTS idx_learn_chat_turns_created ON learn_chat_turns(created_at);

-- Table: learn_chat_issues - Stores issues and issues found in learn_chats
CREATE TABLE IF NOT EXISTS learn_chat_issues (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    chat_id BIGINT NOT NULL,
    chat_turn_id BIGINT NOT NULL,
    issue_type TEXT NOT NULL CHECK(
        issue_type IN ('pronunciation_error', 'grammar_error', 'word_choice',
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
CREATE INDEX IF NOT EXISTS idx_issues_chat ON learn_chat_issues(chat_id, chat_turn_id);
CREATE INDEX IF NOT EXISTS idx_issues_user ON learn_chat_issues(user_id, issue_type);

-- Table: learn_word_practices - Logs each time a word is practiced
CREATE TABLE IF NOT EXISTS learn_practices (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    success_level INTEGER CHECK(success_level BETWEEN 1 AND 5),
    notes TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_learn_practices_user_session ON learn_practices(user_id);

CREATE TABLE IF NOT EXISTS learn_write_practices (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    word_id BIGINT NOT NULL,
    practice_id TEXT NOT NULL,
    success_level INTEGER CHECK(success_level BETWEEN 1 AND 5),
    notes TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_learn_practices_word ON learn_write_practices(word_id, updated_at);
CREATE INDEX IF NOT EXISTS idx_learn_practices_user_session ON learn_write_practices(user_id, practice_id);


-- Table: learn_read_practices - Log of user's reading practice attempts
CREATE TABLE IF NOT EXISTS learn_read_practices (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    sentence_id BIGINT,
    practice_id TEXT NOT NULL,
    user_audio_path TEXT,
    pronunciation_score INTEGER CHECK(pronunciation_score BETWEEN 0 AND 100),
    fluency_score INTEGER CHECK(fluency_score BETWEEN 0 AND 100),
    intonation_score INTEGER CHECK(intonation_score BETWEEN 0 AND 100),
    overall_score INTEGER CHECK(overall_score BETWEEN 0 AND 100),
    detected_errors JSONB,
    ai_feedback_en TEXT,
    ai_feedback_zh TEXT,
    waveform_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_read_practices_sentence ON learn_read_practices(sentence_id, created_at);
CREATE INDEX IF NOT EXISTS idx_read_practices_user_session ON learn_read_practices(user_id, practice_id);

-- Table: learn_achievements - Track user achievements and milestones
CREATE TABLE IF NOT EXISTS learn_achievements (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
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
    user_id BIGINT NOT NULL,
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
    user_id BIGINT NOT NULL,
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
    user_id BIGINT NOT NULL,
    suggestion_type TEXT CHECK(suggestion_type IN ('response', 'correction', 'translation', 'vocabulary')),
    suggested_text TEXT NOT NULL,
    was_accepted BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);



-- Table: learn_script_progress - Track user learn progress in scripts
CREATE TABLE IF NOT EXISTS learn_script_progress (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    stage_id BIGINT NOT NULL,
    script_id BIGINT NOT NULL,
    progress_percent INTEGER DEFAULT 0 CHECK(progress_percent BETWEEN 0 AND 100),
    completed_at TIMESTAMPTZ,
    last_practiced_at TIMESTAMPTZ,
    practice_count INTEGER DEFAULT 0,
    best_score INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, script_id)
);

CREATE INDEX IF NOT EXISTS idx_learn_script_progress_user ON learn_script_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_learn_script_progress_stage ON learn_script_progress(stage_id, script_id);

-- Table: learn_read_progress - Track user progress in reading exercises
CREATE TABLE IF NOT EXISTS learn_read_progress (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    exercise_id BIGINT NOT NULL,
    current_sentence_order INTEGER DEFAULT 0,
    progress_percent INTEGER DEFAULT 0 CHECK(progress_percent BETWEEN 0 AND 100),
    completed_at TIMESTAMPTZ,
    last_practiced_at TIMESTAMPTZ,
    practice_count INTEGER DEFAULT 0,
    average_score INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, exercise_id)
);

CREATE INDEX IF NOT EXISTS idx_learn_read_progress_user ON learn_read_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_learn_read_progress_exercise ON learn_read_progress(exercise_id);