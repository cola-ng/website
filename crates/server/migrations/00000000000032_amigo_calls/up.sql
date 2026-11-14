CREATE TABLE amigo_calls (
 id UUID PRIMARY KEY, device_id UUID NOT NULL REFERENCES amigo_devices(id) ON DELETE CASCADE,
 user_id BIGINT NOT NULL REFERENCES base_users(id) ON DELETE CASCADE, turn_id TEXT NOT NULL,
 stage TEXT NOT NULL, provider TEXT NOT NULL, model TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('running','success','error','cancelled')),
 error TEXT, duration_ms BIGINT NOT NULL DEFAULT 0,
 started_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX amigo_calls_device_time ON amigo_calls(device_id,started_at DESC);
