CREATE TABLE IF NOT EXISTS asset_scenes (
    id BIGSERIAL PRIMARY KEY,
    name_en TEXT NOT NULL,
    name_zh TEXT NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    icon_emoji TEXT,
    difficulty TEXT CHECK(difficulty IN ('beginner', 'intermediate', 'advanced')) DEFAULT 'intermediate',
    category TEXT,
    display_order INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(name_en)
);

CREATE INDEX IF NOT EXISTS idx_asset_scenes_active ON asset_scenes(is_active, display_order);

CREATE TABLE IF NOT EXISTS asset_dialogues (
    id BIGSERIAL PRIMARY KEY,
    scene_id BIGINT NOT NULL,
    title_en TEXT NOT NULL,
    title_zh TEXT NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    total_turns INTEGER DEFAULT 0,
    estimated_duration_seconds INTEGER,
    difficulty TEXT CHECK(difficulty IN ('beginner', 'intermediate', 'advanced')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_asset_dialogues_scene ON asset_dialogues(scene_id);

CREATE TABLE IF NOT EXISTS asset_dialogue_turns (
    id BIGSERIAL PRIMARY KEY,
    dialogue_id BIGINT NOT NULL,
    turn_number INTEGER NOT NULL,
    speaker_role TEXT NOT NULL,
    speaker_name TEXT,
    content_en TEXT NOT NULL,
    content_zh TEXT NOT NULL,
    audio_path TEXT,
    phonetic_transcription TEXT,
    asset_phrases JSONB,
    notes TEXT,
    UNIQUE(dialogue_id, turn_number)
);

CREATE INDEX IF NOT EXISTS idx_asset_dialogue_turns_scene ON asset_dialogue_turns(dialogue_id, turn_number);

CREATE TABLE IF NOT EXISTS asset_classic_sources (
    id BIGSERIAL PRIMARY KEY,
    source_type TEXT NOT NULL CHECK(source_type IN ('movie', 'tv_show', 'ted_talk', 'documentary', 'other')),
    title TEXT NOT NULL,
    year INTEGER,
    description_en TEXT,
    description_zh TEXT,
    thumbnail_url TEXT,
    imdb_id TEXT,
    difficulty TEXT CHECK(difficulty IN ('beginner', 'intermediate', 'advanced')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(source_type, title)
);

CREATE INDEX IF NOT EXISTS idx_asset_classic_sources_type ON asset_classic_sources(source_type);

CREATE TABLE IF NOT EXISTS asset_classic_clips (
    id BIGSERIAL PRIMARY KEY,
    source_id BIGINT NOT NULL,
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

CREATE INDEX IF NOT EXISTS idx_asset_classic_clips_source ON asset_classic_clips(source_id);
CREATE INDEX IF NOT EXISTS idx_asset_classic_clips_popularity ON asset_classic_clips(popularity_score DESC);

-- ============================================================================
-- READING PRACTICE (Shared content - no user_id)
-- ============================================================================

CREATE TABLE IF NOT EXISTS asset_read_exercises (
    id BIGSERIAL PRIMARY KEY,
    title_en TEXT NOT NULL,
    title_zh TEXT NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    difficulty TEXT CHECK(difficulty IN ('beginner', 'intermediate', 'advanced')),
    exercise_type TEXT CHECK(exercise_type IN ('sentence', 'paragraph', 'dialogue', 'tongue_twister')) DEFAULT 'sentence',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS asset_read_sentences (
    id BIGSERIAL PRIMARY KEY,
    exercise_id BIGINT NOT NULL,
    sentence_order INTEGER NOT NULL,
    content_en TEXT NOT NULL,
    content_zh TEXT NOT NULL,
    phonetic_transcription TEXT,
    native_audio_path TEXT,
    focus_sounds JSONB,
    common_mistakes JSONB,
    UNIQUE(exercise_id, sentence_order)
);

CREATE INDEX IF NOT EXISTS idx_asset_read_sentences_exercise ON asset_read_sentences(exercise_id, sentence_order);

-- ============================================================================
-- KEY PHRASES (Shared content - no user_id)
-- ============================================================================

CREATE TABLE IF NOT EXISTS asset_phrases (
    id BIGSERIAL PRIMARY KEY,
    phrase_en TEXT NOT NULL,
    phrase_zh TEXT NOT NULL,
    phonetic_transcription TEXT,
    usage_context TEXT,
    example_sentence_en TEXT,
    example_sentence_zh TEXT,
    category TEXT,
    formality_level TEXT CHECK(formality_level IN ('casual', 'neutral', 'formal')),
    frequency INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(phrase_en)
);

CREATE INDEX IF NOT EXISTS idx_asset_phrases_category ON asset_phrases(category);
