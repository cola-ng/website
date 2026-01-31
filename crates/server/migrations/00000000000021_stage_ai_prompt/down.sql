-- Remove AI prompt field from asset_stages
ALTER TABLE asset_stages DROP COLUMN IF EXISTS ai_prompt;
