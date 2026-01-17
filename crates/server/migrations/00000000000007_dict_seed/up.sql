-- ============================================================================
-- DICTIONARY SEED DATA
-- ============================================================================
-- This migration seeds the dictionary tables with data from various dictionary sources
--
-- Dictionary sources:
-- 1. 英汉词根辞典(李平武+蒋真,7291条) - Chinese root word dictionary
-- 2. Youdict优词英语词根词源词典(18677条) - Youdict root etymology dictionary
-- 3. Etym Word Origins Dictionary词源词典(46104条) - English etymology dictionary
-- 4. 英语词根词源记忆词典(31503条) - Root word memory dictionary
-- 5. 简明英汉字典增强版(3407926条) - Enhanced English-Chinese dictionary
-- 6. Other specialized dictionaries
--
-- Note: Due to the large size and complexity of dictionary files,
-- this migration provides structure. The actual data import should be
-- performed using the import script: src/bin/import_dict_data.rs
-- ============================================================================

-- Create a batch record for this import
INSERT INTO dict_import_batch (batch_name, source, source_type, status, metadata, started_at)
VALUES (
    'dictionary_seed_v1',
    'Multiple dictionary files from D:\Works\colang\dicts',
    'file',
    'pending',
    '{"dictionaries": ["英汉词根辞典", "Youdict词根词源词典", "Etym词源词典", "英语词根词源记忆词典", "简明英汉字典增强版"], "description": "Initial dictionary seed data"}'::jsonb,
    now()
)
ON CONFLICT DO NOTHING;

-- ============================================================================
-- SAMPLE DATA: Root words from Chinese etymology dictionary
-- This is a small sample to demonstrate the data structure
-- ============================================================================

-- Sample root words with etymology information
-- Format from: 英汉词根辞典(李平武+蒋真,7291条)汇总.txt

INSERT INTO dict_words (word, word_lower, word_type, difficulty, is_lemma, created_at) VALUES
('abase', 'abase', 'verb', 3, true, now()),
('abate', 'abate', 'verb', 3, true, now()),
('abbreviate', 'abbreviate', 'verb', 3, true, now()),
('abduct', 'abduct', 'verb', 3, true, now()),
('abnormal', 'abnormal', 'adjective', 2, true, now()),
('aboriginal', 'aboriginal', 'adjective', 4, true, now()),
('abrade', 'abrade', 'verb', 4, true, now()),
('absorb', 'absorb', 'verb', 2, true, now()),
('accelerate', 'accelerate', 'verb', 3, true, now()),
('access', 'access', 'noun', 2, true, now())
ON CONFLICT (word) DO NOTHING;

-- Sample etymology data
INSERT INTO dict_word_etymology (word_id, origin_language, origin_word, origin_meaning, etymology_zh, created_at)
SELECT
    id,
    'Latin',
    CASE word
        WHEN 'abase' THEN 'bassus'
        WHEN 'abate' THEN 'battere'
        WHEN 'abbreviate' THEN 'brevis'
        WHEN 'abduct' THEN 'ducere'
        WHEN 'abnormal' THEN 'norma'
        WHEN 'aboriginal' THEN 'oriri'
        WHEN 'abrade' THEN 'radere'
        WHEN 'absorb' THEN'sorbere'
        WHEN 'accelerate' THEN 'celer'
        WHEN 'access' THEN 'cedere'
    END,
    CASE word
        WHEN 'abase' THEN 'low'
        WHEN 'abate' THEN 'to beat'
        WHEN 'abbreviate' THEN 'short'
        WHEN 'abduct' THEN 'to lead'
        WHEN 'abnormal' THEN 'rule'
        WHEN 'aboriginal' THEN 'to rise'
        WHEN 'abrade' THEN 'to scrape'
        WHEN 'absorb' THEN 'to suck in'
        WHEN 'accelerate' THEN 'swift'
        WHEN 'access' THEN 'to go'
    END,
    CASE word
        WHEN 'abase' THEN '[李] abase [a-;bas;-e] v.使谦卑，使降低地位 ←【词根】: bas, bass (L bassus)=low 低的'
        WHEN 'abate' THEN '[李] abate [a-;bat;-e] v.减少； 减轻 ←【词根】: bat, batt (L battere)=to beat 打击'
        WHEN 'abbreviate' THEN '[李] abbreviate [ab-;brev;-i-;-ate v.] v.缩写； 简略 ←【词根】: brev (L brevis)=short, shallow 短，浅'
        WHEN 'abduct' THEN '[李] abduct [ab-=away from离开；duct=to lead引导→"to lead away from home引离家庭"→]'
        WHEN 'abnormal' THEN '[李] abnormal [ab-;norm;-al a.] a.反常的，变态的n.反常，变态 ←【词根】: norm (L norma)=rule 规则'
        WHEN 'aboriginal' THEN '[李] aboriginal [ab-;origin;-al a.] a.土著的； 原住的 n.土著居民 ←【词根】: ori, ort (L oriri,ortus)=to rise 上升'
        WHEN 'abrade' THEN '[李] abrade [ab-;rad;-e] v.磨，磨掉； 擦伤 ←【词根】: rad, ras (L radere, rasum)=to scrape 擦刮'
        WHEN 'absorb' THEN '[词根]: sorb (L sorbere)=to suck in 吸入'
        WHEN 'accelerate' THEN '[词根]: celer (L celer)=swift 快速'
        WHEN 'access' THEN '[词根]: cess (L cedere)=to go 走'
    END,
    now()
