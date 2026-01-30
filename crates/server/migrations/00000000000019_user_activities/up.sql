-- User Activity Tracking System Migration
-- Records important user actions for admin visibility

CREATE TABLE IF NOT EXISTS user_activities (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES base_users(id) ON DELETE CASCADE,

    -- Activity type: 'login', 'payment', 'subscription', 'password_change', 'profile_update', etc.
    activity_type VARCHAR(32) NOT NULL,

    -- Human-readable description
    description TEXT,

    -- Additional metadata (e.g., payment amount, subscription plan, IP address)
    metadata JSONB,

    -- IP address for security tracking
    ip_address VARCHAR(45),

    -- User agent for device tracking
    user_agent TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for efficient querying
CREATE INDEX idx_user_activities_user ON user_activities(user_id);
CREATE INDEX idx_user_activities_user_type ON user_activities(user_id, activity_type);
CREATE INDEX idx_user_activities_user_created ON user_activities(user_id, created_at DESC);
CREATE INDEX idx_user_activities_created ON user_activities(created_at DESC);
CREATE INDEX idx_user_activities_type ON user_activities(activity_type);

COMMENT ON TABLE user_activities IS 'Records important user actions for admin tracking';
COMMENT ON COLUMN user_activities.activity_type IS 'login, logout, payment, subscription, password_change, profile_update, admin_grant, admin_revoke';
