-- Admin System Migration
-- Simple admin table - just stores which users are admins

CREATE TABLE IF NOT EXISTS base_admins (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL UNIQUE REFERENCES base_users(id) ON DELETE CASCADE,
    granted_by BIGINT REFERENCES base_users(id),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    notes TEXT
);

CREATE INDEX idx_base_admins_user ON base_admins(user_id);

COMMENT ON TABLE base_admins IS 'Stores which users have admin privileges';
COMMENT ON COLUMN base_admins.user_id IS 'The user who has admin privileges';
COMMENT ON COLUMN base_admins.granted_by IS 'The admin who granted the privileges';
COMMENT ON COLUMN base_admins.notes IS 'Optional notes about why admin was granted';
