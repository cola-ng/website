-- Drop user progress tables
DROP TABLE IF EXISTS user_read_progress;
DROP TABLE IF EXISTS user_clip_progress;
DROP TABLE IF EXISTS user_scene_progress;

-- Remove added columns
ALTER TABLE asset_scenes DROP COLUMN IF EXISTS duration_minutes;
ALTER TABLE asset_scenes DROP COLUMN IF EXISTS is_featured;
ALTER TABLE asset_classic_sources DROP COLUMN IF EXISTS icon_emoji;
ALTER TABLE asset_classic_sources DROP COLUMN IF EXISTS is_featured;
ALTER TABLE asset_classic_sources DROP COLUMN IF EXISTS display_order;
ALTER TABLE asset_read_sentences DROP COLUMN IF EXISTS tips;
