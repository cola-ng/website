CREATE TABLE amigo_pairings (
    token_hash VARCHAR(64) PRIMARY KEY,
    code CHAR(6) NOT NULL UNIQUE CHECK (code ~ '^[0-9]{6}$'),
    board TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '5 minutes'),
    device_id UUID REFERENCES amigo_devices(id) ON DELETE CASCADE
);
CREATE INDEX amigo_pairings_expiry ON amigo_pairings(expires_at);
CREATE TABLE amigo_pairing_limits (
    key TEXT PRIMARY KEY,
    window_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    attempts INTEGER NOT NULL DEFAULT 1
);
