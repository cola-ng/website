-- ============================================================================
-- SEED DATA FOR STAGE AND READING
-- ============================================================================

-- Insert additional stages into asset_contexts
INSERT INTO asset_contexts (name_en, name_zh, description_en, description_zh, icon_emoji, difficulty, display_order, is_active) VALUES
('Grocery Shopping', '超市购物', 'English communication skills for grocery shopping', '超市购物时的英语交流技巧', '🛒', 2, 11, true),
('Banking', '银行业务', 'English expressions for banking services', '银行业务办理的英语表达', '🏦', 4, 12, true),
('Phone Booking', '电话预约', 'English conversation skills for phone reservations', '电话预约的英语对话技巧', '📞', 4, 13, true),
('Taking a Taxi', '打车出行', 'English communication when taking a taxi', '打车时的英语交流', '🚕', 2, 14, true),
('Package Delivery', '快递收发', 'English for sending and receiving packages', '收发快递时的英语表达', '📦', 2, 15, true),
('Movie Tickets', '电影购票', 'English dialogue for buying movie tickets', '电影院购票的英语对话', '🎬', 2, 16, true)
ON CONFLICT (name_en, name_zh) DO UPDATE SET
    description_en = EXCLUDED.description_en,
    description_zh = EXCLUDED.description_zh,
    icon_emoji = EXCLUDED.icon_emoji,
    difficulty = EXCLUDED.difficulty,
    display_order = EXCLUDED.display_order;

-- Insert additional stages for script linking
INSERT INTO asset_stages (name_en, name_zh, description_en, description_zh, icon_emoji, difficulty, display_order, is_active) VALUES
('Grocery Shopping', '超市购物', 'English communication skills for grocery shopping', '超市购物时的英语交流技巧', '🛒', 2, 11, true),
('Banking', '银行业务', 'English expressions for banking services', '银行业务办理的英语表达', '🏦', 4, 12, true),
('Phone Booking', '电话预约', 'English conversation skills for phone reservations', '电话预约的英语对话技巧', '📞', 4, 13, true),
('Taking a Taxi', '打车出行', 'English communication when taking a taxi', '打车时的英语交流', '🚕', 2, 14, true),
('Package Delivery', '快递收发', 'English for sending and receiving packages', '收发快递时的英语表达', '📦', 2, 15, true),
('Movie Tickets', '电影购票', 'English dialogue for buying movie tickets', '电影院购票的英语对话', '🎬', 2, 16, true)
ON CONFLICT (name_en, name_zh) DO UPDATE SET
    description_en = EXCLUDED.description_en,
    description_zh = EXCLUDED.description_zh,
    icon_emoji = EXCLUDED.icon_emoji,
    difficulty = EXCLUDED.difficulty,
    display_order = EXCLUDED.display_order;

-- Insert dialogues for asset_scripts (need stage_id from asset_stages)
INSERT INTO asset_scripts (stage_id, title_en, title_zh, description_en, description_zh, total_turns, estimated_duration_seconds, difficulty)
SELECT s.id, 'Restaurant Ordering Full Dialogue', '餐厅点餐完整对话', 'Complete dialogue practice for restaurant ordering', '餐厅点餐完整对话练习', 12, 300, 3
FROM asset_stages s WHERE s.name_en = 'Restaurant Ordering'
ON CONFLICT DO NOTHING;

INSERT INTO asset_scripts (stage_id, title_en, title_zh, description_en, description_zh, total_turns, estimated_duration_seconds, difficulty)
SELECT s.id, 'Hotel Check-in Full Dialogue', '酒店入住完整对话', 'Complete dialogue practice for hotel check-in', '酒店入住完整对话练习', 13, 480, 3
FROM asset_stages s WHERE s.name_en = 'Hotel Reservation'
ON CONFLICT DO NOTHING;

INSERT INTO asset_scripts (stage_id, title_en, title_zh, description_en, description_zh, total_turns, estimated_duration_seconds, difficulty)
SELECT s.id, 'Airport Travel Full Dialogue', '机场出行完整对话', 'Complete dialogue practice for airport travel', '机场出行完整对话练习', 12, 600, 4
FROM asset_stages s WHERE s.name_en = 'Airport Check-in'
ON CONFLICT DO NOTHING;

