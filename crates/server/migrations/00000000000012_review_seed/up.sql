-- Seed data for review page (温故知新)
-- Test data for user_id = 1

-- ============================================================================
-- VOCABULARIES - Words for review
-- ============================================================================

-- Due for review (next_review_at is NULL or in the past, mastery_level < 4)
INSERT INTO learn_vocabularies (user_id, word, word_zh, mastery_level, practice_count, correct_count, next_review_at, last_practiced_at) VALUES
(1, 'accommodation', '住所，住宿', 3, 5, 3, NOW() - INTERVAL '1 day', NOW() - INTERVAL '2 days'),
(1, 'itinerary', '行程表', 2, 3, 1, NOW() - INTERVAL '2 days', NOW() - INTERVAL '3 days'),
(1, 'reservation', '预订', 3, 8, 6, NOW() + INTERVAL '2 hours', NOW() - INTERVAL '1 week'),
(1, 'recommendation', '推荐', 2, 4, 2, NOW() - INTERVAL '1 hour', NOW() - INTERVAL '4 days'),
(1, 'approximately', '大约', 3, 6, 4, NOW(), NOW() - INTERVAL '5 days'),
(1, 'convenience', '便利', 2, 3, 1, NOW() - INTERVAL '3 hours', NOW() - INTERVAL '2 days'),
(1, 'enthusiastic', '热情的', 3, 7, 5, NOW() - INTERVAL '12 hours', NOW() - INTERVAL '3 days'),
(1, 'sophisticated', '复杂的；精致的', 2, 2, 1, NOW(), NOW() - INTERVAL '1 day'),
(1, 'acknowledge', '承认', 3, 5, 3, NOW() - INTERVAL '6 hours', NOW() - INTERVAL '4 days'),
(1, 'comprehensive', '综合的', 2, 4, 2, NOW() - INTERVAL '1 day', NOW() - INTERVAL '5 days'),
(1, 'deteriorate', '恶化', 2, 3, 1, NOW() - INTERVAL '2 days', NOW() - INTERVAL '1 week'),
(1, 'legitimate', '合法的', 3, 5, 4, NOW(), NOW() - INTERVAL '3 days'),
(1, 'preliminary', '初步的', 2, 2, 1, NOW() - INTERVAL '5 hours', NOW() - INTERVAL '2 days'),
(1, 'scrutinize', '仔细检查', 2, 3, 1, NOW() - INTERVAL '1 day', NOW() - INTERVAL '4 days'),
(1, 'substantial', '大量的', 3, 6, 4, NOW() - INTERVAL '3 hours', NOW() - INTERVAL '5 days'),
(1, 'ambiguous', '模棱两可的', 2, 4, 2, NOW(), NOW() - INTERVAL '3 days'),
(1, 'consensus', '共识', 3, 5, 3, NOW() - INTERVAL '8 hours', NOW() - INTERVAL '1 week'),
(1, 'discrepancy', '差异', 2, 3, 2, NOW() - INTERVAL '1 day', NOW() - INTERVAL '4 days'),
(1, 'fluctuate', '波动', 3, 4, 3, NOW() - INTERVAL '4 hours', NOW() - INTERVAL '2 days'),
(1, 'inevitable', '不可避免的', 2, 2, 1, NOW(), NOW() - INTERVAL '3 days'),
(1, 'negligible', '可忽略的', 2, 3, 1, NOW() - INTERVAL '6 hours', NOW() - INTERVAL '5 days'),
(1, 'perpetual', '永久的', 3, 5, 3, NOW() - INTERVAL '2 days', NOW() - INTERVAL '1 week'),
(1, 'spontaneous', '自发的', 2, 4, 2, NOW() - INTERVAL '10 hours', NOW() - INTERVAL '4 days')
ON CONFLICT (user_id, word) DO NOTHING;

-- Mastered words (mastery_level >= 4)
INSERT INTO learn_vocabularies (user_id, word, word_zh, mastery_level, practice_count, correct_count, next_review_at, last_practiced_at) VALUES
(1, 'schedule', '日程安排', 5, 20, 19, NOW() + INTERVAL '30 days', NOW() - INTERVAL '2 weeks'),
(1, 'appointment', '预约', 5, 18, 17, NOW() + INTERVAL '25 days', NOW() - INTERVAL '1 week'),
(1, 'available', '可用的', 5, 25, 24, NOW() + INTERVAL '35 days', NOW() - INTERVAL '3 weeks'),
(1, 'opportunity', '机会', 5, 22, 21, NOW() + INTERVAL '28 days', NOW() - INTERVAL '2 weeks'),
(1, 'experience', '经验', 5, 30, 29, NOW() + INTERVAL '40 days', NOW() - INTERVAL '1 month'),
(1, 'information', '信息', 5, 28, 27, NOW() + INTERVAL '38 days', NOW() - INTERVAL '3 weeks'),
(1, 'important', '重要的', 5, 35, 34, NOW() + INTERVAL '45 days', NOW() - INTERVAL '1 month'),
(1, 'different', '不同的', 5, 32, 31, NOW() + INTERVAL '42 days', NOW() - INTERVAL '2 weeks'),
(1, 'beautiful', '美丽的', 5, 26, 25, NOW() + INTERVAL '36 days', NOW() - INTERVAL '3 weeks'),
(1, 'interesting', '有趣的', 5, 24, 23, NOW() + INTERVAL '33 days', NOW() - INTERVAL '2 weeks'),
(1, 'necessary', '必要的', 4, 15, 13, NOW() + INTERVAL '20 days', NOW() - INTERVAL '1 week'),
(1, 'environment', '环境', 4, 14, 12, NOW() + INTERVAL '18 days', NOW() - INTERVAL '10 days'),
(1, 'development', '发展', 4, 16, 14, NOW() + INTERVAL '22 days', NOW() - INTERVAL '2 weeks'),
(1, 'government', '政府', 4, 13, 11, NOW() + INTERVAL '15 days', NOW() - INTERVAL '1 week'),
(1, 'situation', '情况', 4, 17, 15, NOW() + INTERVAL '24 days', NOW() - INTERVAL '12 days')
ON CONFLICT (user_id, word) DO NOTHING;

