-- Conversation sessions table
CREATE TABLE IF NOT EXISTS learn_chat_sessions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    title TEXT,
    system_prompt TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    message_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_learn_chat_sessions_user_id ON learn_chat_sessions(user_id);
CREATE INDEX idx_learn_chat_sessions_user_active ON learn_chat_sessions(user_id, is_active) WHERE is_active = TRUE;

-- Conversation messages table
CREATE TABLE IF NOT EXISTS learn_chat_messages (
    id BIGSERIAL PRIMARY KEY,
    session_id BIGINT NOT NULL REFERENCES learn_chat_sessions(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL,
    role TEXT NOT NULL, -- 'user' or 'assistant'
    content TEXT NOT NULL,
    audio_base64 TEXT, -- optional audio data
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_learn_chat_messages_session_id ON learn_chat_messages(session_id);
CREATE INDEX idx_learn_chat_messages_user_id ON learn_chat_messages(user_id);
CREATE INDEX idx_learn_chat_messages_session_created ON learn_chat_messages(session_id, created_at);
