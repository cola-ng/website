-- User AI Provider Settings
-- Allows users to configure their own AI providers (Zhipu, Doubao)

CREATE TABLE IF NOT EXISTS user_ai_providers (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES base_users(id) ON DELETE CASCADE,
    provider_type TEXT NOT NULL,  -- 'zhipu', 'doubao'
    is_active BOOLEAN NOT NULL DEFAULT false,
    settings JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_tested_at TIMESTAMPTZ,
    test_status TEXT,  -- 'success', 'failed', null
    test_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Each user can only have one config per provider type
    UNIQUE(user_id, provider_type)
);

-- Only one active provider per user (using partial unique index)
CREATE UNIQUE INDEX idx_user_ai_providers_active_unique
    ON user_ai_providers(user_id) WHERE is_active = true;

-- Index for querying user's providers
CREATE INDEX idx_user_ai_providers_user ON user_ai_providers(user_id);