-- Insert dialogue turns for restaurant ordering
INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, notes)
SELECT d.id, 1, 'assistant', 'Waiter', 'Good evening! Welcome to The Garden Restaurant. Do you have a reservation?', '晚上好！欢迎来到花园餐厅。请问您有预订吗？', NULL
FROM asset_scripts d JOIN asset_stages s ON d.stage_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = 'Restaurant Ordering Full Dialogue'
ON CONFLICT (script_id, turn_number) DO NOTHING;

INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, notes)
SELECT d.id, 2, 'user', 'Customer', 'Yes, I have a reservation under the name Smith for two people.', '是的，我有预订，史密斯，两位。', 'Tip: Say your name and party size'
FROM asset_scripts d JOIN asset_stages s ON d.stage_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = 'Restaurant Ordering Full Dialogue'
ON CONFLICT (script_id, turn_number) DO NOTHING;

INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, notes)
SELECT d.id, 3, 'assistant', 'Waiter', 'Perfect, Mr. Smith. Please follow me. Here is your table.', '好的，史密斯先生。请跟我来。这是您的座位。', NULL
FROM asset_scripts d JOIN asset_stages s ON d.stage_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = 'Restaurant Ordering Full Dialogue'
ON CONFLICT (script_id, turn_number) DO NOTHING;

INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, notes)
SELECT d.id, 4, 'assistant', 'Waiter', 'Here are your menus. Can I get you something to drink while you decide?', '这是菜单。您在看菜单时，要先喝点什么吗？', NULL
FROM asset_scripts d JOIN asset_stages s ON d.stage_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = 'Restaurant Ordering Full Dialogue'
ON CONFLICT (script_id, turn_number) DO NOTHING;

INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, notes)
SELECT d.id, 5, 'user', 'Customer', 'Could I have a glass of water and a cup of coffee, please?', '请给我一杯水和一杯咖啡。', 'Tip: Order some drinks'
FROM asset_scripts d JOIN asset_stages s ON d.stage_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = 'Restaurant Ordering Full Dialogue'
ON CONFLICT (script_id, turn_number) DO NOTHING;

INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, notes)
SELECT d.id, 6, 'assistant', 'Waiter', 'Certainly. Still or sparkling water?', '好的。是矿泉水还是苏打水？', NULL
FROM asset_scripts d JOIN asset_stages s ON d.stage_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = 'Restaurant Ordering Full Dialogue'
ON CONFLICT (script_id, turn_number) DO NOTHING;

INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, notes)
SELECT d.id, 7, 'user', 'Customer', 'Still water, please.', '矿泉水，谢谢。', 'Tip: Choose water type'
FROM asset_scripts d JOIN asset_stages s ON d.stage_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = 'Restaurant Ordering Full Dialogue'
ON CONFLICT (script_id, turn_number) DO NOTHING;

INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, notes)
SELECT d.id, 8, 'assistant', 'Waiter', 'Are you ready to order, or do you need a few more minutes?', '您准备好点餐了吗，还是需要再看一下？', NULL
FROM asset_scripts d JOIN asset_stages s ON d.stage_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = 'Restaurant Ordering Full Dialogue'
ON CONFLICT (script_id, turn_number) DO NOTHING;

INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, notes)
SELECT d.id, 9, 'user', 'Customer', 'I would like the grilled salmon with vegetables, please.', '我想要烤三文鱼配蔬菜。', 'Tip: Order a main course'
FROM asset_scripts d JOIN asset_stages s ON d.stage_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = 'Restaurant Ordering Full Dialogue'
ON CONFLICT (script_id, turn_number) DO NOTHING;

INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, notes)
SELECT d.id, 10, 'assistant', 'Waiter', 'Excellent choice! How would you like your salmon cooked?', '很好的选择！您希望三文鱼怎么烹饪？', NULL
FROM asset_scripts d JOIN asset_stages s ON d.stage_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = 'Restaurant Ordering Full Dialogue'
ON CONFLICT (script_id, turn_number) DO NOTHING;

INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, notes)
SELECT d.id, 11, 'user', 'Customer', 'Medium, please.', '五分熟，谢谢。', 'Tip: Specify doneness'
FROM asset_scripts d JOIN asset_stages s ON d.stage_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = 'Restaurant Ordering Full Dialogue'
ON CONFLICT (script_id, turn_number) DO NOTHING;

INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, notes)
SELECT d.id, 12, 'assistant', 'Waiter', 'Perfect. Your order will be ready in about 15 minutes. Enjoy your meal!', '好的。您的餐大约15分钟后准备好。祝您用餐愉快！', NULL
FROM asset_scripts d JOIN asset_stages s ON d.stage_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = 'Restaurant Ordering Full Dialogue'
ON CONFLICT (script_id, turn_number) DO NOTHING;

-- Insert additional reading subjects
INSERT INTO asset_read_subjects (title_en, title_zh, description_en, description_zh, difficulty, subject_type) VALUES
('Daily Chat', '日常对话', 'Common phrases for everyday situations', '日常情境中的常用表达', 3, 'sentence'),
('Business English', '商务英语', 'Professional expressions for the workplace', '职场专业表达', 5, 'sentence'),
('Advanced Expressions', '高级表达', 'Sophisticated phrases for fluent communication', '流利交流的高级表达', 7, 'sentence')
ON CONFLICT DO NOTHING;

-- Insert reading sentences for Daily Chat
INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 1, 'Could you please help me with this?', '你能帮我一下吗？', '["Could you"]', '["Could you liaison sound"]'
FROM asset_read_subjects e WHERE e.title_en = 'Daily Chat'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 2, 'I would like to make a reservation.', '我想预订一下。', '["would like"]', '["would like weak sound"]'
FROM asset_read_subjects e WHERE e.title_en = 'Daily Chat'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 3, 'Thank you for your patience.', '感谢您的耐心等待。', '["thank you"]', '["thank you liaison"]'
FROM asset_read_subjects e WHERE e.title_en = 'Daily Chat'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 4, 'Could you repeat that more slowly?', '你能说慢一点吗？', '["that"]', '["that weak sound"]'
FROM asset_read_subjects e WHERE e.title_en = 'Daily Chat'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 5, 'I completely agree with you.', '我完全同意你的看法。', '["completely"]', '["completely stress on second syllable"]'
FROM asset_read_subjects e WHERE e.title_en = 'Daily Chat'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

-- Insert reading sentences for Business English
INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 1, 'What time does the meeting start?', '会议几点开始？', '["meeting"]', '["question intonation"]'
FROM asset_read_subjects e WHERE e.title_en = 'Business English'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 2, 'I''m afraid there''s been a misunderstanding.', '恐怕有些误会。', '["there''s been"]', '["there''s been liaison"]'
FROM asset_read_subjects e WHERE e.title_en = 'Business English'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 3, 'Would it be possible to reschedule our appointment?', '可以重新安排我们的预约吗？', '["reschedule"]', '["formal tone"]'
FROM asset_read_subjects e WHERE e.title_en = 'Business English'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 4, 'I''m looking forward to hearing from you soon.', '期待尽快收到您的回复。', '["looking forward to"]', '["looking forward to usage"]'
FROM asset_read_subjects e WHERE e.title_en = 'Business English'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 5, 'Let me get back to you on that.', '这件事我稍后给您答复。', '["get back to"]', '["workplace common expression"]'
FROM asset_read_subjects e WHERE e.title_en = 'Business English'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

-- Insert reading sentences for Advanced Expressions
INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 1, 'Despite the challenges, we managed to complete the project on time.', '尽管面临挑战，我们还是按时完成了项目。', '["despite"]', '["despite usage and long sentence rhythm"]'
FROM asset_read_subjects e WHERE e.title_en = 'Advanced Expressions'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 2, 'The conference has been postponed until further notice.', '会议已延期，另行通知。', '["postponed"]', '["formal written to spoken"]'
FROM asset_read_subjects e WHERE e.title_en = 'Advanced Expressions'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 3, 'I''d appreciate it if you could look into this matter.', '如果您能调查此事，我将不胜感激。', '["appreciate"]', '["polite request advanced expression"]'
FROM asset_read_subjects e WHERE e.title_en = 'Advanced Expressions'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 4, 'Could you elaborate on your previous point?', '您能详细说明一下之前的观点吗？', '["elaborate"]', '["meeting discussion expression"]'
FROM asset_read_subjects e WHERE e.title_en = 'Advanced Expressions'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, focus_sounds, common_mistakes)
SELECT e.id, 5, 'The weather forecast says it will rain tomorrow.', '天气预报说明天会下雨。', '["forecast", "tomorrow"]', '["forecast and tomorrow stress"]'
FROM asset_read_subjects e WHERE e.title_en = 'Advanced Expressions'
ON CONFLICT (subject_id, sentence_order) DO NOTHING;
