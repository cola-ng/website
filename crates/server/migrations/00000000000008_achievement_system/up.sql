-- ============================================================================
-- ACHIEVEMENT SYSTEM TABLES
-- ============================================================================

-- Table: achievement_definitions - All available achievements/badges
CREATE TABLE IF NOT EXISTS achievement_definitions (
    id BIGSERIAL PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,                              -- Unique identifier (e.g., 'first_conversation', 'vocab_100')
    name_en TEXT NOT NULL,                                  -- English name
    name_zh TEXT NOT NULL,                                  -- Chinese name
    description_en TEXT,                                    -- English description
    description_zh TEXT,                                    -- Chinese description
    icon TEXT,                                              -- Icon name or URL
    category TEXT NOT NULL CHECK(category IN ('learning', 'social', 'milestone', 'streak', 'mastery', 'special')),
    rarity TEXT NOT NULL DEFAULT 'common' CHECK(rarity IN ('common', 'uncommon', 'rare', 'epic', 'legendary')),
    xp_reward INTEGER NOT NULL DEFAULT 0,                   -- XP earned when unlocked
    requirement_type TEXT NOT NULL CHECK(requirement_type IN ('count', 'streak', 'score', 'time', 'special')),
    requirement_value INTEGER NOT NULL DEFAULT 1,           -- Value needed to unlock (e.g., 100 for vocab_100)
    requirement_field TEXT,                                 -- Field to check (e.g., 'mastered_vocabulary_count')
    is_hidden BOOLEAN DEFAULT FALSE,                        -- Hidden achievements
    is_active BOOLEAN DEFAULT TRUE,
    sort_order INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_achievement_defs_category ON achievement_definitions(category);
CREATE INDEX IF NOT EXISTS idx_achievement_defs_rarity ON achievement_definitions(rarity);
CREATE INDEX IF NOT EXISTS idx_achievement_defs_active ON achievement_definitions(is_active) WHERE is_active = TRUE;

-- Table: rank_definitions - User rank/level definitions
CREATE TABLE IF NOT EXISTS rank_definitions (
    id BIGSERIAL PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,                              -- Unique identifier (e.g., 'beginner', 'intermediate')
    name_en TEXT NOT NULL,                                  -- English name
    name_zh TEXT NOT NULL,                                  -- Chinese name
    description_en TEXT,
    description_zh TEXT,
    icon TEXT,                                              -- Icon/badge for the rank
    color TEXT,                                             -- Theme color for the rank
    min_xp INTEGER NOT NULL DEFAULT 0,                      -- Minimum XP required
    level INTEGER NOT NULL UNIQUE,                          -- Level number (1, 2, 3, etc.)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_rank_defs_level ON rank_definitions(level);
CREATE INDEX IF NOT EXISTS idx_rank_defs_xp ON rank_definitions(min_xp);

-- Table: user_profiles - Extended user profile with XP and stats
CREATE TABLE IF NOT EXISTS user_profiles (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL UNIQUE,                         -- Reference to base_users
    total_xp INTEGER NOT NULL DEFAULT 0,                    -- Total experience points
    current_rank_id BIGINT,                                 -- Current rank (references rank_definitions)
    current_streak_days INTEGER NOT NULL DEFAULT 0,         -- Current learning streak
    longest_streak_days INTEGER NOT NULL DEFAULT 0,         -- Longest streak ever
    last_activity_date DATE,                                -- Last day user was active
    total_study_minutes INTEGER NOT NULL DEFAULT 0,         -- Total minutes studied
    total_words_mastered INTEGER NOT NULL DEFAULT 0,        -- Total words mastered
    total_conversations INTEGER NOT NULL DEFAULT 0,         -- Total conversations completed
    total_sessions INTEGER NOT NULL DEFAULT 0,              -- Total learning sessions
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),           -- When user started learning
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_user_profiles_user ON user_profiles(user_id);
CREATE INDEX IF NOT EXISTS idx_user_profiles_xp ON user_profiles(total_xp DESC);
CREATE INDEX IF NOT EXISTS idx_user_profiles_streak ON user_profiles(current_streak_days DESC);

-- Table: user_achievements - Achievements earned by users (enhanced version)
CREATE TABLE IF NOT EXISTS user_achievements (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    achievement_id BIGINT NOT NULL,                         -- References achievement_definitions
    progress INTEGER NOT NULL DEFAULT 0,                    -- Current progress towards achievement
    is_completed BOOLEAN NOT NULL DEFAULT FALSE,
    completed_at TIMESTAMPTZ,                               -- When achievement was earned
    notified_at TIMESTAMPTZ,                                -- When user was notified
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, achievement_id)
);

CREATE INDEX IF NOT EXISTS idx_user_achievements_user ON user_achievements(user_id, is_completed);
CREATE INDEX IF NOT EXISTS idx_user_achievements_completed ON user_achievements(completed_at DESC) WHERE is_completed = TRUE;

-- Table: user_xp_history - Track XP gains
CREATE TABLE IF NOT EXISTS user_xp_history (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    xp_amount INTEGER NOT NULL,
    source_type TEXT NOT NULL CHECK(source_type IN ('achievement', 'session', 'daily_bonus', 'streak', 'review', 'special')),
    source_id BIGINT,                                       -- Reference to the source (achievement_id, session_id, etc.)
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_user_xp_history_user ON user_xp_history(user_id, created_at DESC);

-- ============================================================================
-- SEED DATA: Achievement Definitions
-- ============================================================================

INSERT INTO achievement_definitions (code, name_en, name_zh, description_en, description_zh, icon, category, rarity, xp_reward, requirement_type, requirement_value, requirement_field, sort_order) VALUES
-- Learning Milestones
('first_conversation', 'First Chat', '初次对话', 'Complete your first conversation', '完成你的第一次对话', 'message-circle', 'milestone', 'common', 10, 'count', 1, 'total_conversations', 1),
('conversations_10', 'Chatty Learner', '健谈学者', 'Complete 10 conversations', '完成10次对话', 'message-circle', 'milestone', 'common', 50, 'count', 10, 'total_conversations', 2),
('conversations_50', 'Social Butterfly', '社交达人', 'Complete 50 conversations', '完成50次对话', 'messages', 'milestone', 'uncommon', 150, 'count', 50, 'total_conversations', 3),
('conversations_100', 'Conversation Master', '对话大师', 'Complete 100 conversations', '完成100次对话', 'crown', 'milestone', 'rare', 300, 'count', 100, 'total_conversations', 4),

-- Vocabulary Achievements
('vocab_first', 'First Word', '首个单词', 'Learn your first word', '学会你的第一个单词', 'book-open', 'learning', 'common', 10, 'count', 1, 'total_words_mastered', 10),
('vocab_50', 'Word Collector', '单词收集者', 'Master 50 words', '掌握50个单词', 'library', 'learning', 'common', 100, 'count', 50, 'total_words_mastered', 11),
('vocab_100', 'Vocabulary Builder', '词汇建设者', 'Master 100 words', '掌握100个单词', 'book-marked', 'learning', 'uncommon', 200, 'count', 100, 'total_words_mastered', 12),
('vocab_500', 'Word Wizard', '单词魔法师', 'Master 500 words', '掌握500个单词', 'sparkles', 'learning', 'rare', 500, 'count', 500, 'total_words_mastered', 13),
('vocab_1000', 'Lexicon Legend', '词典传奇', 'Master 1000 words', '掌握1000个单词', 'trophy', 'learning', 'epic', 1000, 'count', 1000, 'total_words_mastered', 14),

-- Streak Achievements
('streak_3', 'Getting Started', '坚持不懈', '3-day learning streak', '连续学习3天', 'flame', 'streak', 'common', 30, 'streak', 3, 'current_streak_days', 20),
('streak_7', 'Weekly Warrior', '周学习战士', '7-day learning streak', '连续学习7天', 'flame', 'streak', 'uncommon', 100, 'streak', 7, 'current_streak_days', 21),
('streak_30', 'Monthly Master', '月度大师', '30-day learning streak', '连续学习30天', 'flame', 'streak', 'rare', 500, 'streak', 30, 'current_streak_days', 22),
('streak_100', 'Century Learner', '百日学者', '100-day learning streak', '连续学习100天', 'flame', 'streak', 'epic', 2000, 'streak', 100, 'current_streak_days', 23),
('streak_365', 'Year of Dedication', '年度奉献', '365-day learning streak', '连续学习365天', 'star', 'streak', 'legendary', 10000, 'streak', 365, 'current_streak_days', 24),

-- Study Time Achievements
('time_60', 'Hour Hero', '一小时英雄', 'Study for 1 hour total', '累计学习1小时', 'clock', 'mastery', 'common', 20, 'time', 60, 'total_study_minutes', 30),
('time_300', 'Dedicated Student', '专注学生', 'Study for 5 hours total', '累计学习5小时', 'clock', 'mastery', 'common', 50, 'time', 300, 'total_study_minutes', 31),
('time_600', 'Persistent Learner', '坚韧学习者', 'Study for 10 hours total', '累计学习10小时', 'clock', 'mastery', 'uncommon', 100, 'time', 600, 'total_study_minutes', 32),
('time_3000', 'Time Investor', '时间投资者', 'Study for 50 hours total', '累计学习50小时', 'hourglass', 'mastery', 'rare', 500, 'time', 3000, 'total_study_minutes', 33),
('time_6000', 'Century of Learning', '学习世纪', 'Study for 100 hours total', '累计学习100小时', 'hourglass', 'mastery', 'epic', 1500, 'time', 6000, 'total_study_minutes', 34),

-- Special Achievements
('early_bird', 'Early Bird', '早起鸟儿', 'Complete a session before 7 AM', '早上7点前完成一次学习', 'sun', 'special', 'uncommon', 50, 'special', 1, NULL, 40),
('night_owl', 'Night Owl', '夜猫子', 'Complete a session after 11 PM', '晚上11点后完成一次学习', 'moon', 'special', 'uncommon', 50, 'special', 1, NULL, 41),
('perfect_score', 'Perfectionist', '完美主义者', 'Get 100% on a reading practice', '跟读练习获得满分', 'target', 'special', 'rare', 100, 'score', 100, NULL, 42),
('weekend_warrior', 'Weekend Warrior', '周末战士', 'Study on both Saturday and Sunday', '周六周日都有学习', 'calendar', 'special', 'common', 30, 'special', 1, NULL, 43)
ON CONFLICT (code) DO NOTHING;

-- ============================================================================
-- SEED DATA: Rank Definitions
-- ============================================================================

INSERT INTO rank_definitions (code, name_en, name_zh, description_en, description_zh, icon, color, min_xp, level) VALUES
('novice', 'Novice', '新手', 'Just starting your English journey', '刚开始英语学习之旅', 'seedling', '#9CA3AF', 0, 1),
('beginner', 'Beginner', '初学者', 'Taking your first steps', '迈出第一步', 'sprout', '#6EE7B7', 100, 2),
('elementary', 'Elementary', '初级', 'Building your foundation', '打好基础', 'leaf', '#34D399', 300, 3),
('intermediate', 'Intermediate', '中级', 'Making steady progress', '稳步进步', 'tree', '#10B981', 800, 4),
('upper_intermediate', 'Upper Intermediate', '中高级', 'Becoming confident', '越来越自信', 'mountain', '#059669', 1500, 5),
('advanced', 'Advanced', '高级', 'Mastering the language', '精通语言', 'summit', '#F59E0B', 3000, 6),
('proficient', 'Proficient', '熟练', 'Near-native fluency', '接近母语水平', 'star', '#EF4444', 6000, 7),
('master', 'Master', '大师', 'English master', '英语大师', 'crown', '#8B5CF6', 10000, 8),
('legend', 'Legend', '传奇', 'Legendary English speaker', '传奇英语达人', 'gem', '#EC4899', 20000, 9)
ON CONFLICT (code) DO NOTHING;
