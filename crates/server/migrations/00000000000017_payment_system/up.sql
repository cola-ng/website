-- Payment System Migration
-- Wallet, subscriptions, and payment orders

-- ============================================================================
-- Wallet System
-- ============================================================================

-- User wallet accounts
CREATE TABLE IF NOT EXISTS wallet_accounts (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL UNIQUE REFERENCES base_users(id) ON DELETE CASCADE,
    balance BIGINT NOT NULL DEFAULT 0,  -- Credits (100 credits = 1 CNY)
    frozen_balance BIGINT NOT NULL DEFAULT 0,  -- Frozen during pending transactions
    total_deposited BIGINT NOT NULL DEFAULT 0,  -- Lifetime total deposited
    total_spent BIGINT NOT NULL DEFAULT 0,  -- Lifetime total spent
    currency VARCHAR(3) NOT NULL DEFAULT 'CNY',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_wallet_accounts_user ON wallet_accounts(user_id);

COMMENT ON TABLE wallet_accounts IS 'User wallet accounts storing credit balance';
COMMENT ON COLUMN wallet_accounts.balance IS 'Available balance in credits (100 credits = 1 CNY)';
COMMENT ON COLUMN wallet_accounts.frozen_balance IS 'Balance frozen during pending transactions';

-- Wallet transactions
CREATE TABLE IF NOT EXISTS wallet_transactions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES base_users(id) ON DELETE CASCADE,
    wallet_id BIGINT NOT NULL REFERENCES wallet_accounts(id) ON DELETE CASCADE,
    transaction_type VARCHAR(32) NOT NULL,  -- deposit, purchase, refund, bonus, adjustment
    amount BIGINT NOT NULL,  -- Positive for credit, negative for debit
    balance_before BIGINT NOT NULL,
    balance_after BIGINT NOT NULL,
    reference_type VARCHAR(32),  -- payment_order, subscription, admin, etc.
    reference_id BIGINT,  -- ID of related record
    description TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_wallet_transactions_user ON wallet_transactions(user_id);
CREATE INDEX idx_wallet_transactions_wallet ON wallet_transactions(wallet_id);
CREATE INDEX idx_wallet_transactions_type ON wallet_transactions(transaction_type);
CREATE INDEX idx_wallet_transactions_ref ON wallet_transactions(reference_type, reference_id);
CREATE INDEX idx_wallet_transactions_created ON wallet_transactions(created_at DESC);

COMMENT ON TABLE wallet_transactions IS 'Wallet transaction history';
COMMENT ON COLUMN wallet_transactions.transaction_type IS 'deposit, purchase, refund, bonus, adjustment';

-- ============================================================================
-- Subscription System
-- ============================================================================

-- Subscription plans
CREATE TABLE IF NOT EXISTS subscription_plans (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(32) NOT NULL UNIQUE,  -- free, pro_monthly, pro_yearly
    name_en VARCHAR(100) NOT NULL,
    name_zh VARCHAR(100) NOT NULL,
    description_en TEXT,
    description_zh TEXT,
    tier VARCHAR(16) NOT NULL,  -- free, pro
    billing_cycle VARCHAR(16),  -- monthly, yearly, null for free
    price_credits BIGINT NOT NULL DEFAULT 0,  -- Price in credits
    price_display VARCHAR(32),  -- Display price like "¥99/月"
    features JSONB,  -- JSON array of feature descriptions
    limits JSONB,  -- JSON object of usage limits
    trial_days INT DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    sort_order INT DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_subscription_plans_code ON subscription_plans(code);
CREATE INDEX idx_subscription_plans_tier ON subscription_plans(tier);
CREATE INDEX idx_subscription_plans_active ON subscription_plans(is_active);

COMMENT ON TABLE subscription_plans IS 'Available subscription plans';
COMMENT ON COLUMN subscription_plans.tier IS 'Subscription tier: free, pro';
COMMENT ON COLUMN subscription_plans.billing_cycle IS 'monthly, yearly, or null for free tier';

-- User subscriptions
CREATE TABLE IF NOT EXISTS user_subscriptions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES base_users(id) ON DELETE CASCADE,
    plan_id BIGINT NOT NULL REFERENCES subscription_plans(id),
    status VARCHAR(16) NOT NULL DEFAULT 'active',  -- active, cancelled, expired, trial
    started_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,
    cancel_reason TEXT,
    auto_renew BOOLEAN NOT NULL DEFAULT true,
    trial_ends_at TIMESTAMPTZ,
    last_payment_id BIGINT,  -- Reference to payment_orders
    next_billing_at TIMESTAMPTZ,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_user_subscriptions_user ON user_subscriptions(user_id);
CREATE INDEX idx_user_subscriptions_plan ON user_subscriptions(plan_id);
CREATE INDEX idx_user_subscriptions_status ON user_subscriptions(status);
CREATE INDEX idx_user_subscriptions_expires ON user_subscriptions(expires_at);

COMMENT ON TABLE user_subscriptions IS 'User subscription records';
COMMENT ON COLUMN user_subscriptions.status IS 'active, cancelled, expired, trial';

-- ============================================================================
-- Payment System
-- ============================================================================

-- Payment orders
CREATE TABLE IF NOT EXISTS payment_orders (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES base_users(id) ON DELETE CASCADE,
    order_no VARCHAR(64) NOT NULL UNIQUE,  -- Our order number
    order_type VARCHAR(32) NOT NULL,  -- deposit, subscription, one_time
    amount_credits BIGINT NOT NULL,  -- Amount in credits
    amount_cents BIGINT NOT NULL,  -- Amount in cents (for payment provider)
    currency VARCHAR(3) NOT NULL DEFAULT 'CNY',
    provider VARCHAR(32) NOT NULL,  -- alipay, wechat, stripe
    provider_order_id VARCHAR(128),  -- Provider's order ID
    provider_data JSONB,  -- Provider-specific data
    status VARCHAR(16) NOT NULL DEFAULT 'pending',  -- pending, paid, failed, cancelled, refunded
    paid_at TIMESTAMPTZ,
    failed_at TIMESTAMPTZ,
    failure_reason TEXT,
    reference_type VARCHAR(32),  -- subscription, etc.
    reference_id BIGINT,
    client_ip VARCHAR(45),
    user_agent TEXT,
    expires_at TIMESTAMPTZ,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_payment_orders_user ON payment_orders(user_id);
CREATE INDEX idx_payment_orders_no ON payment_orders(order_no);
CREATE INDEX idx_payment_orders_provider ON payment_orders(provider, provider_order_id);
CREATE INDEX idx_payment_orders_status ON payment_orders(status);
CREATE INDEX idx_payment_orders_created ON payment_orders(created_at DESC);

COMMENT ON TABLE payment_orders IS 'Payment order records';
COMMENT ON COLUMN payment_orders.provider IS 'Payment provider: alipay, wechat, stripe';
COMMENT ON COLUMN payment_orders.status IS 'pending, paid, failed, cancelled, refunded';

-- Payment refunds
CREATE TABLE IF NOT EXISTS payment_refunds (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES base_users(id) ON DELETE CASCADE,
    order_id BIGINT NOT NULL REFERENCES payment_orders(id),
    refund_no VARCHAR(64) NOT NULL UNIQUE,  -- Our refund number
    provider_refund_id VARCHAR(128),  -- Provider's refund ID
    amount_cents BIGINT NOT NULL,  -- Refund amount in cents
    amount_credits BIGINT NOT NULL,  -- Refund amount in credits
    reason TEXT,
    status VARCHAR(16) NOT NULL DEFAULT 'pending',  -- pending, completed, failed
    completed_at TIMESTAMPTZ,
    failed_at TIMESTAMPTZ,
    failure_reason TEXT,
    processed_by BIGINT REFERENCES base_users(id),  -- Admin who processed
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_payment_refunds_user ON payment_refunds(user_id);
CREATE INDEX idx_payment_refunds_order ON payment_refunds(order_id);
CREATE INDEX idx_payment_refunds_no ON payment_refunds(refund_no);
CREATE INDEX idx_payment_refunds_status ON payment_refunds(status);

COMMENT ON TABLE payment_refunds IS 'Payment refund records';

-- ============================================================================
-- Seed Data: Default Subscription Plans
-- ============================================================================

INSERT INTO subscription_plans (code, name_en, name_zh, description_en, description_zh, tier, billing_cycle, price_credits, price_display, features, limits, trial_days, sort_order) VALUES
('free', 'Free', '免费版', 'Basic features with usage limits', '基础功能，有使用限制', 'free', NULL, 0, '免费',
 '["每日5次AI对话", "基础词典查询", "基础复习功能"]'::jsonb,
 '{"daily_chats": 5, "daily_translations": 10, "vocab_limit": 100}'::jsonb,
 0, 0),
('pro_monthly', 'Pro Monthly', '专业版（月付）', 'Unlimited access with all features', '无限制使用所有功能', 'pro', 'monthly', 9900, '¥99/月',
 '["无限AI对话", "高级语音评测", "完整词典功能", "无限单词收藏", "优先客服支持"]'::jsonb,
 '{"daily_chats": -1, "daily_translations": -1, "vocab_limit": -1}'::jsonb,
 7, 1),
('pro_yearly', 'Pro Yearly', '专业版（年付）', 'Unlimited access with all features, save 20%', '无限制使用所有功能，省20%', 'pro', 'yearly', 95000, '¥950/年',
 '["无限AI对话", "高级语音评测", "完整词典功能", "无限单词收藏", "优先客服支持", "专属学习报告"]'::jsonb,
 '{"daily_chats": -1, "daily_translations": -1, "vocab_limit": -1}'::jsonb,
 14, 2)
ON CONFLICT (code) DO UPDATE SET
    name_en = EXCLUDED.name_en,
    name_zh = EXCLUDED.name_zh,
    description_en = EXCLUDED.description_en,
    description_zh = EXCLUDED.description_zh,
    price_credits = EXCLUDED.price_credits,
    price_display = EXCLUDED.price_display,
    features = EXCLUDED.features,
    limits = EXCLUDED.limits,
    trial_days = EXCLUDED.trial_days,
    sort_order = EXCLUDED.sort_order,
    updated_at = CURRENT_TIMESTAMP;
