CREATE TABLE amigo_device_ai (
    device_id UUID PRIMARY KEY REFERENCES amigo_devices(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES base_users(id) ON DELETE CASCADE,
    bindings JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
