ALTER TABLE base_users ADD COLUMN token_version BIGINT NOT NULL DEFAULT 0;
ALTER TABLE auth_codes ADD COLUMN token_version BIGINT NOT NULL DEFAULT 0;

-- Keep the most recent password if historical writers created duplicates.
DELETE FROM base_passwords p USING base_passwords newer
WHERE p.user_id = newer.user_id AND p.id < newer.id;
CREATE UNIQUE INDEX base_passwords_user_unique ON base_passwords(user_id);

UPDATE user_subscriptions SET status = 'expired', auto_renew = false,
    next_billing_at = NULL, updated_at = NOW()
WHERE status IN ('active', 'trial') AND expires_at <= NOW();
WITH ranked AS (
    SELECT id, ROW_NUMBER() OVER (
        PARTITION BY user_id ORDER BY created_at DESC, id DESC
    ) AS n FROM user_subscriptions WHERE status IN ('active', 'trial')
)
UPDATE user_subscriptions s SET status = 'cancelled', auto_renew = false,
    cancelled_at = NOW(), cancel_reason = 'duplicate subscription cleanup',
    next_billing_at = NULL, updated_at = NOW()
FROM ranked r WHERE s.id = r.id AND r.n > 1;
CREATE UNIQUE INDEX user_subscriptions_one_current ON user_subscriptions(user_id)
WHERE status IN ('active', 'trial');

-- Trial eligibility is account-wide, independent of plan changes/cancellation.
CREATE TABLE subscription_trial_claims (
    user_id BIGINT PRIMARY KEY REFERENCES base_users(id) ON DELETE CASCADE,
    claimed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
INSERT INTO subscription_trial_claims(user_id)
SELECT DISTINCT user_id FROM user_subscriptions WHERE trial_ends_at IS NOT NULL;

-- Batch id makes commit-ack loss safe to retry in the running process.
CREATE TABLE study_time_flush_batches (
    id UUID PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
