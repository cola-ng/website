-- Table: taxon_spheres - spheres and classifications
CREATE TABLE IF NOT EXISTS taxon_spheres (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    name TEXT NOT NULL,                               -- 领域名称
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_taxon_spheres_name ON taxon_spheres(name);

-- Table: taxon_categories - categories and classifications
CREATE TABLE IF NOT EXISTS taxon_categories (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    name TEXT NOT NULL,                               -- 分类名称
    sphere_id BIGINT NOT NULL,                        -- 关联领域 ID
    parent_id BIGINT,                                  -- 父分类 ID
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_taxon_categories_name ON taxon_categories(name);

CREATE TABLE IF NOT EXISTS asset_scenes (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    icon_emoji TEXT,
    display_order INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(name_en)
);
CREATE INDEX IF NOT EXISTS idx_asset_scenes_active ON asset_scenes(is_active, display_order);

-- Table: dict_scene_categories - Scene categories and classifications
CREATE TABLE IF NOT EXISTS dict_scene_categories (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    scene_id BIGINT NOT NULL,                            -- 关联场景 ID
    category_id BIGINT NOT NULL,                        -- 分类 ID
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(scene_id, category_id)
);
CREATE INDEX IF NOT EXISTS idx_dict_scene_categories_scene ON dict_scene_categories(scene_id);
CREATE INDEX IF NOT EXISTS idx_dict_scene_categories_id ON dict_scene_categories(category_id);


CREATE TABLE IF NOT EXISTS asset_scripts (
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

CREATE INDEX IF NOT EXISTS idx_asset_scripts_scene ON asset_scripts(scene_id);

CREATE TABLE IF NOT EXISTS asset_script_turns (
    id BIGSERIAL PRIMARY KEY,
    script_id BIGINT NOT NULL,
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

-- ============================================================================
-- READING SUBJECT (Shared content - no user_id)
-- ============================================================================
CREATE TABLE IF NOT EXISTS asset_read_subjects (
    id BIGSERIAL PRIMARY KEY,
    title_en TEXT NOT NULL,
    title_zh TEXT NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    difficulty TEXT,
    subject_type TEXT DEFAULT 'sentence',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS asset_read_sentences (
    id BIGSERIAL PRIMARY KEY,
    subject_id BIGINT NOT NULL,
    sentence_order INTEGER NOT NULL,
    content_en TEXT NOT NULL,
    content_zh TEXT NOT NULL,
    phonetic_transcription TEXT,
    native_audio_path TEXT,
    focus_sounds JSONB,
    common_mistakes JSONB,
    UNIQUE(subject_id, sentence_order)
);

CREATE INDEX IF NOT EXISTS idx_asset_read_sentences_subject ON asset_read_sentences(subject_id, sentence_order);

CREATE TABLE IF NOT EXISTS asset_word_sentences (
    id BIGSERIAL PRIMARY KEY,
    word_id BIGINT NOT NULL, -- Word reference to dict_words table
    sentence_id BIGINT NOT NULL,
    sentence_order INTEGER NOT NULL,
    UNIQUE(word_id, sentence_order)
);
CREATE INDEX IF NOT EXISTS idx_asset_word_sentences_word ON asset_word_sentences(word_id, sentence_id);