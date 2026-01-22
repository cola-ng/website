-- Table: taxon_domains - domains and classifications
CREATE TABLE IF NOT EXISTS taxon_domains (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    name_en TEXT NOT NULL,                               -- 领域名称
    name_zh TEXT NOT NULL,                               -- 领域名称
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_taxon_domains_name ON taxon_domains(name_en, name_zh);

-- Table: taxon_categories - categories and classifications
CREATE TABLE IF NOT EXISTS taxon_categories (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    name_en TEXT NOT NULL,                               -- 分类名称
    name_zh TEXT NOT NULL,                               -- 分类名称
    domain_id BIGINT NOT NULL,                        -- 关联领域 ID
    parent_id BIGINT,                                  -- 父分类 ID
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_taxon_categories_name ON taxon_categories(name_en, name_zh);

CREATE TABLE IF NOT EXISTS asset_contexts (
    id BIGSERIAL PRIMARY KEY,
    name_en TEXT NOT NULL,
    name_zh TEXT NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    icon_emoji TEXT,
    display_order INTEGER DEFAULT 0,
    difficulty SMALLINT, -- 1-10,
    user_id BIGINT, -- If null, it's a shared stage
    prompt TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(name_en, name_zh)
);
CREATE INDEX IF NOT EXISTS idx_asset_contexts_active ON asset_contexts(is_active, display_order);

-- Table: asset_context_categories - Context categories and classifications
CREATE TABLE IF NOT EXISTS asset_context_categories (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    context_id BIGINT NOT NULL,                            -- 关联场景 ID
    category_id BIGINT NOT NULL,                        -- 分类 ID
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(context_id, category_id)
);
CREATE INDEX IF NOT EXISTS idx_asset_context_categories_context ON asset_context_categories(context_id);
CREATE INDEX IF NOT EXISTS idx_asset_context_categories_id ON asset_context_categories(category_id);

CREATE TABLE IF NOT EXISTS asset_stages (
    id BIGSERIAL PRIMARY KEY,
    name_en TEXT NOT NULL,
    name_zh TEXT NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    icon_emoji TEXT,
    display_order INTEGER DEFAULT 0,
    difficulty SMALLINT, -- 1-10,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(name_en, name_zh)
);
CREATE INDEX IF NOT EXISTS idx_asset_stages_active ON asset_stages(is_active, display_order);
CREATE TABLE IF NOT EXISTS asset_scripts (
    id BIGSERIAL PRIMARY KEY,
    stage_id BIGINT NOT NULL,
    title_en TEXT NOT NULL,
    title_zh TEXT NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    total_turns INTEGER DEFAULT 0,
    estimated_duration_seconds INTEGER,
    difficulty SMALLINT,-- 1-10,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_asset_scripts_stage ON asset_scripts(stage_id);

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
    UNIQUE(script_id, turn_number)
);

CREATE INDEX IF NOT EXISTS idx_asset_script_turns_stage ON asset_script_turns(script_id, turn_number);

-- ============================================================================
-- READING SUBJECT (Shared content - no user_id)
-- ============================================================================
CREATE TABLE IF NOT EXISTS asset_read_subjects (
    id BIGSERIAL PRIMARY KEY,
    title_en TEXT NOT NULL,
    title_zh TEXT NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    difficulty SMALLINT, -- 1-10,
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
    difficulty SMALLINT, -- 1-10,
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