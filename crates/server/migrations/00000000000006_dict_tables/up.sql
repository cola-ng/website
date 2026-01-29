-- ============================================================================
-- DICTIONARY TABLES (English Dictionary - Server-side)
-- ============================================================================

-- Table: dict_dictionaries - Dictionary metadata]
drop table if exists dict_dictionaries;
CREATE TABLE IF NOT EXISTS dict_dictionaries (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    name_en TEXT NOT NULL UNIQUE,               -- 字典英语名称
    name_zh TEXT NOT NULL UNIQUE,               -- 字典中文名称
    short_en TEXT NOT NULL UNIQUE,        -- 英文短名称
    short_zh TEXT NOT NULL UNIQUE,        -- 中文短名称
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
    priority_order INTEGER,                 -- 优先级（越小越优先）
    created_by BIGINT,                                  -- 创建者用户 ID
    updated_by BIGINT,                                  -- 更新者用户 ID
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 更新时间
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);
CREATE INDEX IF NOT EXISTS idx_dict_dictionaries_name_en ON dict_dictionaries(name_en);
CREATE INDEX IF NOT EXISTS idx_dict_dictionaries_name_zh ON dict_dictionaries(name_zh);
CREATE INDEX IF NOT EXISTS idx_dict_dictionaries_active ON dict_dictionaries(is_active);
CREATE INDEX IF NOT EXISTS idx_dict_dictionaries_priority ON dict_dictionaries(priority_order ASC);

