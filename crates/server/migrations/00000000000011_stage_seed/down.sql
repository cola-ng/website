-- Remove seed data

-- Remove reading sentences first (references subjects)
DELETE FROM asset_read_sentences WHERE subject_id IN (SELECT id FROM asset_read_subjects WHERE title_en IN ('Daily Chat', 'Business English', 'Advanced Expressions'));

-- Remove reading subjects
DELETE FROM asset_read_subjects WHERE title_en IN ('Daily Chat', 'Business English', 'Advanced Expressions');

-- Remove script turns first (references scripts)
DELETE FROM asset_script_turns WHERE script_id IN (SELECT id FROM asset_scripts WHERE title_en IN ('Restaurant Ordering Full Dialogue', 'Hotel Check-in Full Dialogue', 'Airport Travel Full Dialogue'));

-- Remove scripts
DELETE FROM asset_scripts WHERE title_en IN ('Restaurant Ordering Full Dialogue', 'Hotel Check-in Full Dialogue', 'Airport Travel Full Dialogue');

-- Remove additional stages
DELETE FROM asset_stages WHERE name_en IN ('Grocery Shopping', 'Banking', 'Phone Booking', 'Taking a Taxi', 'Package Delivery', 'Movie Tickets');

-- Remove additional contexts
DELETE FROM asset_contexts WHERE name_en IN ('Grocery Shopping', 'Banking', 'Phone Booking', 'Taking a Taxi', 'Package Delivery', 'Movie Tickets');
