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

-- Table: dict_words - Main word table
CREATE TABLE IF NOT EXISTS dict_words (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word TEXT NOT NULL UNIQUE,                          -- 单词（原形式）
    word_lower TEXT NOT NULL,                           -- 单词小写形式，用于搜索
    word_type TEXT CHECK(word_type IN ('noun', 'verb', 'adjective', 'adverb', 'pronoun', 'preposition', 'conjunction', 'interjection', 'article', 'abbreviation', 'phrase', 'idiom')), -- 词性：名词、动词、形容词、副词等
    language TEXT DEFAULT 'en',                         -- 语言（默认英文）
    frequency_score INTEGER CHECK(frequency_score BETWEEN 0 AND 100), -- 频率评分 (0-100)，越高表示越常用
    difficulty_level INTEGER CHECK(difficulty_level BETWEEN 1 AND 5), -- 难度等级 (1-5)，1 最简单，5 最难
    syllable_count INTEGER DEFAULT 1,                   -- 音节数量
    is_lemma BOOLEAN DEFAULT TRUE,                      -- 是否为词元（词根/原形）
    word_count INTEGER DEFAULT 0,                       -- 单词出现次数统计
    is_active BOOLEAN DEFAULT TRUE,                     -- 是否激活（可用状态）
    created_by BIGINT,                                  -- 创建者用户 ID
    updated_by BIGINT,                                  -- 更新者用户 ID
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 更新时间
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_words_word ON dict_words(word);
CREATE INDEX IF NOT EXISTS idx_dict_words_word_lower ON dict_words(word_lower);
CREATE INDEX IF NOT EXISTS idx_dict_words_word_type ON dict_words(word_type);
CREATE INDEX IF NOT EXISTS idx_dict_words_frequency ON dict_words(frequency_score DESC);
CREATE INDEX IF NOT EXISTS idx_dict_words_difficulty ON dict_words(difficulty_level);

-- Table: dict_word_dictionaries - Many-to-many relationship between words and dictionaries
CREATE TABLE IF NOT EXISTS dict_word_dictionaries (
    id BIGSERIAL PRIMARY KEY,                          -- Primary key ID
    word_id BIGINT NOT NULL,                            -- Associated word ID
    dictionary_id BIGINT NOT NULL,                      -- Associated dictionary ID
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- Unique constraint to ensure each word-dictionary pair is only added once
);

CREATE INDEX IF NOT EXISTS idx_dict_word_dicts_word ON dict_word_dictionaries(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_word_dicts_dict ON dict_word_dictionaries(dictionary_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_dict_word_dicts_unique ON dict_word_dictionaries(word_id, dictionary_id);


-- Table: dict_word_definitions - Word definitions (multiple per word)
CREATE TABLE IF NOT EXISTS dict_word_definitions (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    definition_en TEXT NOT NULL,                        -- 英文释义
    definition_zh TEXT,                                 -- 中文释义
    part_of_speech TEXT CHECK(part_of_speech IN ('noun', 'verb', 'adjective', 'adverb', 'pronoun', 'preposition', 'conjunction', 'interjection', 'article', 'abbreviation', 'phrase', 'idiom')), -- 词性：名词、动词、形容词、副词等
    definition_order INTEGER DEFAULT 1,                 -- 释义顺序
    register TEXT CHECK(register IN ('formal', 'informal', 'slang', 'archaic', 'literary', 'technical', 'colloquial', 'neutral')), -- 语体：正式、非正式、俚语、古语等
    region TEXT CHECK(region IN ('US', 'UK', 'AU', 'CA', 'NZ', 'IN', 'general')), -- 地区：美式、英式、澳式等
    context TEXT,                                       -- 使用语境
    usage_notes TEXT,                                   -- 用法说明
    is_primary BOOLEAN DEFAULT FALSE,                   -- 是否为主要释义
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_definitions_word ON dict_word_definitions(word_id, definition_order);
CREATE INDEX IF NOT EXISTS idx_dict_definitions_pos ON dict_word_definitions(part_of_speech);

CREATE TABLE IF NOT EXISTS dict_word_lemmas (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    lemma_word_id BIGINT NOT NULL,                    -- 同义词单词 ID
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(word_id, lemma_word_id)
);
CREATE INDEX IF NOT EXISTS idx_dict_word_lemmas_word ON dict_word_lemmas(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_word_lemmas_lemma_word ON dict_word_lemmas(word_id, lemma_word_id);

-- Table: dict_word_pronunciations - Word pronunciations
CREATE TABLE IF NOT EXISTS dict_word_pronunciations (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    ipa TEXT NOT NULL,                                 -- 国际音标 (IPA)
    audio_url TEXT,                                     -- 音频文件 URL
    audio_path TEXT,                                    -- 音频文件存储路径
    dialect TEXT CHECK(dialect IN ('US', 'UK', 'AU', 'CA', 'NZ', 'IN', 'other')), -- 口音：美式、英式、澳式等
    is_primary BOOLEAN DEFAULT FALSE,                   -- 是否为主要发音
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);
CREATE INDEX IF NOT EXISTS idx_dict_pronunciations_word ON dict_word_pronunciations(word_id, dialect);

-- Table: dict_word_examples - Example sentences
CREATE TABLE IF NOT EXISTS dict_word_examples (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    definition_id BIGINT,                               -- 关联释义 ID
    sentence_en TEXT NOT NULL,                          -- 英文例句
    sentence_zh TEXT,                                   -- 中文翻译
    source TEXT,                                        -- 来源
    author TEXT,                                        -- 作者
    example_order INTEGER DEFAULT 1,                    -- 例句顺序
    difficulty_level INTEGER CHECK(difficulty_level BETWEEN 1 AND 5), -- 难度等级 (1-5)
    is_common BOOLEAN DEFAULT FALSE,                    -- 是否为常用例句
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_examples_word ON dict_word_examples(word_id, example_order);
CREATE INDEX IF NOT EXISTS idx_dict_examples_definition ON dict_word_examples(definition_id);

-- Table: dict_word_synonyms - Synonym relationships
CREATE TABLE IF NOT EXISTS dict_word_synonyms (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    synonym_word_id BIGINT NOT NULL,                    -- 同义词单词 ID
    similarity_score REAL CHECK(similarity_score BETWEEN 0 AND 1), -- 相似度评分 (0-1)
    context TEXT,                                       -- 使用语境
    nuance_notes TEXT,                                  -- 细微差别说明
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(word_id, synonym_word_id)
);

CREATE INDEX IF NOT EXISTS idx_dict_synonyms_word ON dict_word_synonyms(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_synonyms_similarity ON dict_word_synonyms(similarity_score DESC);

-- Table: dict_word_antonyms - Antonym relationships
CREATE TABLE IF NOT EXISTS dict_word_antonyms (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    antonym_word_id BIGINT NOT NULL,                    -- 反义词单词 ID
    antonym_type TEXT CHECK(antonym_type IN ('direct', 'gradable', 'relational', 'complementary')), -- 反义词类型：直接、可分级、关系、互补
    context TEXT,                                       -- 使用语境
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(word_id, antonym_word_id)
);

CREATE INDEX IF NOT EXISTS idx_dict_antonyms_word ON dict_word_antonyms(word_id);

-- Table: dict_word_collocations - Word collocations (phrases where words appear together)
CREATE TABLE IF NOT EXISTS dict_word_collocations (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    collocation_type TEXT CHECK(collocation_type IN ('adjective+noun', 'verb+noun', 'noun+verb', 'verb+adverb', 'adverb+adjective', 'noun+noun', 'preposition+noun', 'other')), -- 搭配类型：形容词+名词、动词+名词等
    collocated_word_id BIGINT,                          -- 搭配单词 ID
    phrase TEXT NOT NULL,                               -- 搭配短语（小写）
    phrase_en TEXT NOT NULL,                            -- 英文搭配短语
    phrase_zh TEXT,                                     -- 中文翻译
    frequency_score INTEGER CHECK(frequency_score BETWEEN 0 AND 100), -- 频率评分 (0-100)
    register TEXT CHECK(register IN ('formal', 'informal', 'slang', 'archaic', 'literary', 'technical', 'colloquial', 'neutral')), -- 语体：正式、非正式、俚语等
    example_en TEXT,                                    -- 英文例句
    example_zh TEXT,                                    -- 中文翻译
    is_common BOOLEAN DEFAULT FALSE,                    -- 是否为常用搭配
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_collocations_word ON dict_word_collocations(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_collocations_type ON dict_word_collocations(collocation_type);
CREATE INDEX IF NOT EXISTS idx_dict_collocations_phrase ON dict_word_collocations(phrase_en);

-- Table: dict_word_etymology - Word etymology information
CREATE TABLE IF NOT EXISTS dict_word_etymology (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    origin_language TEXT,                               -- 起源语言
    origin_word TEXT,                                   -- 起源单词
    origin_meaning TEXT,                                -- 起源含义
    etymology_en TEXT,                                  -- 英文词源说明
    etymology_zh TEXT,                                  -- 中文词源说明
    first_attested_year INTEGER,                        -- 首次出现的年份
    historical_forms JSONB,                              -- 历史形式 (JSONB)
    cognate_words JSONB,                                -- 同源词 (JSONB)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);
CREATE INDEX IF NOT EXISTS idx_dict_etymology_word ON dict_word_etymology(word_id);

CREATE TABLE IF NOT EXISTS dict_word_etymologies (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    etymolog_id TEXT,                               -- 起源语言
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_etymology_word ON dict_word_etymology(word_id);

-- Table: dict_word_categories - Word categories and classifications
CREATE TABLE IF NOT EXISTS dict_word_categories (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    category_type TEXT CHECK(category_type IN ('subject', 'topic', 'field', 'usage', 'level', 'frequency_band', 'semantic_field', 'thematic', 'academic_level')), -- 分类类型：学科、主题、领域、用法等
    category_name TEXT NOT NULL,                        -- 分类名称
    category_value TEXT NOT NULL,                        -- 分类值
    confidence_score REAL CHECK(confidence_score BETWEEN 0 AND 1), -- 置信度 (0-1)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(word_id, category_type, category_value)
);

CREATE INDEX IF NOT EXISTS idx_dict_categories_word ON dict_word_categories(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_categories_type ON dict_word_categories(category_type);
CREATE INDEX IF NOT EXISTS idx_dict_categories_value ON dict_word_categories(category_value);

-- Table: dict_word_usage_notes - Usage notes and warnings
CREATE TABLE IF NOT EXISTS dict_word_usage_notes (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    note_type TEXT CHECK(note_type IN ('warning', 'preference', 'avoidance', 'contextual', 'regional', 'formality', 'tone', 'grammar', 'style', 'spelling')), -- 说明类型：警告、偏好、避免、语境等
    note_en TEXT NOT NULL,                              -- 英文说明
    note_zh TEXT,                                       -- 中文翻译
    examples_en JSONB,                                  -- 英文例句列表 (JSONB)
    examples_zh JSONB,                                  -- 中文例句列表 (JSONB)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_usage_notes_word ON dict_word_usage_notes(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_usage_notes_type ON dict_word_usage_notes(note_type);

-- Table: dict_phrases - Common phrases and idioms
CREATE TABLE IF NOT EXISTS dict_phrases (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    phrase TEXT NOT NULL UNIQUE,                        -- 短语（原形式）
    phrase_lower TEXT NOT NULL,                         -- 短语小写形式，用于搜索
    phrase_type TEXT CHECK(phrase_type IN ('idiom', 'proverb', 'saying', 'slang', 'colloquial', 'formal', 'technical', 'phrase', 'expression')), -- 短语类型：习语、谚语、俗语、俚语等
    meaning_en TEXT NOT NULL,                           -- 英文含义
    meaning_zh TEXT,                                    -- 中文含义
    origin TEXT,                                        -- 来源
    example_en TEXT,                                    -- 英文例句
    example_zh TEXT,                                    -- 中文翻译
    difficulty_level INTEGER CHECK(difficulty_level BETWEEN 1 AND 5), -- 难度等级 (1-5)
    frequency_score INTEGER CHECK(frequency_score BETWEEN 0 AND 100), -- 频率评分 (0-100)
    is_active BOOLEAN DEFAULT TRUE,                     -- 是否激活（可用状态）
    created_by BIGINT,                                  -- 创建者用户 ID
    updated_by BIGINT,                                  -- 更新者用户 ID
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 更新时间
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_phrases_phrase ON dict_phrases(phrase);
CREATE INDEX IF NOT EXISTS idx_dict_phrases_lower ON dict_phrases(phrase_lower);
CREATE INDEX IF NOT EXISTS idx_dict_phrases_type ON dict_phrases(phrase_type);

-- Table: dict_phrase_words - Words that make up a phrase
CREATE TABLE IF NOT EXISTS dict_phrase_words (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    phrase_id BIGINT NOT NULL,                          -- 关联短语 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    word_position INTEGER NOT NULL,                      -- 单词在短语中的位置
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_phrase_words_phrase ON dict_phrase_words(phrase_id, word_position);

-- Table: dict_word_family - Word family relationships
CREATE TABLE IF NOT EXISTS dict_word_family (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    root_word_id BIGINT NOT NULL,                       -- 词根单词 ID
    related_word_id BIGINT NOT NULL,                    -- 相关单词 ID
    relationship_type TEXT CHECK(relationship_type IN ('derivation', 'inflection', 'compound', 'prefix', 'suffix', 'conversion', 'other')), -- 关系类型：派生、屈折、复合等
    morpheme TEXT,                                      -- 词素（前缀、后缀等）
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(root_word_id, related_word_id)
);

CREATE INDEX IF NOT EXISTS idx_dict_family_root ON dict_word_family(root_word_id);
CREATE INDEX IF NOT EXISTS idx_dict_family_related ON dict_word_family(related_word_id);

-- Table: dict_word_forms - Word forms (plurals, tenses, comparatives, etc.)
CREATE TABLE IF NOT EXISTS dict_word_forms (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    form_type TEXT CHECK(form_type IN ('plural', 'singular', 'past', 'present', 'future', 'present_participle', 'past_participle', 'comparative', 'superlative', 'adverbial', 'nominalization', 'other')), -- 词形类型：复数、过去式、现在分词等
    form TEXT NOT NULL,                                 -- 词形
    is_irregular BOOLEAN DEFAULT FALSE,                 -- 是否为不规则变形
    notes TEXT,                                         -- 备注
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE INDEX IF NOT EXISTS idx_dict_forms_word ON dict_word_forms(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_forms_type ON dict_word_forms(form_type);

-- Table: dict_word_thesaurus - Thesaurus entries
CREATE TABLE IF NOT EXISTS dict_word_thesaurus (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    entry_type TEXT CHECK(entry_type IN ('synonym', 'antonym', 'related', 'broader', 'narrower', 'part_of', 'member_of', 'substance_of', 'instance_of', 'similar')), -- 条目类型：同义词、反义词、相关词、上位词等
    related_word_id BIGINT NOT NULL,                    -- 相关单词 ID
    semantic_field TEXT,                                 -- 语义场
    relationship_strength REAL CHECK(relationship_strength BETWEEN 0 AND 1), -- 关系强度 (0-1)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(word_id, entry_type, related_word_id)
);

CREATE INDEX IF NOT EXISTS idx_dict_thesaurus_word ON dict_word_thesaurus(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_thesaurus_type ON dict_word_thesaurus(entry_type);

-- Table: dict_frequency_bands - Frequency band classifications
CREATE TABLE IF NOT EXISTS dict_frequency_bands (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    corpus_name TEXT NOT NULL,                          -- 语料库名称
    corpus_type TEXT CHECK(corpus_type IN ('spoken', 'written', 'academic', 'news', 'fiction', 'internet', 'general')), -- 语料库类型：口语、书面语、学术等
    band TEXT CHECK(band IN ('top_1000', 'top_2000', 'top_3000', 'top_5000', 'top_10000', 'beyond_10000')), -- 频率等级：前1000、前2000等
    rank INTEGER,                                       -- 排名
    frequency_per_million REAL,                         -- 每百万词出现频率
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(word_id, corpus_name, corpus_type)
);

CREATE INDEX IF NOT EXISTS idx_dict_frequency_word ON dict_frequency_bands(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_frequency_band ON dict_frequency_bands(band);

-- Table: dict_import_batch - Track import batches
CREATE TABLE IF NOT EXISTS dict_import_batch (
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

CREATE INDEX IF NOT EXISTS idx_dict_import_status ON dict_import_batch(status);
CREATE INDEX IF NOT EXISTS idx_dict_import_created ON dict_import_batch(created_at DESC);

-- Table: dict_word_images - Word images for visual learning
CREATE TABLE IF NOT EXISTS dict_word_images (
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

CREATE INDEX IF NOT EXISTS idx_dict_images_word ON dict_word_images(word_id);

-- Table: dict_related_topics - Topics and themes related to words
CREATE TABLE IF NOT EXISTS dict_related_topics (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    topic_name TEXT NOT NULL,                           -- 主题名称
    topic_category TEXT,                                -- 主题分类
    relevance_score REAL CHECK(relevance_score BETWEEN 0 AND 1), -- 相关性评分 (0-1)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(word_id, topic_name)
);

CREATE INDEX IF NOT EXISTS idx_dict_topics_word ON dict_related_topics(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_topics_name ON dict_related_topics(topic_name);
