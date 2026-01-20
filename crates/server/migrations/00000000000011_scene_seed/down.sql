-- Remove seed data
DELETE FROM asset_phrases WHERE phrase_en IN ('reservation', 'check-in', 'check-out', 'Could you please...', 'I would like to...', 'Thank you for...');
DELETE FROM asset_read_sentences WHERE exercise_id IN (SELECT id FROM asset_read_exercises WHERE title_en IN ('Daily Conversations', 'Business English', 'Advanced Expressions'));
DELETE FROM asset_read_exercises WHERE title_en IN ('Daily Conversations', 'Business English', 'Advanced Expressions');
DELETE FROM asset_classic_clips WHERE source_id IN (SELECT id FROM asset_classic_sources WHERE title IN ('The Shawshank Redemption', 'Friends', 'Your Body Language May Shape Who You Are', 'Forrest Gump', 'The Office', 'How Great Leaders Inspire Action'));
DELETE FROM asset_classic_sources WHERE title IN ('The Shawshank Redemption', 'Friends', 'Your Body Language May Shape Who You Are', 'Forrest Gump', 'The Office', 'How Great Leaders Inspire Action');
DELETE FROM asset_dialogue_turns WHERE dialogue_id IN (SELECT id FROM asset_dialogues WHERE title_en IN ('餐厅点餐完整对话', '酒店入住完整对话', '机场出行完整对话'));
DELETE FROM asset_dialogues WHERE title_en IN ('餐厅点餐完整对话', '酒店入住完整对话', '机场出行完整对话');
DELETE FROM asset_scenes WHERE name_en IN ('Restaurant Ordering', 'Hotel Check-in', 'Airport Travel', 'Grocery Shopping', 'Job Interview', 'Doctor Visit', 'Banking', 'Phone Booking', 'Coffee Shop Order', 'Taking a Taxi', 'Package Delivery', 'Movie Tickets');
