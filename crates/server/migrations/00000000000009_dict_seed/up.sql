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
