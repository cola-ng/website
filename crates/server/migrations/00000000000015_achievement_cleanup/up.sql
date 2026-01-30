-- Clean up old achievement definitions that are no longer used
-- Keep only the 8 core achievements

-- First, delete user_achievements for removed achievement definitions
DELETE FROM archive_user_achievements
WHERE achievement_id IN (
    SELECT id FROM archive_achievement_definitions
    WHERE code NOT IN (
        'first_conversation',
        'conversations_100',
        'vocab_100',
        'vocab_1000',
        'streak_7',
        'streak_30',
        'time_600',
        'perfect_score'
    )
);

-- Delete the removed achievement definitions
DELETE FROM archive_achievement_definitions
WHERE code NOT IN (
    'first_conversation',
    'conversations_100',
    'vocab_100',
    'vocab_1000',
    'streak_7',
    'streak_30',
    'time_600',
    'perfect_score'
);

-- Update XP rewards and descriptions for the remaining achievements
UPDATE archive_achievement_definitions SET
    xp_reward = 50,
    sort_order = 1
WHERE code = 'first_conversation';

UPDATE archive_achievement_definitions SET
    xp_reward = 500,
    sort_order = 2
WHERE code = 'conversations_100';

UPDATE archive_achievement_definitions SET
    name_zh = '词汇达人',
    xp_reward = 200,
    sort_order = 3
WHERE code = 'vocab_100';

UPDATE archive_achievement_definitions SET
    name_zh = '词汇大师',
    xp_reward = 1000,
    sort_order = 4
WHERE code = 'vocab_1000';

UPDATE archive_achievement_definitions SET
    xp_reward = 150,
    sort_order = 5
WHERE code = 'streak_7';

UPDATE archive_achievement_definitions SET
    xp_reward = 500,
    sort_order = 6
WHERE code = 'streak_30';

UPDATE archive_achievement_definitions SET
    name_zh = '时间达人',
    xp_reward = 200,
    sort_order = 7
WHERE code = 'time_600';

UPDATE archive_achievement_definitions SET
    xp_reward = 200,
    sort_order = 8
WHERE code = 'perfect_score';
