-- 每日学习时间统计表 (按页面类型分类)
CREATE TABLE learn_study_time_daily (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES base_users(id) ON DELETE CASCADE,
    stat_date DATE NOT NULL,
    -- 各类型学习时长 (单位: 秒)
    chat_seconds INTEGER NOT NULL DEFAULT 0,
    review_seconds INTEGER NOT NULL DEFAULT 0,
    stage_seconds INTEGER NOT NULL DEFAULT 0,
    read_seconds INTEGER NOT NULL DEFAULT 0,
    dictionary_seconds INTEGER NOT NULL DEFAULT 0,
    translate_seconds INTEGER NOT NULL DEFAULT 0,
    -- 总学习时长 (冗余字段, 方便查询)
    total_seconds INTEGER NOT NULL DEFAULT 0,
    -- 心跳统计 (用于调试和分析)
    heartbeat_count INTEGER NOT NULL DEFAULT 0,
    -- 时间戳
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- 唯一约束: 每用户每天一条记录
    CONSTRAINT learn_study_time_daily_user_date_unique UNIQUE (user_id, stat_date)
);

-- 索引
CREATE INDEX idx_learn_study_time_daily_user_id ON learn_study_time_daily(user_id);
CREATE INDEX idx_learn_study_time_daily_stat_date ON learn_study_time_daily(stat_date);
CREATE INDEX idx_learn_study_time_daily_user_date ON learn_study_time_daily(user_id, stat_date DESC);
