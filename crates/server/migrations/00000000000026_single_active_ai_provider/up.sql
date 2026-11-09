WITH ranked AS (
    SELECT id, row_number() OVER (PARTITION BY user_id ORDER BY updated_at DESC, id DESC) AS position
    FROM user_ai_providers WHERE is_active
)
UPDATE user_ai_providers SET is_active = false
WHERE id IN (SELECT id FROM ranked WHERE position > 1);
CREATE UNIQUE INDEX user_ai_providers_one_active ON user_ai_providers (user_id) WHERE is_active;