-- Table: dict_words - Main word table, 单词表, 也包含了短语
drop table if exists dict_words;
CREATE TABLE IF NOT EXISTS dict_words (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word TEXT NOT NULL UNIQUE,                          -- 单词（原形式）
    word_lower TEXT NOT NULL,                           -- 单词小写形式，用于搜索
    word_type TEXT CHECK(word_type IN ('noun', 'verb', 'adjective', 'adverb', 'pronoun', 'preposition', 'conjunction', 'interjection', 'article', 'abbreviation', 'phrase', 'idiom')), -- 词性：名词、动词、形容词、副词等
    language TEXT DEFAULT 'en',                         -- 语言（默认英文）
    frequency SMALLINT CHECK(frequency BETWEEN 0 AND 100), -- 频率评分 (0-100)，越高表示越常用
    difficulty SMALLINT,-- CHECK(difficulty BETWEEN 1 AND 5), -- 难度等级 (1-5)，1 最简单，5 最难
    syllable_count SMALLINT,                   -- 音节数量
    is_lemma BOOLEAN DEFAULT TRUE,                      -- 是否为词元（词根/原形）
    word_count INTEGER,                       -- 单词数量
    is_active BOOLEAN DEFAULT TRUE,                     -- 是否激活（可用状态）
    memory TEXT,                                       -- 记忆提示
    notes TEXT,                                        -- 备注
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
drop table if exists dict_word_dictionaries;
CREATE TABLE IF NOT EXISTS dict_word_dictionaries (
    id BIGSERIAL PRIMARY KEY,                          -- Primary key ID
    word_id BIGINT NOT NULL,                            -- Associated word ID
    dictionary_id BIGINT NOT NULL,                      -- Associated dictionary ID
    definition_id BIGINT,                      -- Associated definition ID
    priority_order INTEGER,
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
    region TEXT, -- 地区：美式、英式、澳式等
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
    dialect TEXT, -- 口音：美式、英式、澳式等
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
    difficulty INTEGER,-- CHECK(difficulty BETWEEN 1 AND 5), -- 难度等级 (1-5)
    is_common BOOLEAN DEFAULT FALSE,                    -- 是否为常用例句
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE TABLE IF NOT EXISTS dict_word_sentences (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    definition_id BIGINT,                               -- 关联释义 ID
    sentence_id BIGINT NOT NULL,                               -- 关联释义 ID
    priority_order INTEGER,                    -- 例句顺序
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);

CREATE Unique INDEX IF NOT EXISTS idx_dict_sentences_word ON dict_word_sentences(word_id, sentence_id);
CREATE INDEX IF NOT EXISTS idx_dict_sentences_definition ON dict_word_sentences(definition_id);

-- Table: dict_etymology - etymology information 语源；词源学
drop table if exists dict_etymologies;
CREATE TABLE IF NOT EXISTS dict_etymologies (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    origin_language TEXT,                               -- 起源语言
    origin_word TEXT,                                   -- 起源单词
    origin_meaning TEXT,                                -- 起源含义
    language TEXT NOT NULL,                             -- 词源说明语言
    etymology TEXT NOT NULL,                            -- 词源说明
    first_attested_year TEXT,                        -- 首次出现的年份
    historical_forms JSONB,                              -- 历史形式 (JSONB)
    cognate_words JSONB,                                -- 同源词 (JSONB)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);
-- 一个单词可能来源于不同的语源
CREATE TABLE IF NOT EXISTS dict_word_etymologies (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    etymology_id BIGINT NOT NULL,                       -- 关联语源 ID
    priority_order INTEGER,                    -- 例句顺序
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()       -- 创建时间
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_dict_etymology_word ON dict_word_etymologies(word_id, etymology_id);

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
    range_begin SMALLINT NOT NULL,                      -- 单词在短语中的起始位置(包括)
    range_until SMALLINT,                      -- 单词在短语中的结束位置(包括)
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
    relation_strength SMALLINT DEFAULT 50 CHECK(relation_strength BETWEEN 0 AND 100), -- 关系强度 (0-100)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),      -- 创建时间
    UNIQUE(word_id, relation_type, related_word_id)
);

CREATE INDEX IF NOT EXISTS idx_dict_relations_word ON dict_relations(word_id);
CREATE INDEX IF NOT EXISTS idx_dict_relations_type ON dict_relations(relation_type);


-- Table: dict_frequency_bands - Frequency band classifications
drop table if exists dict_frequencies;
CREATE TABLE IF NOT EXISTS dict_frequencies (
    id BIGSERIAL PRIMARY KEY,                          -- 主键 ID
    word_id BIGINT NOT NULL,                            -- 关联单词 ID
    corpus_name TEXT NOT NULL,                          -- 语料库名称
    corpus_type TEXT CHECK(corpus_type IN ('spoken', 'written', 'academic', 'news', 'fiction', 'internet', 'general')), -- 语料库类型：口语、书面语、学术等
    band TEXT CHECK(band IN ('top_1000', 'top_2000', 'top_3000', 'top_5000', 'top_10000', 'beyond_10000')), -- 频率等级：前1000、前2000等
    rank INTEGER,                                       -- 排名
    per_million REAL,                         -- 每百万词出现频率
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

-- Dictionary search history table
drop table if exists dict_searched_words;
CREATE TABLE IF NOT EXISTS dict_searched_words (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL DEFAULT 1,
    word_id BIGINT,
    word VARCHAR(255) NOT NULL,
    searched_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for efficient queries
CREATE INDEX idx_dict_searched_words_user ON dict_searched_words(user_id, searched_at DESC);
CREATE INDEX idx_dict_searched_words_word ON dict_searched_words(word);


-- Seed data for dict_dictionaries table
-- This migration populates the database with dictionary metadata

-- ============================================================================
-- EXAM-BASED DICTIONARIES (Chinese National Exams)
-- ============================================================================

INSERT INTO dict_dictionaries (
    name_en, name_zh, short_en, short_zh,
    description_en, description_zh,
    version, publisher, license_type, license_url, source_url,
    total_entries, is_active, is_official, priority_order
) VALUES
(
    'College English Test Band 4 Vocabulary',
    '大学英语四级考试词汇',
    'CET4',
    '四级',
    'Official vocabulary list for the College English Test Band 4, a standardized English proficiency test for non-English major undergraduate students in China.',
    '大学英语四级考试官方词汇表，适用于中国非英语专业本科生的标准化英语水平测试。',
    '2023',
    'National Education Examinations Authority',
    'educational',
    NULL,
    'https://cet.neea.edu.cn',
    4500,
    TRUE,
    TRUE,
    1
),
(
    'College English Test Band 6 Vocabulary',
    '大学英语六级考试词汇',
    'CET6',
    '六级',
    'Official vocabulary list for the College English Test Band 6, a higher-level standardized English proficiency test for non-English major undergraduate students in China.',
    '大学英语六级考试官方词汇表，适用于中国非英语专业本科生的高级标准化英语水平测试。',
    '2023',
    'National Education Examinations Authority',
    'educational',
    NULL,
    'https://cet.neea.edu.cn',
    6000,
    TRUE,
    TRUE,
    2
),
(
    'Test for English Majors Band 4 Vocabulary',
    '英语专业四级考试词汇',
    'TEM4',
    '专四',
    'Official vocabulary list for the Test for English Majors Band 4 (TEM-4), a national English examination for English major students in their second year of university in China.',
    '英语专业四级考试官方词汇表，中国英语专业学生大学二年级参加的国家英语考试。',
    '2023',
    'National Education Examinations Authority',
    'educational',
    NULL,
    'https://tem.fltrp.com',
    8000,
    TRUE,
    TRUE,
    3
),
(
    'Test for English Majors Band 8 Vocabulary',
    '英语专业八级考试词汇',
    'TEM8',
    '专八',
    'Official vocabulary list for the Test for English Majors Band 8 (TEM-8), the highest-level national English examination for English major students in their fourth year of university in China.',
    '英语专业八级考试官方词汇表，中国英语专业学生大学四年级参加的最高级别国家英语考试。',
    '2023',
    'National Education Examinations Authority',
    'educational',
    NULL,
    'https://tem.fltrp.com',
    13000,
    TRUE,
    TRUE,
    4
),
(
    'National Entrance Examination for Postgraduate English Vocabulary',
    '考研英语词汇',
    'NEEP',
    '考研',
    'Official vocabulary list for the English section of the National Entrance Examination for Postgraduate (NEEP), required for graduate school admission in China.',
    '全国硕士研究生入学考试英语部分官方词汇表，中国研究生入学必考科目。',
    '2024',
    'Ministry of Education of China',
    'educational',
    NULL,
    'https://yz.chsi.com.cn',
    5500,
    TRUE,
    TRUE,
    5
);

-- ============================================================================
-- AUTHORITATIVE ENGLISH DICTIONARIES
-- ============================================================================

INSERT INTO dict_dictionaries (
    name_en, name_zh, short_en, short_zh,
    description_en, description_zh,
    version, publisher, license_type, license_url, source_url,
    total_entries, is_active, is_official, priority_order
) VALUES
(
    'Oxford English Dictionary',
    '牛津英语词典',
    'OED',
    '牛津',
    'The Oxford English Dictionary (OED) is the principal historical dictionary of the English language, published by Oxford University Press. It traces the historical development of the English language with comprehensive etymological analysis.',
    '牛津英语词典是英语的主要历史词典，由牛津大学出版社出版。它追溯英语的历史发展，并提供全面的词源分析。',
    '3rd Edition',
    'Oxford University Press',
    'proprietary',
    'https://www.oed.com/information/terms-of-use',
    'https://www.oed.com',
    600000,
    TRUE,
    TRUE,
    10
),
(
    'Webster''s Third New International Dictionary',
    '韦氏第三版新国际词典',
    'Webster',
    '韦氏',
    'Webster''s Third New International Dictionary is an American dictionary published by Merriam-Webster. It is one of the most comprehensive dictionaries of American English.',
    '韦氏第三版新国际词典是由梅里亚姆-韦伯斯特出版的美式英语词典，是最全面的美式英语词典之一。',
    '3rd Edition',
    'Merriam-Webster',
    'proprietary',
    'https://www.merriam-webster.com/terms-of-use',
    'https://www.merriam-webster.com',
    470000,
    TRUE,
    TRUE,
    11
),
(
    'Collins COBUILD Advanced English-Chinese Bilingual Learning Dictionary',
    '柯林斯COBUILD高阶英汉双解学习词典',
    'Collins',
    '柯林斯',
    'Collins COBUILD Advanced English-Chinese Bilingual Learning Dictionary is designed for learners of English as a foreign language, featuring full-sentence definitions and extensive example sentences from the Bank of English corpus.',
    '柯林斯COBUILD高阶英汉双解学习词典专为英语学习者设计，采用完整句子定义，并提供来自英语语料库的丰富例句。',
    '9th Edition',
    'HarperCollins Publishers',
    'proprietary',
    'https://www.collinsdictionary.com/terms-and-conditions',
    'https://www.collinsdictionary.com',
    110000,
    TRUE,
    TRUE,
    12
),
(
    'Cambridge Advanced Learner''s Dictionary',
    '剑桥高级英语学习者词典',
    'CALD',
    '剑桥',
    'Cambridge Advanced Learner''s Dictionary (CALD) is a dictionary for learners of English published by Cambridge University Press. It includes clear definitions, example sentences, and guidance on grammar, collocation and usage.',
    '剑桥高级英语学习者词典是剑桥大学出版社出版的英语学习者词典，提供清晰的定义、例句以及语法、搭配和用法指导。',
    '4th Edition',
    'Cambridge University Press',
    'proprietary',
    'https://dictionary.cambridge.org/about/terms/',
    'https://dictionary.cambridge.org',
    140000,
    TRUE,
    TRUE,
    13
),
(
    'Macmillan English Dictionary for Advanced Learners',
    '麦克米伦高阶英语词典',
    'Macmillan',
    '麦克米伦',
    'Macmillan English Dictionary for Advanced Learners (MEDAL) is a dictionary for learners of English, featuring frequency information showing the most important words to learn and a unique metaphor awareness feature.',
    '麦克米伦高阶英语词典是为英语学习者编写的词典，提供词频信息以显示最重要的学习词汇，并具有独特的隐喻意识功能。',
    '2nd Edition',
    'Macmillan Education',
    'proprietary',
    'https://www.macmillandictionary.com/terms-and-conditions',
    'https://www.macmillandictionary.com',
    100000,
    TRUE,
    TRUE,
    14
);
