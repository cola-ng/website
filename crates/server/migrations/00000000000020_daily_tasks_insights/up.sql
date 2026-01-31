-- Daily Tasks and AI Insights for homepage

-- Daily tasks generated for each user
CREATE TABLE learn_daily_tasks (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    task_date DATE NOT NULL,
    -- Task type: 'chat' (自由对话), 'stage' (场景练习), 'review' (复习易错点), 'read' (跟读训练)
    task_type TEXT NOT NULL CHECK (task_type IN ('chat', 'stage', 'review', 'read')),
    task_title TEXT NOT NULL,
    task_description TEXT,
    -- Target and progress (e.g., 3 minutes, 8 words)
    target_value INTEGER NOT NULL DEFAULT 1,
    current_value INTEGER NOT NULL DEFAULT 0,
    -- Status: 'pending', 'in_progress', 'completed'
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'completed')),
    -- Display order on the page
    display_order INTEGER NOT NULL DEFAULT 0,
    -- Optional reference to related content (stage_id, etc.)
    reference_id BIGINT,
    reference_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,

    UNIQUE(user_id, task_date, task_type, display_order)
);

CREATE INDEX idx_learn_daily_tasks_user_date ON learn_daily_tasks(user_id, task_date DESC);
CREATE INDEX idx_learn_daily_tasks_date ON learn_daily_tasks(task_date);

-- AI-generated insights based on user's learning data
CREATE TABLE learn_daily_insights (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    insight_date DATE NOT NULL,
    -- Insight type: 'progress' (进步), 'weakness' (薄弱点), 'suggestion' (建议), 'encouragement' (鼓励)
    insight_type TEXT NOT NULL CHECK (insight_type IN ('progress', 'weakness', 'suggestion', 'encouragement')),
    -- Icon for frontend display: 'bulb', 'warning', 'chart', 'star'
    icon_type TEXT NOT NULL DEFAULT 'bulb',
    -- Content in Chinese (primary)
    content_zh TEXT NOT NULL,
    -- Optional English content
    content_en TEXT,
    -- Optional action link
    action_text TEXT,
    action_link TEXT,
    -- Additional metadata (JSON) - e.g., specific stats, percentages
    metadata JSONB,
    -- Display order
    display_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(user_id, insight_date, display_order)
);

CREATE INDEX idx_learn_daily_insights_user_date ON learn_daily_insights(user_id, insight_date DESC);
CREATE INDEX idx_learn_daily_insights_date ON learn_daily_insights(insight_date);

-- Track when daily content was last generated for a user
CREATE TABLE learn_daily_generation_log (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    generation_date DATE NOT NULL,
    tasks_generated INTEGER NOT NULL DEFAULT 0,
    insights_generated INTEGER NOT NULL DEFAULT 0,
    generation_source TEXT, -- 'auto', 'manual', 'api'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(user_id, generation_date)
);

CREATE INDEX idx_learn_daily_gen_log_user_date ON learn_daily_generation_log(user_id, generation_date DESC);
