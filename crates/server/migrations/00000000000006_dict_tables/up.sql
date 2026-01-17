-- ============================================================================
-- DICTIONARY TABLES (English Dictionary - Server-side)
-- ============================================================================

-- Table: dict_dictionaries - Dictionary metadata
CREATE TABLE IF NOT EXISTS dict_dictionaries (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    name TEXT NOT NULL UNIQUE,               -- 字典名称
    description_en TEXT,                                -- 英文描述
    description_zh TEXT,                                -- 中文描述
    version TEXT,                                       -- 版本号
    publisher TEXT,                                     -- 出版者
    license_type TEXT CHECK(license_type IN ('public_domain', 'creative_commons', 'proprietary', 'educational', 'commercial')), -- 许可证类型
    license_url TEXT,                                   -- 许可证链接
    source_url TEXT,                                    -- 来源链接
    total_entries BIGINT DEFAULT 0,                     -- 总词条数
    is_active BOOLEAN DEFAULT TRUE,                     -- 是否激活（可用状态）
    is_official BOOLEAN DEFAULT FALSE,                  -- 是否为官方字典
    priority_order INTEGER DEFAULT 100,                 -- 优先级（越小越优先）
    created_by BIGINT,                                  -- 创建者用户 ID
    updated_by BIGINT,                                  -- 更新者用户 ID
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 更新时间
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);
CREATE INDEX IF NOT EXISTS idx_dict_dictionaries_name ON dict_dictionaries(name);
CREATE INDEX IF NOT EXISTS idx_dict_dictionaries_active ON dict_dictionaries(is_active);
CREATE INDEX IF NOT EXISTS idx_dict_dictionaries_priority ON dict_dictionaries(priority_order ASC);

