-- Remove indexes
DROP INDEX IF EXISTS idx_dict_entries_word_search;
DROP INDEX IF EXISTS idx_dict_entries_frequency;
DROP INDEX IF EXISTS idx_dict_entries_category;
DROP INDEX IF EXISTS idx_dict_entries_difficulty;
DROP INDEX IF EXISTS idx_dict_entries_word_lower;
DROP INDEX IF EXISTS idx_dict_entries_word;

-- Remove table
DROP TABLE IF EXISTS asset_dict_entries;
