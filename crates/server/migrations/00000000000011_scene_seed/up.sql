-- ============================================================================
-- SEED DATA FOR SCENES AND READING
-- ============================================================================

-- Insert scenes into asset_scenes
INSERT INTO asset_scenes (name_en, name_zh, description_en, description_zh, icon_emoji, difficulty, category, display_order, is_active, duration_minutes, is_featured) VALUES
('Restaurant Ordering', '餐厅点餐', 'Learn common English expressions for ordering at a restaurant', '学习在餐厅点餐的常用英语表达', '🍽️', 'beginner', 'daily', 1, true, 5, true),
('Hotel Check-in', '酒店入住', 'Master English conversations for hotel front desk check-in', '掌握酒店前台办理入住的英语对话', '🏨', 'beginner', 'travel', 2, true, 8, true),
('Airport Travel', '机场出行', 'Learn common English for airport security and boarding', '学习机场安检、登机等常用英语', '✈️', 'intermediate', 'travel', 3, true, 10, true),
('Grocery Shopping', '超市购物', 'English communication skills for grocery shopping', '超市购物时的英语交流技巧', '🛒', 'beginner', 'daily', 4, true, 5, true),
('Job Interview', '工作面试', 'Professional English for job interviews', '职场面试英语表达与技巧', '💼', 'advanced', 'business', 5, true, 15, true),
('Doctor Visit', '看病就医', 'Learn English communication for medical visits', '学习就医时的英语沟通', '🏥', 'intermediate', 'daily', 6, true, 10, false),
('Banking', '银行业务', 'English expressions for banking services', '银行业务办理的英语表达', '🏦', 'intermediate', 'daily', 7, true, 8, false),
('Phone Booking', '电话预约', 'English conversation skills for phone reservations', '电话预约的英语对话技巧', '📞', 'intermediate', 'daily', 8, true, 6, false),
('Coffee Shop Order', '咖啡店点单', 'Authentic English for ordering at coffee shops', '咖啡店点单的地道英语', '☕', 'beginner', 'daily', 9, true, 5, false),
('Taking a Taxi', '打车出行', 'English communication when taking a taxi', '打车时的英语交流', '🚕', 'beginner', 'travel', 10, true, 5, false),
('Package Delivery', '快递收发', 'English for sending and receiving packages', '收发快递时的英语表达', '📦', 'beginner', 'daily', 11, true, 5, false),
('Movie Tickets', '电影购票', 'English dialogue for buying movie tickets', '电影院购票的英语对话', '🎬', 'beginner', 'entertainment', 12, true, 5, false)
ON CONFLICT (name_en) DO UPDATE SET
    description_en = EXCLUDED.description_en,
    description_zh = EXCLUDED.description_zh,
    icon_emoji = EXCLUDED.icon_emoji,
    difficulty = EXCLUDED.difficulty,
    category = EXCLUDED.category,
    display_order = EXCLUDED.display_order,
    duration_minutes = EXCLUDED.duration_minutes,
    is_featured = EXCLUDED.is_featured;

-- Insert dialogues for asset_dialogues (need scene_id from asset_scenes)
INSERT INTO asset_dialogues (scene_id, title_en, title_zh, description_en, description_zh, total_turns, estimated_duration_seconds, difficulty)
SELECT s.id, '餐厅点餐完整对话', 'Restaurant Ordering Full Dialogue', 'Complete dialogue practice for restaurant ordering', '餐厅点餐完整对话练习', 12, 300, 'beginner'
FROM asset_scenes s WHERE s.name_en = 'Restaurant Ordering'
ON CONFLICT DO NOTHING;

INSERT INTO asset_dialogues (scene_id, title_en, title_zh, description_en, description_zh, total_turns, estimated_duration_seconds, difficulty)
SELECT s.id, '酒店入住完整对话', 'Hotel Check-in Full Dialogue', 'Complete dialogue practice for hotel check-in', '酒店入住完整对话练习', 13, 480, 'beginner'
FROM asset_scenes s WHERE s.name_en = 'Hotel Check-in'
ON CONFLICT DO NOTHING;

INSERT INTO asset_dialogues (scene_id, title_en, title_zh, description_en, description_zh, total_turns, estimated_duration_seconds, difficulty)
SELECT s.id, '机场出行完整对话', 'Airport Travel Full Dialogue', 'Complete dialogue practice for airport travel', '机场出行完整对话练习', 12, 600, 'intermediate'
FROM asset_scenes s WHERE s.name_en = 'Airport Travel'
ON CONFLICT DO NOTHING;

