-- Restore the removed achievements (this is a best-effort restoration)
-- Note: User progress for deleted achievements cannot be restored

INSERT INTO archive_achievement_definitions (code, name_en, name_zh, description_en, description_zh, icon, category, rarity, xp_reward, requirement_type, requirement_value, requirement_field, sort_order) VALUES
('conversations_10', 'Chatty Learner', '健谈学者', 'Complete 10 conversations', '完成10次对话', 'message-circle', 'milestone', 'common', 50, 'count', 10, 'total_stages', 2),
('conversations_50', 'Social Butterfly', '社交达人', 'Complete 50 conversations', '完成50次对话', 'messages', 'milestone', 'uncommon', 150, 'count', 50, 'total_stages', 3),
('vocab_first', 'First Word', '首个单词', 'Learn your first word', '学会你的第一个单词', 'book-open', 'learning', 'common', 10, 'count', 1, 'total_words_mastered', 10),
('vocab_50', 'Word Collector', '单词收集者', 'Master 50 words', '掌握50个单词', 'library', 'learning', 'common', 100, 'count', 50, 'total_words_mastered', 11),
('vocab_500', 'Word Wizard', '单词魔法师', 'Master 500 words', '掌握500个单词', 'sparkles', 'learning', 'rare', 500, 'count', 500, 'total_words_mastered', 13),
('streak_3', 'Getting Started', '坚持不懈', '3-day learning streak', '连续学习3天', 'flame', 'streak', 'common', 30, 'streak', 3, 'current_streak_days', 20),
('streak_100', 'Century Learner', '百日学者', '100-day learning streak', '连续学习100天', 'flame', 'streak', 'epic', 2000, 'streak', 100, 'current_streak_days', 23),
('streak_365', 'Year of Dedication', '年度奉献', '365-day learning streak', '连续学习365天', 'star', 'streak', 'legendary', 10000, 'streak', 365, 'current_streak_days', 24),
('time_60', 'Hour Hero', '一小时英雄', 'Study for 1 hour total', '累计学习1小时', 'clock', 'mastery', 'common', 20, 'time', 60, 'total_study_minutes', 30),
('time_300', 'Dedicated Student', '专注学生', 'Study for 5 hours total', '累计学习5小时', 'clock', 'mastery', 'common', 50, 'time', 300, 'total_study_minutes', 31),
('time_3000', 'Time Investor', '时间投资者', 'Study for 50 hours total', '累计学习50小时', 'hourglass', 'mastery', 'rare', 500, 'time', 3000, 'total_study_minutes', 33),
('time_6000', 'Century of Learning', '学习世纪', 'Study for 100 hours total', '累计学习100小时', 'hourglass', 'mastery', 'epic', 1500, 'time', 6000, 'total_study_minutes', 34),
('early_bird', 'Early Bird', '早起鸟儿', 'Complete a session before 7 AM', '早上7点前完成一次学习', 'sun', 'special', 'uncommon', 50, 'special', 1, NULL, 40),
('night_owl', 'Night Owl', '夜猫子', 'Complete a session after 11 PM', '晚上11点后完成一次学习', 'moon', 'special', 'uncommon', 50, 'special', 1, NULL, 41),
('weekend_warrior', 'Weekend Warrior', '周末战士', 'Study on both Saturday and Sunday', '周六周日都有学习', 'calendar', 'special', 'common', 30, 'special', 1, NULL, 43)
ON CONFLICT (code) DO NOTHING;

-- Restore original values
UPDATE archive_achievement_definitions SET
    xp_reward = 10,
    sort_order = 1
WHERE code = 'first_conversation';

UPDATE archive_achievement_definitions SET
    xp_reward = 300,
    sort_order = 4
WHERE code = 'conversations_100';

UPDATE archive_achievement_definitions SET
    name_zh = '词汇建设者',
    xp_reward = 200,
    sort_order = 12
WHERE code = 'vocab_100';

UPDATE archive_achievement_definitions SET
    name_zh = '词典传奇',
    xp_reward = 1000,
    sort_order = 14
WHERE code = 'vocab_1000';

UPDATE archive_achievement_definitions SET
    xp_reward = 100,
    sort_order = 21
WHERE code = 'streak_7';

UPDATE archive_achievement_definitions SET
    xp_reward = 500,
    sort_order = 22
WHERE code = 'streak_30';

UPDATE archive_achievement_definitions SET
    name_zh = '坚韧学习者',
    xp_reward = 100,
    sort_order = 32
WHERE code = 'time_600';

UPDATE archive_achievement_definitions SET
    xp_reward = 100,
    sort_order = 42
WHERE code = 'perfect_score';
