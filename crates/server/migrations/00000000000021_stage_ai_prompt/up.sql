-- Add AI prompt field to asset_stages for chat scenario configuration
ALTER TABLE asset_stages ADD COLUMN ai_prompt TEXT;

-- Add comment
COMMENT ON COLUMN asset_stages.ai_prompt IS 'System prompt for AI when user chats in this scenario';