FROM dict_words
WHERE word IN ('abase', 'abate', 'abbreviate', 'abduct', 'abnormal', 'aboriginal', 'abrade', 'absorb', 'accelerate', 'access')
ON CONFLICT DO NOTHING;

-- Sample definitions
INSERT INTO dict_word_definitions (word_id, definition_en, definition_zh, part_of_speech, is_primary, definition_order, created_at)
SELECT
    id,
    CASE word
        WHEN 'abase' THEN 'To humiliate or degrade; to lower in position or rank'
        WHEN 'abate' THEN 'To become less intense; to decrease or reduce'
        WHEN 'abbreviate' THEN 'To shorten a word or phrase'
        WHEN 'abduct' THEN 'To take away by fraud or violence; kidnap'
        WHEN 'abnormal' THEN 'Deviating from the normal or average'
        WHEN 'aboriginal' THEN 'Inhabiting or existing in a land from the earliest times'
        WHEN 'abrade' THEN 'To wear away or rub off by friction'
        WHEN 'absorb' THEN 'To take in or soak up'
        WHEN 'accelerate' THEN 'To increase in speed or rate'
        WHEN 'access' THEN 'The means or opportunity to approach or enter a place'
    END,
    CASE word
        WHEN 'abase' THEN '使谦卑，使降低地位'
        WHEN 'abate' THEN '减少；减轻'
        WHEN 'abbreviate' THEN '缩写；简略'
        WHEN 'abduct' THEN '诱拐；劫持'
        WHEN 'abnormal' THEN '反常的，变态的'
        WHEN 'aboriginal' THEN '土著的；原住的'
        WHEN 'abrade' THEN '磨，磨掉；擦伤'
        WHEN 'absorb' THEN '吸收；同化'
        WHEN 'accelerate' THEN '加速；促进'
        WHEN 'access' THEN '接近；通道；使用权'
    END,
    CASE word
        WHEN 'abase' THEN 'verb'
        WHEN 'abate' THEN 'verb'
        WHEN 'abbreviate' THEN 'verb'
        WHEN 'abduct' THEN 'verb'
        WHEN 'abnormal' THEN 'adjective'
        WHEN 'aboriginal' THEN 'adjective'
        WHEN 'abrade' THEN 'verb'
        WHEN 'absorb' THEN 'verb'
        WHEN 'accelerate' THEN 'verb'
        WHEN 'access' THEN 'noun'
    END,
    true,
    1,
    now()
FROM dict_words
WHERE word IN ('abase', 'abate', 'abbreviate', 'abduct', 'abnormal', 'aboriginal', 'abrade', 'absorb', 'accelerate', 'access')
ON CONFLICT DO NOTHING;

-- Sample example sentences
INSERT INTO dict_word_examples (word_id, sentence_en, sentence_zh, example_order, is_common, created_at)
SELECT
    id,
    CASE word
        WHEN 'abase' THEN 'He refused to abase himself before his tormentors.'
        WHEN 'abate' THEN 'The storm suddenly abated.'
        WHEN 'abbreviate' THEN 'We can abbreviate "Monday" to "Mon".'
        WHEN 'abduct' THEN 'The man abducted a boy for ransom.'
        WHEN 'abnormal' THEN 'This abnormal behavior worries me.'
        WHEN 'aboriginal' THEN 'The aboriginal people of Australia have a rich culture.'
        WHEN 'abrade' THEN 'The sharp rocks will abrade the skin.'
        WHEN 'absorb' THEN 'Plants absorb water through their roots.'
        WHEN 'accelerate' THEN 'The car began to accelerate.'
        WHEN 'access' THEN 'Students need access to computers.'
    END,
    CASE word
        WHEN 'abase' THEN '他拒绝在折磨他的人面前卑躬屈膝。'
        WHEN 'abate' THEN '暴风雨突然减弱了。'
        WHEN 'abbreviate' THEN '我们可以将"Monday"缩写成"Mon"。'
        WHEN 'abduct' THEN '那人诱拐了一个男孩以求勒索赎金。'
        WHEN 'abnormal' THEN '这种反常的行为让我担心。'
        WHEN 'aboriginal' THEN '澳大利亚的土著居民拥有丰富的文化。'
        WHEN 'abrade' THEN '尖锐的岩石会擦伤皮肤。'
        WHEN 'absorb' THEN '植物通过根系吸收水分。'
        WHEN 'accelerate' THEN '汽车开始加速。'
        WHEN 'access' THEN '学生需要使用电脑的机会。'
    END,
    1,
    true,
    now()
FROM dict_words
WHERE word IN ('abase', 'abate', 'abbreviate', 'abduct', 'abnormal', 'aboriginal', 'abrade', 'absorb', 'accelerate', 'access')
ON CONFLICT DO NOTHING;

-- ============================================================================
-- NOTES FOR FULL IMPORT
-- ============================================================================
--
-- To import the full dictionary data:
--
-- 1. Run the import script:
--    cargo run --bin import_dict_data
--
-- 2. The script will:
--    - Parse all dictionary files from D:\Works\colang\dicts
--    - Generate appropriate INSERT or COPY statements
--    - Handle deduplication and data validation
--    - Update the dict_import_batch status
--
-- 3. Dictionary file formats supported:
--    - Tab-separated: word\tetymology/definition
--    - Space-separated: word [spaces] word [spaces] definition
--    - Multi-line: word followed by detailed etymology
--
-- 4. For large imports (>100k records), consider using:
--    - COPY commands for bulk data loading
--    - Transaction batching (commit every 1000 records)
--    - Parallel processing for independent tables
--
-- ============================================================================
