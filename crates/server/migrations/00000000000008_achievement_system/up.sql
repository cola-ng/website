-- ============================================================================
-- ACHIEVEMENT SYSTEM TABLES
-- ============================================================================

-- Table: archive_achievement_definitions - All available achievements/badges
CREATE TABLE IF NOT EXISTS archive_achievement_definitions (
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

CREATE INDEX IF NOT EXISTS idx_achievement_defs_category ON archive_achievement_definitions(category);
CREATE INDEX IF NOT EXISTS idx_achievement_defs_rarity ON archive_achievement_definitions(rarity);
CREATE INDEX IF NOT EXISTS idx_achievement_defs_active ON archive_achievement_definitions(is_active) WHERE is_active = TRUE;

-- Table: archive_rank_definitions - User rank/level definitions
CREATE TABLE IF NOT EXISTS archive_rank_definitions (
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

CREATE INDEX IF NOT EXISTS idx_rank_defs_level ON archive_rank_definitions(level);
CREATE INDEX IF NOT EXISTS idx_rank_defs_xp ON archive_rank_definitions(min_xp);

-- Table: archive_user_profiles - Extended user profile with XP and stats
CREATE TABLE IF NOT EXISTS archive_user_profiles (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL UNIQUE,                         -- Reference to base_users
    total_xp INTEGER NOT NULL DEFAULT 0,                    -- Total experience points
    current_rank_id BIGINT,                                 -- Current rank (references archive_rank_definitions)
    current_streak_days INTEGER NOT NULL DEFAULT 0,         -- Current learning streak
    longest_streak_days INTEGER NOT NULL DEFAULT 0,         -- Longest streak ever
    last_activity_date DATE,                                -- Last day user was active
    total_study_minutes INTEGER NOT NULL DEFAULT 0,         -- Total minutes studied
    total_words_mastered INTEGER NOT NULL DEFAULT 0,        -- Total words mastered
    total_stages INTEGER NOT NULL DEFAULT 0,         -- Total stages completed
    total_sessions INTEGER NOT NULL DEFAULT 0,              -- Total learning sessions
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),           -- When user started learning
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_archive_user_profiles_user ON archive_user_profiles(user_id);
CREATE INDEX IF NOT EXISTS idx_archive_user_profiles_xp ON archive_user_profiles(total_xp DESC);
CREATE INDEX IF NOT EXISTS idx_archive_user_profiles_streak ON archive_user_profiles(current_streak_days DESC);

-- Table: archive_user_achievements - Achievements earned by users (enhanced version)
CREATE TABLE IF NOT EXISTS archive_user_achievements (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    achievement_id BIGINT NOT NULL,                         -- References archive_achievement_definitions
    progress INTEGER NOT NULL DEFAULT 0,                    -- Current progress towards achievement
    is_completed BOOLEAN NOT NULL DEFAULT FALSE,
    completed_at TIMESTAMPTZ,                               -- When achievement was earned
    notified_at TIMESTAMPTZ,                                -- When user was notified
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, achievement_id)
);

CREATE INDEX IF NOT EXISTS idx_archive_user_achievements_user ON archive_user_achievements(user_id, is_completed);
CREATE INDEX IF NOT EXISTS idx_archive_user_achievements_completed ON archive_user_achievements(completed_at DESC) WHERE is_completed = TRUE;

-- Table: archive_user_xp_history - Track XP gains
CREATE TABLE IF NOT EXISTS archive_user_xp_history (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    xp_amount INTEGER NOT NULL,
    source_type TEXT NOT NULL CHECK(source_type IN ('achievement', 'session', 'daily_bonus', 'streak', 'review', 'special')),
    source_id BIGINT,                                       -- Reference to the source (achievement_id, session_id, etc.)
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_archive_user_xp_history_user ON archive_user_xp_history(user_id, created_at DESC);

-- ============================================================================
-- SEED DATA: Achievement Definitions
-- ============================================================================

INSERT INTO archive_achievement_definitions (code, name_en, name_zh, description_en, description_zh, icon, category, rarity, xp_reward, requirement_type, requirement_value, requirement_field, sort_order) VALUES
-- Milestone: Getting Started
('first_conversation', 'First Chat', '初次对话', 'Complete your first conversation', '完成你的第一次对话', 'message-circle', 'milestone', 'common', 50, 'count', 1, 'total_stages', 1),

-- Milestone: Major Progress
('conversations_100', 'Chat Master', '对话大师', 'Complete 100 conversations', '完成100次对话', 'crown', 'milestone', 'rare', 500, 'count', 100, 'total_stages', 2),

-- Vocabulary: Foundation
('vocab_100', 'Vocabulary Builder', '词汇达人', 'Master 100 words', '掌握100个单词', 'book-marked', 'learning', 'uncommon', 200, 'count', 100, 'total_words_mastered', 3),

-- Vocabulary: Mastery
('vocab_1000', 'Lexicon Legend', '词汇大师', 'Master 1000 words', '掌握1000个单词', 'trophy', 'learning', 'epic', 1000, 'count', 1000, 'total_words_mastered', 4),

-- Streak: Weekly Commitment
('streak_7', 'Weekly Warrior', '周学习战士', '7-day learning streak', '连续学习7天', 'flame', 'streak', 'uncommon', 150, 'streak', 7, 'current_streak_days', 5),

-- Streak: Monthly Dedication
('streak_30', 'Monthly Master', '月度大师', '30-day learning streak', '连续学习30天', 'flame', 'streak', 'rare', 500, 'streak', 30, 'current_streak_days', 6),

-- Time Investment
('time_600', 'Time Investor', '时间达人', 'Study for 10 hours total', '累计学习10小时', 'clock', 'mastery', 'uncommon', 200, 'time', 600, 'total_study_minutes', 7),

-- Special Challenge
('perfect_score', 'Perfectionist', '完美主义者', 'Get 100% on a reading practice', '跟读练习获得满分', 'target', 'special', 'rare', 200, 'score', 100, NULL, 8)
ON CONFLICT (code) DO NOTHING;

-- ============================================================================
-- SEED DATA: Rank Definitions
-- ============================================================================

INSERT INTO archive_rank_definitions (code, name_en, name_zh, description_en, description_zh, icon, color, min_xp, level) VALUES
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
