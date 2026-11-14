DROP TABLE study_time_flush_batches;
DROP TABLE subscription_trial_claims;
DROP INDEX user_subscriptions_one_current;
DROP INDEX base_passwords_user_unique;
ALTER TABLE base_users DROP COLUMN token_version;

ALTER TABLE auth_codes DROP COLUMN token_version;