-- Insert dialogue turns for restaurant ordering
INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, content_en, content_zh, notes)
SELECT d.id, 1, 'assistant', 'Good evening! Welcome to The Garden Restaurant. Do you have a reservation?', '晚上好！欢迎来到花园餐厅。请问您有预订吗？', NULL
FROM asset_dialogues d JOIN asset_scenes s ON d.scene_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = '餐厅点餐完整对话'
ON CONFLICT (dialogue_id, turn_number) DO NOTHING;

INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, content_en, content_zh, notes)
SELECT d.id, 2, 'user', 'Yes, I have a reservation under the name Smith for two people.', '是的，我有预订，史密斯，两位。', '提示: 说出你的姓名和人数'
FROM asset_dialogues d JOIN asset_scenes s ON d.scene_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = '餐厅点餐完整对话'
ON CONFLICT (dialogue_id, turn_number) DO NOTHING;

INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, content_en, content_zh, notes)
SELECT d.id, 3, 'assistant', 'Perfect, Mr. Smith. Please follow me. Here is your table.', '好的，史密斯先生。请跟我来。这是您的座位。', NULL
FROM asset_dialogues d JOIN asset_scenes s ON d.scene_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = '餐厅点餐完整对话'
ON CONFLICT (dialogue_id, turn_number) DO NOTHING;

INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, content_en, content_zh, notes)
SELECT d.id, 4, 'assistant', 'Here are your menus. Can I get you something to drink while you decide?', '这是菜单。您在看菜单时，要先喝点什么吗？', NULL
FROM asset_dialogues d JOIN asset_scenes s ON d.scene_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = '餐厅点餐完整对话'
ON CONFLICT (dialogue_id, turn_number) DO NOTHING;

INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, content_en, content_zh, notes)
SELECT d.id, 5, 'user', 'Could I have a glass of water and a cup of coffee, please?', '请给我一杯水和一杯咖啡。', '提示: 点一些饮料'
FROM asset_dialogues d JOIN asset_scenes s ON d.scene_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = '餐厅点餐完整对话'
ON CONFLICT (dialogue_id, turn_number) DO NOTHING;

INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, content_en, content_zh, notes)
SELECT d.id, 6, 'assistant', 'Certainly. Still or sparkling water?', '好的。是矿泉水还是苏打水？', NULL
FROM asset_dialogues d JOIN asset_scenes s ON d.scene_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = '餐厅点餐完整对话'
ON CONFLICT (dialogue_id, turn_number) DO NOTHING;

INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, content_en, content_zh, notes)
SELECT d.id, 7, 'user', 'Still water, please.', '矿泉水，谢谢。', '提示: 选择水的类型'
FROM asset_dialogues d JOIN asset_scenes s ON d.scene_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = '餐厅点餐完整对话'
ON CONFLICT (dialogue_id, turn_number) DO NOTHING;

INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, content_en, content_zh, notes)
SELECT d.id, 8, 'assistant', 'Are you ready to order, or do you need a few more minutes?', '您准备好点餐了吗，还是需要再看一下？', NULL
FROM asset_dialogues d JOIN asset_scenes s ON d.scene_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = '餐厅点餐完整对话'
ON CONFLICT (dialogue_id, turn_number) DO NOTHING;

INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, content_en, content_zh, notes)
SELECT d.id, 9, 'user', 'I would like the grilled salmon with vegetables, please.', '我想要烤三文鱼配蔬菜。', '提示: 点一道主菜'
FROM asset_dialogues d JOIN asset_scenes s ON d.scene_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = '餐厅点餐完整对话'
ON CONFLICT (dialogue_id, turn_number) DO NOTHING;

INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, content_en, content_zh, notes)
SELECT d.id, 10, 'assistant', 'Excellent choice! How would you like your salmon cooked?', '很好的选择！您希望三文鱼怎么烹饪？', NULL
FROM asset_dialogues d JOIN asset_scenes s ON d.scene_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = '餐厅点餐完整对话'
ON CONFLICT (dialogue_id, turn_number) DO NOTHING;

INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, content_en, content_zh, notes)
SELECT d.id, 11, 'user', 'Medium, please.', '五分熟，谢谢。', '提示: 说出烹饪程度'
FROM asset_dialogues d JOIN asset_scenes s ON d.scene_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = '餐厅点餐完整对话'
ON CONFLICT (dialogue_id, turn_number) DO NOTHING;

INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, content_en, content_zh, notes)
SELECT d.id, 12, 'assistant', 'Perfect. Your order will be ready in about 15 minutes. Enjoy your meal!', '好的。您的餐大约15分钟后准备好。祝您用餐愉快！', NULL
FROM asset_dialogues d JOIN asset_scenes s ON d.scene_id = s.id WHERE s.name_en = 'Restaurant Ordering' AND d.title_en = '餐厅点餐完整对话'
ON CONFLICT (dialogue_id, turn_number) DO NOTHING;

-- Insert classic sources
INSERT INTO asset_classic_sources (source_type, title, year, description_en, description_zh, difficulty, icon_emoji, is_featured, display_order) VALUES
('movie', 'The Shawshank Redemption', 1994, 'Hope is a good thing - Classic inspirational dialogue', '希望是个好东西 - 经典励志台词', 'intermediate', '🎬', true, 1),
('tv_show', 'Friends', 1994, 'Daily conversation highlights - Authentic American English', '日常对话精选 - 地道美式口语', 'beginner', '📺', true, 2),
('ted_talk', 'Your Body Language May Shape Who You Are', 2012, 'Confident expression skills - Amy Cuddy TED Talk', '自信表达技巧 - Amy Cuddy TED演讲', 'intermediate', '🎤', true, 3),
('movie', 'Forrest Gump', 1994, 'Classic inspirational dialogue - Life is like a box of chocolates', '经典励志台词 - 人生如巧克力', 'beginner', '🎬', true, 4),
('tv_show', 'The Office', 2005, 'Office humor dialogues - American workplace culture', '职场幽默对话 - 美式职场文化', 'intermediate', '📺', false, 5),
('ted_talk', 'How Great Leaders Inspire Action', 2009, 'Leadership presentation skills - Simon Sinek', '领导力演讲技巧 - Simon Sinek', 'advanced', '🎤', false, 6)
ON CONFLICT (source_type, title) DO UPDATE SET
    description_en = EXCLUDED.description_en,
    description_zh = EXCLUDED.description_zh,
    difficulty = EXCLUDED.difficulty,
    icon_emoji = EXCLUDED.icon_emoji,
    is_featured = EXCLUDED.is_featured,
    display_order = EXCLUDED.display_order;

-- Insert classic clips for Shawshank Redemption
INSERT INTO asset_classic_clips (source_id, clip_title_en, clip_title_zh, transcript_en, transcript_zh, key_vocabulary, cultural_notes)
SELECT s.id, 'Hope Speech', '希望演讲',
       'Hope is a good thing, maybe the best of things, and no good thing ever dies.',
       '希望是美好的，也许是人间至善，而美好的事物永不消逝。',
       '["hope", "best", "dies"]'::jsonb,
       'This is one of the most famous quotes from the movie'
FROM asset_classic_sources s WHERE s.title = 'The Shawshank Redemption'
ON CONFLICT DO NOTHING;

INSERT INTO asset_classic_clips (source_id, clip_title_en, clip_title_zh, transcript_en, transcript_zh, key_vocabulary, cultural_notes)
SELECT s.id, 'Get Busy Living', '忙着活',
       'Get busy living, or get busy dying.',
       '要么忙着活，要么忙着死。',
       '["busy", "living", "dying"]'::jsonb,
       'A motivational quote about making choices in life'
FROM asset_classic_sources s WHERE s.title = 'The Shawshank Redemption'
ON CONFLICT DO NOTHING;

-- Insert classic clips for Friends
INSERT INTO asset_classic_clips (source_id, clip_title_en, clip_title_zh, transcript_en, transcript_zh, key_vocabulary, cultural_notes)
SELECT s.id, 'How You Doin', 'Joey经典台词',
       'How you doin''?',
       '你好吗？（Joey经典台词）',
       '["how", "doing"]'::jsonb,
       'Joey''s signature pickup line, a classic Friends moment'
FROM asset_classic_sources s WHERE s.title = 'Friends'
ON CONFLICT DO NOTHING;

-- Insert reading exercises
INSERT INTO asset_read_exercises (title_en, title_zh, description_en, description_zh, difficulty, exercise_type) VALUES
('Daily Conversations', '日常对话', 'Common phrases for everyday situations', '日常情境中的常用表达', 'beginner', 'sentence'),
('Business English', '商务英语', 'Professional expressions for the workplace', '职场专业表达', 'intermediate', 'sentence'),
('Advanced Expressions', '高级表达', 'Sophisticated phrases for fluent communication', '流利交流的高级表达', 'advanced', 'sentence')
ON CONFLICT DO NOTHING;

