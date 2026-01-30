-- Payment System Migration Down
-- Drop tables in reverse order of creation

DROP TABLE IF EXISTS payment_refunds;
DROP TABLE IF EXISTS payment_orders;
DROP TABLE IF EXISTS user_subscriptions;
DROP TABLE IF EXISTS subscription_plans;
DROP TABLE IF EXISTS wallet_transactions;
DROP TABLE IF EXISTS wallet_accounts;
