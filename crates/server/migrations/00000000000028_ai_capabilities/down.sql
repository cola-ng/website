DROP INDEX user_ai_providers_one_tts;
DROP INDEX user_ai_providers_one_asr;
ALTER TABLE user_ai_providers DROP COLUMN tts_active;
ALTER TABLE user_ai_providers DROP COLUMN asr_active;
