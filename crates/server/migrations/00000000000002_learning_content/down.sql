-- Drop user-specific tables first (due to foreign key dependencies)
DROP TABLE IF EXISTS assistant_suggestions;
DROP TABLE IF EXISTS assistant_conversations;
DROP TABLE IF EXISTS user_vocabulary;
DROP TABLE IF EXISTS daily_stats;
DROP TABLE IF EXISTS user_achievements;
DROP TABLE IF EXISTS reading_practice_attempts;
DROP TABLE IF EXISTS word_practice_log;
DROP TABLE IF EXISTS conversation_annotations;
DROP TABLE IF EXISTS conversations;
DROP TABLE IF EXISTS learning_sessions;
DROP TABLE IF EXISTS issue_words;

-- Drop shared content tables
DROP TABLE IF EXISTS key_phrases;
DROP TABLE IF EXISTS reading_sentences;
DROP TABLE IF EXISTS reading_exercises;
DROP TABLE IF EXISTS classic_dialogue_clips;
DROP TABLE IF EXISTS classic_dialogue_sources;
DROP TABLE IF EXISTS dialogue_turns;
DROP TABLE IF EXISTS scene_dialogues;
DROP TABLE IF EXISTS scenarios;
