-- ============================================================================
-- USER PROGRESS TABLES & ASSET TABLE ENHANCEMENTS
-- ============================================================================

-- Add duration_minutes column to asset_scenes if not exists
ALTER TABLE asset_scenes ADD COLUMN IF NOT EXISTS duration_minutes INTEGER DEFAULT 5;
ALTER TABLE asset_scenes ADD COLUMN IF NOT EXISTS is_featured BOOLEAN DEFAULT FALSE;

-- Add icon column to asset_classic_sources if not exists
ALTER TABLE asset_classic_sources ADD COLUMN IF NOT EXISTS icon_emoji TEXT;
ALTER TABLE asset_classic_sources ADD COLUMN IF NOT EXISTS is_featured BOOLEAN DEFAULT FALSE;
ALTER TABLE asset_classic_sources ADD COLUMN IF NOT EXISTS display_order INTEGER DEFAULT 0;

-- Add tips and difficulty columns to asset_read_sentences if not exists
ALTER TABLE asset_read_sentences ADD COLUMN IF NOT EXISTS tips TEXT;

-- Table: user_scene_progress - Track user progress in scenes
CREATE TABLE IF NOT EXISTS user_scene_progress (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    scene_id BIGINT NOT NULL,
    current_dialogue_id BIGINT,
    progress_percent INTEGER DEFAULT 0 CHECK(progress_percent BETWEEN 0 AND 100),
    completed_at TIMESTAMPTZ,
    last_practiced_at TIMESTAMPTZ,
    practice_count INTEGER DEFAULT 0,
    best_score INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, scene_id)
);

CREATE INDEX IF NOT EXISTS idx_user_scene_progress_user ON user_scene_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_user_scene_progress_scene ON user_scene_progress(scene_id);

-- Table: user_clip_progress - Track user progress in classic clips
CREATE TABLE IF NOT EXISTS user_clip_progress (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    clip_id BIGINT NOT NULL,
    progress_percent INTEGER DEFAULT 0 CHECK(progress_percent BETWEEN 0 AND 100),
    completed_at TIMESTAMPTZ,
    last_practiced_at TIMESTAMPTZ,
    practice_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, clip_id)
);

CREATE INDEX IF NOT EXISTS idx_user_clip_progress_user ON user_clip_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_user_clip_progress_clip ON user_clip_progress(clip_id);

-- Table: user_read_progress - Track user progress in reading exercises
CREATE TABLE IF NOT EXISTS user_read_progress (
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

CREATE INDEX IF NOT EXISTS idx_user_read_progress_user ON user_read_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_user_read_progress_exercise ON user_read_progress(exercise_id);