-- ============================================================================
-- ISSUE WORDS - Common mistakes
-- ============================================================================

INSERT INTO learn_issue_words (user_id, word, issue_type, description_en, description_zh, difficulty, next_review_at, review_interval_days, context) VALUES
(1, 'affect vs effect', 'usage', 'affect (v.) to influence / effect (n.) result', 'affect (动词) 影响 / effect (名词) 效果', 3, NOW() - INTERVAL '1 day', 3, 'The weather affects my mood. The effect was immediate.'),
(1, 'their vs there', 'usage', 'their (possessive) / there (location)', 'their (他们的) / there (那里)', 2, NOW() - INTERVAL '2 days', 2, 'Their car is over there.'),
(1, 'its vs it''s', 'grammar', 'its (possessive) / it''s (contraction of it is)', 'its (它的) / it''s (它是的缩写)', 2, NOW() - INTERVAL '3 days', 2, 'The cat licked its paw. It''s a beautiful day.'),
(1, 'then vs than', 'usage', 'then (time) / than (comparison)', 'then (然后) / than (比较)', 2, NOW() - INTERVAL '1 day', 3, 'First do this, then that. She is taller than me.'),
(1, 'loose vs lose', 'pronunciation', 'loose /luːs/ (not tight) / lose /luːz/ (misplace)', 'loose (松的) / lose (丢失)', 3, NOW() - INTERVAL '4 days', 4, 'The screw is loose. Don''t lose your keys.'),
(1, 'accept vs except', 'usage', 'accept (receive) / except (exclude)', 'accept (接受) / except (除了)', 3, NOW() - INTERVAL '2 days', 3, 'I accept your offer. Everyone except Tom came.'),
(1, 'principle vs principal', 'usage', 'principle (rule) / principal (main, head)', 'principle (原则) / principal (主要的，校长)', 4, NOW() - INTERVAL '5 days', 5, 'It''s against my principles. The principal spoke at assembly.'),
(1, 'complement vs compliment', 'usage', 'complement (complete) / compliment (praise)', 'complement (补充) / compliment (赞美)', 4, NOW() - INTERVAL '3 days', 4, 'Wine complements the meal. She gave me a compliment.')
ON CONFLICT (user_id, word, issue_type) DO NOTHING;

-- ============================================================================
-- DAILY STATS - Learning statistics
-- ============================================================================

INSERT INTO learn_daily_stats (user_id, stat_date, minutes_studied, words_practiced, sessions_completed, errors_corrected, new_words_learned, review_words_count) VALUES
-- This week (assuming today is the current day)
(1, CURRENT_DATE - INTERVAL '6 days', 45, 15, 2, 3, 5, 10),
(1, CURRENT_DATE - INTERVAL '5 days', 60, 20, 3, 5, 8, 12),
(1, CURRENT_DATE - INTERVAL '4 days', 30, 10, 1, 2, 3, 7),
(1, CURRENT_DATE - INTERVAL '3 days', 75, 25, 4, 6, 10, 15),
(1, CURRENT_DATE - INTERVAL '2 days', 50, 18, 2, 4, 6, 12),
(1, CURRENT_DATE - INTERVAL '1 day', 25, 8, 1, 1, 2, 6),
(1, CURRENT_DATE, 40, 12, 2, 3, 4, 8),
-- Previous weeks for history
(1, CURRENT_DATE - INTERVAL '7 days', 55, 18, 3, 4, 7, 11),
(1, CURRENT_DATE - INTERVAL '8 days', 65, 22, 3, 5, 9, 13),
(1, CURRENT_DATE - INTERVAL '9 days', 35, 12, 2, 2, 4, 8),
(1, CURRENT_DATE - INTERVAL '10 days', 70, 24, 4, 6, 8, 16),
(1, CURRENT_DATE - INTERVAL '11 days', 45, 15, 2, 3, 5, 10),
(1, CURRENT_DATE - INTERVAL '12 days', 80, 28, 4, 7, 12, 16),
(1, CURRENT_DATE - INTERVAL '13 days', 50, 16, 2, 4, 6, 10),
(1, CURRENT_DATE - INTERVAL '14 days', 60, 20, 3, 5, 8, 12),
(1, CURRENT_DATE - INTERVAL '21 days', 55, 18, 3, 4, 7, 11),
(1, CURRENT_DATE - INTERVAL '28 days', 70, 24, 4, 6, 9, 15)
ON CONFLICT (user_id, stat_date) DO NOTHING;