-- Insert reading sentences for Daily Conversations
INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 1, 'Could you please help me with this?', '你能帮我一下吗？', '注意 "Could you" 的连读，发音像 "Couldja"'
FROM asset_read_exercises e WHERE e.title_en = 'Daily Conversations'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 2, 'I would like to make a reservation.', '我想预订一下。', '注意 "would like" 的弱读，"like to" 连读'
FROM asset_read_exercises e WHERE e.title_en = 'Daily Conversations'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 3, 'Thank you for your patience.', '感谢您的耐心等待。', '注意 "thank you" 的连读'
FROM asset_read_exercises e WHERE e.title_en = 'Daily Conversations'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 4, 'Could you repeat that more slowly?', '你能说慢一点吗？', '注意 "that" 的弱读'
FROM asset_read_exercises e WHERE e.title_en = 'Daily Conversations'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 5, 'I completely agree with you.', '我完全同意你的看法。', '注意 "completely" 的重音在第二音节'
FROM asset_read_exercises e WHERE e.title_en = 'Daily Conversations'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

-- Insert reading sentences for Business English
INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 1, 'What time does the meeting start?', '会议几点开始？', '注意疑问句的升调'
FROM asset_read_exercises e WHERE e.title_en = 'Business English'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 2, 'I''m afraid there''s been a misunderstanding.', '恐怕有些误会。', '注意 "there''s been" 的连读'
FROM asset_read_exercises e WHERE e.title_en = 'Business English'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 3, 'Would it be possible to reschedule our appointment?', '可以重新安排我们的预约吗？', '注意正式语气的表达'
FROM asset_read_exercises e WHERE e.title_en = 'Business English'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 4, 'I''m looking forward to hearing from you soon.', '期待尽快收到您的回复。', '注意 "looking forward to" 的用法'
FROM asset_read_exercises e WHERE e.title_en = 'Business English'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 5, 'Let me get back to you on that.', '这件事我稍后给您答复。', '职场常用表达'
FROM asset_read_exercises e WHERE e.title_en = 'Business English'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

-- Insert reading sentences for Advanced Expressions
INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 1, 'Despite the challenges, we managed to complete the project on time.', '尽管面临挑战，我们还是按时完成了项目。', '注意 "despite" 的用法和长句的节奏'
FROM asset_read_exercises e WHERE e.title_en = 'Advanced Expressions'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 2, 'The conference has been postponed until further notice.', '会议已延期，另行通知。', '正式书面语的口语化表达'
FROM asset_read_exercises e WHERE e.title_en = 'Advanced Expressions'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 3, 'I''d appreciate it if you could look into this matter.', '如果您能调查此事，我将不胜感激。', '礼貌请求的高级表达'
FROM asset_read_exercises e WHERE e.title_en = 'Advanced Expressions'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 4, 'Could you elaborate on your previous point?', '您能详细说明一下之前的观点吗？', '会议讨论常用表达'
FROM asset_read_exercises e WHERE e.title_en = 'Advanced Expressions'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, tips)
SELECT e.id, 5, 'The weather forecast says it will rain tomorrow.', '天气预报说明天会下雨。', '注意 "forecast" 和 "tomorrow" 的重音'
FROM asset_read_exercises e WHERE e.title_en = 'Advanced Expressions'
ON CONFLICT (exercise_id, sentence_order) DO NOTHING;

-- Insert key phrases
INSERT INTO asset_phrases (phrase_en, phrase_zh, usage_context, example_sentence_en, example_sentence_zh, category, formality_level) VALUES
('reservation', '预订', 'Making bookings at restaurants or hotels', 'I have a reservation for two.', '我预订了两位。', 'travel', 'neutral'),
('check-in', '办理入住', 'Hotel and airport contexts', 'What time is check-in?', '几点可以办理入住？', 'travel', 'neutral'),
('check-out', '退房', 'Hotel departure', 'I''d like to check out, please.', '我想办理退房。', 'travel', 'neutral'),
('Could you please...', '你能...吗？', 'Polite requests', 'Could you please help me with this?', '你能帮我一下吗？', 'daily', 'formal'),
('I would like to...', '我想要...', 'Expressing wants politely', 'I would like to make a reservation.', '我想预订一下。', 'daily', 'formal'),
('Thank you for...', '感谢你...', 'Expressing gratitude', 'Thank you for your patience.', '感谢您的耐心等待。', 'daily', 'neutral')
ON CONFLICT (phrase_en) DO UPDATE SET
    phrase_zh = EXCLUDED.phrase_zh,
    usage_context = EXCLUDED.usage_context,
    example_sentence_en = EXCLUDED.example_sentence_en,
    example_sentence_zh = EXCLUDED.example_sentence_zh,
    category = EXCLUDED.category,
    formality_level = EXCLUDED.formality_level;
