-- Remove saved credentials for retired model providers. Usage history is retained.
DELETE FROM user_ai_providers WHERE lower(provider_type) IN ('doubao', 'volcengine', 'volcano', 'ark');
