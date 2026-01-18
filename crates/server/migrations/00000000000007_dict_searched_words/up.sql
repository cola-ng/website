-- Dictionary search history table
CREATE TABLE IF NOT EXISTS dict_searched_words (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL DEFAULT 1,
    word_id BIGINT,
    word VARCHAR(255) NOT NULL,
    searched_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for efficient queries
CREATE INDEX idx_dict_searched_words_user ON dict_searched_words(user_id, searched_at DESC);
CREATE INDEX idx_dict_searched_words_word ON dict_searched_words(word);
