-- Table: learn_levels - Track user skill levels in various areas
CREATE TABLE IF NOT EXISTS learn_levels (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    skill_type TEXT NOT NULL CHECK(skill_type IN ('speaking', 'pronunciation', 'spelling', 'listening', 'reading', 'vocabulary', 'grammar')),
    level INTEGER NOT NULL DEFAULT 5 CHECK(level BETWEEN 1 AND 10),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, skill_type)
);

CREATE INDEX IF NOT EXISTS idx_learn_levels_user ON learn_levels(user_id);
