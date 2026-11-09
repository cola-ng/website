CREATE TABLE sentence_audio_whitelist (
    provider_type TEXT PRIMARY KEY,
    enabled BOOLEAN NOT NULL DEFAULT false,
    updated_by BIGINT REFERENCES base_users(id) ON DELETE SET NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO sentence_audio_whitelist(provider_type,enabled) VALUES ('openai',true),('zhipu',true),('minimax',true);
CREATE TABLE sentence_audio_candidates (
    id BIGSERIAL PRIMARY KEY,
    source_kind TEXT NOT NULL CHECK(source_kind IN ('dict','read','script')),
    source_id BIGINT NOT NULL,
    text_hash TEXT NOT NULL,
    sentence TEXT NOT NULL,
    user_id BIGINT REFERENCES base_users(id) ON DELETE SET NULL,
    provider_type TEXT NOT NULL,
    model TEXT NOT NULL,
    voice TEXT NOT NULL,
    audio_path TEXT,
    status TEXT NOT NULL CHECK(status IN ('generating','pending','approved','rejected','superseded','failed')),
    review_note TEXT,
    reviewed_by BIGINT REFERENCES base_users(id) ON DELETE SET NULL,
    reviewed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX sentence_audio_one_generation ON sentence_audio_candidates(text_hash) WHERE status='generating';
CREATE UNIQUE INDEX sentence_audio_one_approved ON sentence_audio_candidates(text_hash) WHERE status='approved';
CREATE INDEX sentence_audio_review_queue ON sentence_audio_candidates(status,id DESC);
CREATE INDEX sentence_audio_owner ON sentence_audio_candidates(user_id,created_at DESC);
