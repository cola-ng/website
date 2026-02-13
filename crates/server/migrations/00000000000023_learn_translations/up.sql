-- 用户翻译记录表 (用于收藏翻译结果)
CREATE TABLE learn_translations (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES base_users(id) ON DELETE CASCADE,
    source_text TEXT NOT NULL,
    translated_text TEXT NOT NULL,
    source_lang TEXT NOT NULL,
    target_lang TEXT NOT NULL,
    tone TEXT NOT NULL DEFAULT 'original',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- 唯一约束: 同一用户、同一原文、同一目标语言和语气不重复
    CONSTRAINT learn_translations_unique UNIQUE (user_id, source_text, target_lang, tone)
);

-- 索引
CREATE INDEX idx_learn_translations_user_id ON learn_translations(user_id);
