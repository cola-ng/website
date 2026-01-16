-- ============================================================================
-- ROLLBACK DICTIONARY SEED DATA
-- ============================================================================

-- Remove sample example sentences
DELETE FROM dict_word_examples
WHERE word_id IN (
    SELECT id FROM dict_words
    WHERE word IN ('abase', 'abate', 'abbreviate', 'abduct', 'abnormal',
                   'aboriginal', 'abrade', 'absorb', 'accelerate', 'access')
);

-- Remove sample etymology data
DELETE FROM dict_word_etymology
WHERE word_id IN (
    SELECT id FROM dict_words
    WHERE word IN ('abase', 'abate', 'abbreviate', 'abduct', 'abnormal',
                   'aboriginal', 'abrade', 'absorb', 'accelerate', 'access')
);

-- Remove sample definitions
DELETE FROM dict_word_definitions
WHERE word_id IN (
    SELECT id FROM dict_words
    WHERE word IN ('abase', 'abate', 'abbreviate', 'abduct', 'abnormal',
                   'aboriginal', 'abrade', 'absorb', 'accelerate', 'access')
);

-- Remove sample words
DELETE FROM dict_words
WHERE word IN ('abase', 'abate', 'abbreviate', 'abduct', 'abnormal',
               'aboriginal', 'abrade', 'absorb', 'accelerate', 'access');

-- Remove import batch record
DELETE FROM dict_import_batch
WHERE batch_name = 'dictionary_seed_v1';