-- Table: dict_words - Main word table, 单词表, 也包含了短语
CREATE TABLE IF NOT EXISTS dict_words (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word TEXT NOT NULL UNIQUE,                          -- 单词（原形式）
    word_lower TEXT NOT NULL,                           -- 单词小写形式，用于搜索
    word_type TEXT CHECK(word_type IN ('noun', 'verb', 'adjective', 'adverb', 'pronoun', 'preposition', 'conjunction', 'interjection', 'article', 'abbreviation', 'phrase', 'idiom')), -- 词性：名词、动词、形容词、副词等
    language TEXT DEFAULT 'en',                         -- 语言（默认英文）
    frequency SMALLINT CHECK(frequency BETWEEN 0 AND 100), -- 频率评分 (0-100)，越高表示越常用
    difficulty SMALLINT CHECK(difficulty BETWEEN 1 AND 10), -- 难度等级 (1-5)，1 最简单，5 最难
    syllable_count SMALLINT,                   -- 音节数量
    is_lemma BOOLEAN DEFAULT TRUE,                      -- 是否为词元（词根/原形）
    word_count INTEGER DEFAULT 0,                       -- 单词数量
    is_active BOOLEAN DEFAULT TRUE,                     -- 是否激活（可用状态）
    created_by BIGINT,                                  -- 创建者用户 ID
    updated_by BIGINT,                                  -- 更新者用户 ID
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 更新时间
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_words_word ON dict_words(word);
CREATE INDEX IF NOT EXISTS idx_dict_words_word_lower ON dict_words(word_lower);
CREATE INDEX IF NOT EXISTS idx_dict_words_word_type ON dict_words(word_type);
CREATE INDEX IF NOT EXISTS idx_dict_words_frequency ON dict_words(frequency DESC);
CREATE INDEX IF NOT EXISTS idx_dict_words_difficulty ON dict_words(difficulty);

-- Table: dict_dictionaries - Many-to-many relationship between words and dictionaries
CREATE TABLE IF NOT EXISTS dict_word_dictionaries (
    id BIGSERIAL PRIMARY KEY,                          -- Primary key ID
    word_id BIGINT NOT NULL,                            -- Associated word ID
    dictionary_id BIGINT NOT NULL,                      -- Associated dictionary ID
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- Unique constraint to ensure each word-dictionary pair is only added once
);
CREATE INDEX IF NOT EXISTS idx_dict_word_dicts_word ON dict_word_dictionaries(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_word_dicts_dict ON dict_word_dictionaries(dictionary_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_dict_word_dicts_unique ON dict_word_dictionaries(word_id, dictionary_id);

-- Table: dict_definitions - Word definitions (multiple per word)
CREATE TABLE IF NOT EXISTS dict_definitions (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    language TEXT NOT NULL,                         -- 语言
    definition TEXT NOT NULL,                        -- 释义
    part_of_speech TEXT CHECK(part_of_speech IN ('noun', 'verb', 'adjective', 'adverb', 'pronoun', 'preposition', 'conjunction', 'interjection', 'article', 'abbreviation', 'phrase', 'idiom')), -- 词性：名词、动词、形容词、副词等
    definition_order INTEGER DEFAULT 1,                 -- 释义顺序
    register TEXT CHECK(register IN ('formal', 'informal', 'slang', 'archaic', 'literary', 'technical', 'colloquial', 'neutral')), -- 语体：正式、非正式、俚语、古语等
    region TEXT CHECK(region IN ('US', 'UK', 'AU', 'CA', 'NZ', 'IN', 'general')), -- 地区：美式、英式、澳式等
    context TEXT,                                       -- 使用语境
    usage_notes TEXT,                                   -- 用法说明
    is_primary BOOLEAN DEFAULT FALSE,                   -- 是否为主要释义
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);
CREATE INDEX IF NOT EXISTS idx_dict_definitions_word ON dict_definitions(word_id, definition_order);
CREATE INDEX IF NOT EXISTS idx_dict_definitions_pos ON dict_definitions(part_of_speech);

CREATE TABLE IF NOT EXISTS dict_translations (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    origin_entity TEXT NOT NULL, -- 来源实体（单词或释义）
    origin_id BIGINT NOT NULL,                         -- 关联 ID
    language TEXT NOT NULL,                         -- 语言
    translation TEXT NOT NULL,                        -- 释义
    context TEXT,                                       -- 使用语境
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

-- Table: dict_pronunciations - Word pronunciations
CREATE TABLE IF NOT EXISTS dict_pronunciations (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    definition_id BIGINT,                               -- 关联释义 ID
    ipa TEXT NOT NULL,                                 -- 国际音标 (IPA)
    audio_url TEXT,                                     -- 音频文件 URL
    audio_path TEXT,                                    -- 音频文件存储路径
    dialect TEXT CHECK(dialect IN ('US', 'UK', 'AU', 'CA', 'NZ', 'IN', 'other')), -- 口音：美式、英式、澳式等
    gender TEXT CHECK(gender IN ('male', 'female', 'neutral')), -- 性别：男声、女声、中性
    is_primary BOOLEAN DEFAULT FALSE,                   -- 是否为主要发音
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);
CREATE INDEX IF NOT EXISTS idx_dict_pronunciations_word ON dict_pronunciations(word_id, dialect);

-- Table: dict_sentences - Example sentences
CREATE TABLE IF NOT EXISTS dict_sentences (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    language TEXT NOT NULL,                         -- 语言
    sentence TEXT NOT NULL,                          -- 例句
    source TEXT,                                        -- 来源
    author TEXT,                                        -- 作者
    difficulty INTEGER CHECK(difficulty BETWEEN 1 AND 5), -- 难度等级 (1-5)
    is_common BOOLEAN DEFAULT FALSE,                    -- 是否为常用例句
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE TABLE IF NOT EXISTS dict_word_sentences (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    definition_id BIGINT,                               -- 关联释义 ID
    sentence_id BIGINT NOT NULL,                               -- 关联释义 ID
    priority_order INTEGER DEFAULT 1,                    -- 例句顺序
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE Unique INDEX IF NOT EXISTS idx_dict_sentences_word ON dict_word_sentences(word_id, sentence_id);
CREATE INDEX IF NOT EXISTS idx_dict_sentences_definition ON dict_word_sentences(definition_id);

-- Table: dict_etymology - etymology information 语源；词源学
CREATE TABLE IF NOT EXISTS dict_etymologies (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    origin_language TEXT,                               -- 起源语言
    origin_word TEXT,                                   -- 起源单词
    origin_meaning TEXT,                                -- 起源含义
    language TEXT NOT NULL,                             -- 词源说明语言
    etymology TEXT NOT NULL,                            -- 词源说明
    first_attested_year INTEGER,                        -- 首次出现的年份
    historical_forms JSONB,                              -- 历史形式 (JSONB)
    cognate_words JSONB,                                -- 同源词 (JSONB)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);
-- 一个单词可能来源于不同的语源
CREATE TABLE IF NOT EXISTS dict_word_etymologies (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    etymology_id BIGINT NOT NULL,                       -- 关联语源 ID
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_dict_etymology_word ON dict_word_etymologies(word_id, etymology_id);

-- Table: dict_categories - Word categories and classifications
CREATE TABLE IF NOT EXISTS dict_categories (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    name TEXT NOT NULL,                               -- 分类名称
    parent_id BIGINT,                                  -- 父分类 ID
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_dict_categories_name ON dict_categories(name);

-- Table: dict_word_categories - Word categories and classifications
CREATE TABLE IF NOT EXISTS dict_word_categories (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    category_id BIGINT NOT NULL,                        -- 分类 ID
    confidence SMALLINT NOT NULL DEFAULT 50 CHECK(confidence BETWEEN 0 AND 100), -- 置信度 (0-100)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(word_id, category_id)
);
CREATE INDEX IF NOT EXISTS idx_dict_word_categories_word ON dict_word_categories(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_word_categories_id ON dict_word_categories(category_id);

-- Table: dict_parts - Words that make up a phrase
CREATE TABLE IF NOT EXISTS dict_parts (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    part_id BIGINT NOT NULL,                            -- 组成成分 ID, 其实也是  word id
    range_begin INTEGER NOT NULL,                      -- 单词在短语中的起始位置(包括)
    range_until INTEGER,                      -- 单词在短语中的结束位置(包括)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_parts_range ON dict_parts(word_id, part_id);


-- Table: dict_forms - Word forms (plurals, tenses, comparatives, etc.)
CREATE TABLE IF NOT EXISTS dict_forms (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    form_type TEXT CHECK(form_type IN ('plural', 'singular', 'past', 'present', 'future', 'present_participle', 'past_participle', 'comparative', 'superlative', 'adverbial', 'nominalization', 'other')), -- 词形类型：复数、过去式、现在分词等
    form TEXT NOT NULL,                                 -- 词形
    is_irregular BOOLEAN DEFAULT FALSE,                 -- 是否为不规则变形
    notes TEXT,                                         -- 备注
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_forms_word ON dict_forms(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_forms_type ON dict_forms(form_type);

-- Table: dict_relations - Word relations (synonyms, antonyms, related words, etc.)
CREATE TABLE IF NOT EXISTS dict_relations (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    relation_type TEXT CHECK(relation_type IN ('synonym', 'antonym', 'related', 'broader', 'narrower', 'part_of', 'member_of', 'substance_of', 'instance_of', 'similar')), -- 条目类型：同义词、反义词、相关词、上位词等
    related_word_id BIGINT NOT NULL,                    -- 相关单词 ID
    semantic_field TEXT,                                 -- 语义场
    relation_strength REAL CHECK(relation_strength BETWEEN 0 AND 1), -- 关系强度 (0-1)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(word_id, relation_type, related_word_id)
);

CREATE INDEX IF NOT EXISTS idx_dict_relations_word ON dict_relations(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_relations_type ON dict_relations(relation_type);


-- Table: dict_frequency_bands - Frequency band classifications
CREATE TABLE IF NOT EXISTS dict_frequencies (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    corpus_name TEXT NOT NULL,                          -- 语料库名称
    corpus_type TEXT CHECK(corpus_type IN ('spoken', 'written', 'academic', 'news', 'fiction', 'internet', 'general')), -- 语料库类型：口语、书面语、学术等
    band TEXT CHECK(band IN ('top_1000', 'top_2000', 'top_3000', 'top_5000', 'top_10000', 'beyond_10000')), -- 频率等级：前1000、前2000等
    rank INTEGER,                                       -- 排名
    frequency_per_million INTEGER,                         -- 每百万词出现频率
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(word_id, corpus_name, corpus_type)
);
CREATE INDEX IF NOT EXISTS idx_dict_frequencies_word ON dict_frequencies(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_frequencies_band ON dict_frequencies(band);

-- Table: dict_import_batches - Track import batches
CREATE TABLE IF NOT EXISTS dict_import_batches (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    batch_name TEXT NOT NULL,                           -- 批次名称
    source TEXT,                                        -- 来源
    source_type TEXT CHECK(source_type IN ('api', 'file', 'manual', 'database', 'other')), -- 来源类型：API、文件、手动、数据库等
    total_words INTEGER DEFAULT 0,                      -- 总单词数
    successful_words INTEGER DEFAULT 0,                 -- 成功导入的单词数
    failed_words INTEGER DEFAULT 0,                     -- 失败的单词数
    status TEXT CHECK(status IN ('pending', 'running', 'completed', 'failed', 'partial')), -- 状态：待处理、运行中、完成、失败、部分完成
    metadata JSONB,                                     -- 元数据 (JSONB)
    started_at TIMESTAMPTZ,                              -- 开始时间
    completed_at TIMESTAMPTZ,                            -- 完成时间
    created_by BIGINT,                                  -- 创建者用户 ID
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);
CREATE INDEX IF NOT EXISTS idx_dict_import_status ON dict_import_batches(status);
CREATE INDEX IF NOT EXISTS idx_dict_import_created ON dict_import_batches(created_at DESC);

-- Table: dict_images - Word images for visual learning
CREATE TABLE IF NOT EXISTS dict_images (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    image_url TEXT,                                     -- 图片 URL
    image_path TEXT,                                    -- 图片存储路径
    image_type TEXT CHECK(image_type IN ('illustration', 'photograph', 'icon', 'diagram', 'other')), -- 图片类型：插画、照片、图标、图表等
    alt_text_en TEXT,                                   -- 英文替代文本
    alt_text_zh TEXT,                                   -- 中文替代文本
    is_primary BOOLEAN DEFAULT FALSE,                   -- 是否为主要图片
    created_by BIGINT,                                  -- 创建者用户 ID
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_images_word ON dict_images(word_id);