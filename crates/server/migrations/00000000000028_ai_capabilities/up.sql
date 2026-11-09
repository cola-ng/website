ALTER TABLE user_ai_providers ADD COLUMN tts_active BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE user_ai_providers ADD COLUMN asr_active BOOLEAN NOT NULL DEFAULT false;
UPDATE user_ai_providers SET tts_active = is_active, asr_active = is_active WHERE provider_type = 'zhipu';
CREATE UNIQUE INDEX user_ai_providers_one_tts ON user_ai_providers(user_id) WHERE tts_active;
CREATE UNIQUE INDEX user_ai_providers_one_asr ON user_ai_providers(user_id) WHERE asr_active;
