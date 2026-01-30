-- AI Usage Tracking System Migration
-- Tracks individual AI API calls and aggregated quotas per billing period

-- Individual AI usage records for detailed tracking
CREATE TABLE IF NOT EXISTS ai_usage_records (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES base_users(id) ON DELETE CASCADE,

    -- Service type: 'chat', 'asr', 'tts', 'translation'
    service_type VARCHAR(32) NOT NULL,

    -- Provider info
    provider_type VARCHAR(32) NOT NULL,  -- 'zhipu', 'doubao'
    is_user_provider BOOLEAN NOT NULL DEFAULT false,  -- true if user's own provider

    -- Usage metrics (only relevant field populated per service_type)
    tokens_used INT,              -- For chat: total tokens
    prompt_tokens INT,            -- For chat: prompt tokens only
    completion_tokens INT,        -- For chat: completion tokens only
    duration_seconds FLOAT,       -- For ASR/TTS: duration in seconds
    characters_count INT,         -- For translation: character count

    -- Request context
    model_name VARCHAR(64),       -- Model used (e.g., 'glm-4-flash', 'doubao-1-5-pro')

    -- Status
    status VARCHAR(16) NOT NULL DEFAULT 'completed',  -- completed, failed
    error_message TEXT,

    -- Metadata for extensibility
    metadata JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for efficient querying
CREATE INDEX idx_ai_usage_records_user ON ai_usage_records(user_id);
CREATE INDEX idx_ai_usage_records_user_service ON ai_usage_records(user_id, service_type);
CREATE INDEX idx_ai_usage_records_user_created ON ai_usage_records(user_id, created_at DESC);
CREATE INDEX idx_ai_usage_records_created ON ai_usage_records(created_at DESC);

COMMENT ON TABLE ai_usage_records IS 'Detailed AI API usage records';
COMMENT ON COLUMN ai_usage_records.service_type IS 'chat, asr, tts, translation';
COMMENT ON COLUMN ai_usage_records.is_user_provider IS 'If true, does not count toward billing quota';

-- Aggregated usage quotas per billing period
CREATE TABLE IF NOT EXISTS ai_usage_quotas (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES base_users(id) ON DELETE CASCADE,

    -- Billing period
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,

    -- Usage counters (only system-provided usage counts toward quota)
    chat_tokens_used BIGINT NOT NULL DEFAULT 0,
    asr_seconds_used FLOAT NOT NULL DEFAULT 0,
    tts_seconds_used FLOAT NOT NULL DEFAULT 0,
    translation_chars_used BIGINT NOT NULL DEFAULT 0,

    -- Request counts
    chat_requests INT NOT NULL DEFAULT 0,
    asr_requests INT NOT NULL DEFAULT 0,
    tts_requests INT NOT NULL DEFAULT 0,
    translation_requests INT NOT NULL DEFAULT 0,

    -- Snapshot of limits at period start (from subscription plan)
    limits_snapshot JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Unique constraint per user per period
    UNIQUE(user_id, period_start)
);

CREATE INDEX idx_ai_usage_quotas_user ON ai_usage_quotas(user_id);
CREATE INDEX idx_ai_usage_quotas_period ON ai_usage_quotas(period_start, period_end);

COMMENT ON TABLE ai_usage_quotas IS 'Aggregated AI usage quotas per billing period';
