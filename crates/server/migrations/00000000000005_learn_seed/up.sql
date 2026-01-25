-- Seed data for learning content
-- This migration populates the database with initial learning content

-- ============================================================================
-- TAXONOMY - DOMAINS AND CATEGORIES
-- ============================================================================

-- Insert context domain
INSERT INTO taxon_domains (code, name_en, name_zh) VALUES
('context', 'Context', '场景')
ON CONFLICT DO NOTHING;

-- Insert context categories
INSERT INTO taxon_categories (code, name_en, name_zh, domain_id) VALUES
('daily_life', 'Daily Life', '日常生活', (SELECT id FROM taxon_domains WHERE name_en = 'Context')),
('business', 'Business', '商务', (SELECT id FROM taxon_domains WHERE name_en = 'Context')),
('travel', 'Travel', '旅行', (SELECT id FROM taxon_domains WHERE name_en = 'Context'))
ON CONFLICT DO NOTHING;

-- ============================================================================
-- SCENARIOS (asset_contexts)
-- ============================================================================

INSERT INTO asset_contexts (code, name_en, name_zh, description_en, description_zh, icon_emoji, difficulty, display_order, is_active) VALUES
('airport_checkin', 'Airport Check-in', '机场值机', 'Practice conversations at airport check-in counters', '练习机场值机柜台对话', '✈️', 3, 1, true),
('hotel_reservation', 'Hotel Reservation', '酒店预订', 'Learn to book rooms and handle hotel situations', '学习预订房间和处理酒店情况', '🏨', 3, 2, true),
('restaurant_ordering', 'Restaurant Ordering', '餐厅点餐', 'Order food and interact with restaurant staff', '点餐和与餐厅员工互动', '🍽️', 3, 3, true),
('job_interview', 'Job Interview', '求职面试', 'Prepare for professional job interviews', '准备专业求职面试', '💼', 4, 4, true),
('doctor_visit', 'Doctor Visit', '看医生', 'Describe symptoms and understand medical advice', '描述症状和理解医疗建议', '🏥', 3, 5, true),
('shopping', 'Shopping', '购物', 'Shop for clothes, electronics, and negotiate prices', '购买衣服、电子产品和讨价还价', '🛍️', 2, 6, true),
('business_meeting', 'Business Meeting', '商务会议', 'Participate in professional meetings and presentations', '参加专业会议和演示', '📊', 4, 7, true),
('asking_for_directions', 'Asking for Directions', '问路', 'Ask for and give directions in various situations', '在各种情况下问路和指路', '🗺️', 2, 8, true),
('phone_call', 'Phone Call', '电话沟通', 'Handle phone conversations professionally', '专业处理电话交流', '📞', 3, 9, true),
('coffee_shop', 'Coffee Shop', '咖啡店', 'Order drinks and have casual conversations', '点饮料和进行日常交谈', '☕', 1, 10, true)
ON CONFLICT DO NOTHING;

-- ============================================================================
-- STAGES (asset_stages) - Mirror contexts for script linking
-- ============================================================================

INSERT INTO asset_stages (code, name_en, name_zh, description_en, description_zh, icon_emoji, difficulty, display_order, is_active) VALUES
('airport_checkin', 'Airport Check-in', '机场值机', 'Practice conversations at airport check-in counters', '练习机场值机柜台对话', '✈️', 3, 1, true),
('hotel_reservation', 'Hotel Reservation', '酒店预订', 'Learn to book rooms and handle hotel situations', '学习预订房间和处理酒店情况', '🏨', 3, 2, true),
('restaurant_ordering', 'Restaurant Ordering', '餐厅点餐', 'Order food and interact with restaurant staff', '点餐和与餐厅员工互动', '🍽️', 3, 3, true),
('job_interview', 'Job Interview', '求职面试', 'Prepare for professional job interviews', '准备专业求职面试', '💼', 4, 4, true),
('doctor_visit', 'Doctor Visit', '看医生', 'Describe symptoms and understand medical advice', '描述症状和理解医疗建议', '🏥', 3, 5, true),
('shopping', 'Shopping', '购物', 'Shop for clothes, electronics, and negotiate prices', '购买衣服、电子产品和讨价还价', '🛍️', 2, 6, true),
('business_meeting', 'Business Meeting', '商务会议', 'Participate in professional meetings and presentations', '参加专业会议和演示', '📊', 4, 7, true),
('asking_for_directions', 'Asking for Directions', '问路', 'Ask for and give directions in various situations', '在各种情况下问路和指路', '🗺️', 2, 8, true),
('phone_call', 'Phone Call', '电话沟通', 'Handle phone conversations professionally', '专业处理电话交流', '📞', 3, 9, true),
('coffee_shop', 'Coffee Shop', '咖啡店', 'Order drinks and have casual conversations', '点饮料和进行日常交谈', '☕', 1, 10, true)
ON CONFLICT DO NOTHING;

-- Link contexts to categories
INSERT INTO asset_context_categories (context_id, category_id) VALUES
-- Travel contexts
((SELECT id FROM asset_contexts WHERE name_en = 'Airport Check-in'), (SELECT id FROM taxon_categories WHERE name_en = 'Travel')),
((SELECT id FROM asset_contexts WHERE name_en = 'Hotel Reservation'), (SELECT id FROM taxon_categories WHERE name_en = 'Travel')),
((SELECT id FROM asset_contexts WHERE name_en = 'Asking for Directions'), (SELECT id FROM taxon_categories WHERE name_en = 'Travel')),

-- Daily Life contexts
((SELECT id FROM asset_contexts WHERE name_en = 'Restaurant Ordering'), (SELECT id FROM taxon_categories WHERE name_en = 'Daily Life')),
((SELECT id FROM asset_contexts WHERE name_en = 'Doctor Visit'), (SELECT id FROM taxon_categories WHERE name_en = 'Daily Life')),
((SELECT id FROM asset_contexts WHERE name_en = 'Shopping'), (SELECT id FROM taxon_categories WHERE name_en = 'Daily Life')),
((SELECT id FROM asset_contexts WHERE name_en = 'Coffee Shop'), (SELECT id FROM taxon_categories WHERE name_en = 'Daily Life')),

-- Business contexts
((SELECT id FROM asset_contexts WHERE name_en = 'Job Interview'), (SELECT id FROM taxon_categories WHERE name_en = 'Business')),
((SELECT id FROM asset_contexts WHERE name_en = 'Business Meeting'), (SELECT id FROM taxon_categories WHERE name_en = 'Business')),
((SELECT id FROM asset_contexts WHERE name_en = 'Phone Call'), (SELECT id FROM taxon_categories WHERE name_en = 'Business'))
ON CONFLICT (context_id, category_id) DO NOTHING;

-- ============================================================================
-- SCENE DIALOGUES (asset_scripts)
-- ============================================================================

-- Airport Check-in scripts
INSERT INTO asset_scripts (stage_id, code, title_en, title_zh, description_en, description_zh, total_turns, estimated_duration_seconds, difficulty) VALUES
((SELECT id FROM asset_stages WHERE name_en = 'Airport Check-in'), 'basic_checkin', 'Basic Check-in', '基础值机', 'A simple check-in conversation', '简单的值机对话', 8, 120, 3),
((SELECT id FROM asset_stages WHERE name_en = 'Airport Check-in'), 'overweight_luggage', 'Overweight Luggage', '行李超重', 'Handling overweight baggage situation', '处理行李超重的情况', 10, 180, 5),
((SELECT id FROM asset_stages WHERE name_en = 'Airport Check-in'), 'seat_upgrade_request', 'Seat Upgrade Request', '升舱请求', 'Requesting a seat upgrade', '请求升舱', 8, 150, 5)
ON CONFLICT DO NOTHING;

-- Hotel Reservation scripts
INSERT INTO asset_scripts (stage_id, code, title_en, title_zh, description_en, description_zh, total_turns, estimated_duration_seconds, difficulty) VALUES
((SELECT id FROM asset_stages WHERE name_en = 'Hotel Reservation'), 'making_reservation', 'Making a Reservation', '预订房间', 'Booking a hotel room', '预订酒店房间', 8, 120, 3),
((SELECT id FROM asset_stages WHERE name_en = 'Hotel Reservation'), 'checking_in', 'Checking In', '办理入住', 'Hotel check-in process', '酒店入住流程', 6, 90, 3),
((SELECT id FROM asset_stages WHERE name_en = 'Hotel Reservation'), 'room_complaint', 'Room Complaint', '房间投诉', 'Handling issues with the room', '处理房间问题', 10, 180, 5)
ON CONFLICT DO NOTHING;

-- Restaurant scripts
INSERT INTO asset_scripts (stage_id, code, title_en, title_zh, description_en, description_zh, total_turns, estimated_duration_seconds, difficulty) VALUES
((SELECT id FROM asset_stages WHERE name_en = 'Restaurant Ordering'), 'ordering_meal', 'Ordering a Meal', '点餐', 'Basic restaurant ordering', '基础餐厅点餐', 8, 120, 3),
((SELECT id FROM asset_stages WHERE name_en = 'Restaurant Ordering'), 'special_dietary_needs', 'Special Dietary Needs', '特殊饮食需求', 'Explaining allergies and preferences', '解释过敏和偏好', 10, 150, 5),
((SELECT id FROM asset_stages WHERE name_en = 'Restaurant Ordering'), 'paying_bill', 'Paying the Bill', '结账', 'Asking for the check and paying', '要账单和付款', 6, 90, 3)
ON CONFLICT DO NOTHING;

-- ============================================================================
-- DIALOGUE TURNS (asset_script_turns)
-- ============================================================================

-- Basic Check-in script turns
INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, phonetic_transcription, asset_phrases, notes) VALUES
((SELECT id FROM asset_scripts WHERE title_en = 'Basic Check-in'), 1, 'npc', 'Agent', 'Good morning! May I see your passport and ticket, please?', '早上好！请出示您的护照和机票好吗？', '/ɡʊd ˈmɔːrnɪŋ meɪ aɪ siː jɔːr ˈpæspɔːrt ænd ˈtɪkɪt pliːz/', '["May I see", "please"]', 'Polite request format'),
((SELECT id FROM asset_scripts WHERE title_en = 'Basic Check-in'), 2, 'user', 'Traveler', 'Here you go. I have a flight to New York.', '给您。我有一班飞往纽约的航班。', '/hɪr juː ɡoʊ aɪ hæv ə flaɪt tuː nuː jɔːrk/', '["Here you go", "flight to"]', NULL),
((SELECT id FROM asset_scripts WHERE title_en = 'Basic Check-in'), 3, 'npc', 'Agent', 'Would you like a window or aisle seat?', '您想要靠窗还是靠过道的座位？', '/wʊd juː laɪk ə ˈwɪndoʊ ɔːr aɪl siːt/', '["Would you like", "window or aisle"]', 'Common seat preference question'),
((SELECT id FROM asset_scripts WHERE title_en = 'Basic Check-in'), 4, 'user', 'Traveler', 'A window seat, please.', '请给我靠窗的座位。', '/ə ˈwɪndoʊ siːt pliːz/', '["please"]', NULL),
((SELECT id FROM asset_scripts WHERE title_en = 'Basic Check-in'), 5, 'npc', 'Agent', 'How many bags are you checking in today?', '您今天要托运几件行李？', '/haʊ ˈmeni bæɡz ɑːr juː ˈtʃekɪŋ ɪn təˈdeɪ/', '["How many", "checking in"]', NULL),
((SELECT id FROM asset_scripts WHERE title_en = 'Basic Check-in'), 6, 'user', 'Traveler', 'Just one suitcase.', '只有一个行李箱。', '/dʒʌst wʌn ˈsuːtkeɪs/', '["Just one"]', NULL),
((SELECT id FROM asset_scripts WHERE title_en = 'Basic Check-in'), 7, 'npc', 'Agent', 'Here is your boarding pass. Gate 15, boarding at 10:30.', '这是您的登机牌。15号登机口，10:30开始登机。', '/hɪr ɪz jɔːr ˈbɔːrdɪŋ pæs ɡeɪt fɪfˈtiːn ˈbɔːrdɪŋ æt ten ˈθɜːrti/', '["boarding pass", "Gate", "boarding at"]', NULL),
((SELECT id FROM asset_scripts WHERE title_en = 'Basic Check-in'), 8, 'user', 'Traveler', 'Thank you very much!', '非常感谢！', '/θæŋk juː ˈveri mʌtʃ/', '["Thank you very much"]', NULL)
ON CONFLICT (script_id, turn_number) DO NOTHING;

-- Ordering a Meal script turns
INSERT INTO asset_script_turns (script_id, turn_number, speaker_role, speaker_name, content_en, content_zh, phonetic_transcription, asset_phrases, notes) VALUES
((SELECT id FROM asset_scripts WHERE title_en = 'Ordering a Meal'), 1, 'npc', 'Waiter', 'Welcome! Here is your menu. Can I get you something to drink?', '欢迎光临！这是您的菜单。要点些喝的吗？', NULL, '["Can I get you", "something to drink"]', NULL),
((SELECT id FROM asset_scripts WHERE title_en = 'Ordering a Meal'), 2, 'user', 'Customer', 'Yes, I will have a glass of water, please.', '好的，请给我一杯水。', NULL, '["I will have", "a glass of"]', NULL),
((SELECT id FROM asset_scripts WHERE title_en = 'Ordering a Meal'), 3, 'npc', 'Waiter', 'Are you ready to order, or do you need a few more minutes?', '您准备好点餐了吗，还是需要再看一会儿？', NULL, '["ready to order", "a few more minutes"]', NULL),
((SELECT id FROM asset_scripts WHERE title_en = 'Ordering a Meal'), 4, 'user', 'Customer', 'I am ready. I would like the grilled salmon, please.', '我准备好了。我想要烤三文鱼。', NULL, '["I would like", "grilled salmon"]', NULL),
((SELECT id FROM asset_scripts WHERE title_en = 'Ordering a Meal'), 5, 'npc', 'Waiter', 'Excellent choice! Would you like any sides with that?', '很好的选择！您要配菜吗？', NULL, '["Excellent choice", "any sides"]', NULL),
((SELECT id FROM asset_scripts WHERE title_en = 'Ordering a Meal'), 6, 'user', 'Customer', 'Yes, I will have the mashed potatoes and a salad.', '好的，我要土豆泥和一份沙拉。', NULL, '["I will have", "mashed potatoes"]', NULL),
((SELECT id FROM asset_scripts WHERE title_en = 'Ordering a Meal'), 7, 'npc', 'Waiter', 'Perfect. Your order will be ready in about 15 minutes.', '好的。您的菜大约15分钟后上。', NULL, '["will be ready", "in about"]', NULL),
((SELECT id FROM asset_scripts WHERE title_en = 'Ordering a Meal'), 8, 'user', 'Customer', 'That sounds great, thank you!', '太好了，谢谢！', NULL, '["That sounds great"]', NULL)
ON CONFLICT (script_id, turn_number) DO NOTHING;

-- ============================================================================
-- TAXONOMY - CLASSIC DOMAIN AND CATEGORIES
-- ============================================================================

-- Insert classic domain
INSERT INTO taxon_domains (code, name_en, name_zh) VALUES ('stage', 'Stage', '角色扮演') ON CONFLICT DO NOTHING;

-- Insert classic categories (movie, tv_show, ted_talk)
INSERT INTO taxon_categories (code, name_en, name_zh, domain_id) VALUES
('movie', 'Movie', '电影', (SELECT id FROM taxon_domains WHERE code = 'stage')),
('tv_show', 'TV Show', '电视剧', (SELECT id FROM taxon_domains WHERE code = 'stage')),
('ted_talk', 'TED Talk', 'TED演讲', (SELECT id FROM taxon_domains WHERE code = 'stage'))
ON CONFLICT DO NOTHING;

INSERT INTO taxon_domains (code, name_en, name_zh) VALUES ('dictionary', 'Dictionary', '词典') ON CONFLICT DO NOTHING;
-- ============================================================================
-- READING SUBJECTS (asset_read_subjects)
-- ============================================================================

INSERT INTO asset_read_subjects (code, title_en, title_zh, description_en, description_zh, difficulty, subject_type) VALUES
('daily_greetings', 'Daily Greetings', '日常问候', 'Practice common greeting phrases', '练习常见问候短语', 3, 'sentence'),
('business_introductions', 'Business Introductions', '商务介绍', 'Professional introduction phrases', '专业介绍短语', 5, 'sentence'),
('travel_conversations', 'Travel Conversations', '旅行对话', 'Useful phrases for traveling', '旅行中有用的短语', 3, 'sentence'),
('tongue_twisters', 'Tongue Twisters', '绕口令', 'Fun pronunciation practice', '有趣的发音练习', 5, 'sentence'),
('news_reading', 'News Reading', '新闻阅读', 'Practice reading news articles', '练习阅读新闻文章', 8, 'sentence')
ON CONFLICT DO NOTHING;

-- ============================================================================
-- READING SENTENCES (asset_read_sentences)
-- ============================================================================

-- Daily Greetings sentences
INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, phonetic_transcription, focus_sounds, common_mistakes) VALUES
((SELECT id FROM asset_read_subjects WHERE title_en = 'Daily Greetings'), 1, 'Good morning! How are you doing today?', '早上好！你今天怎么样？', '/ɡʊd ˈmɔːrnɪŋ haʊ ɑːr juː ˈduːɪŋ təˈdeɪ/', '["morning", "doing"]', '["mor-ning not mourning", "today stress on second syllable"]'),
((SELECT id FROM asset_read_subjects WHERE title_en = 'Daily Greetings'), 2, 'Nice to meet you. My name is John.', '很高兴认识你。我叫John。', '/naɪs tuː miːt juː maɪ neɪm ɪz dʒɑːn/', '["meet", "name"]', '["meet vs mit", "name long a sound"]'),
((SELECT id FROM asset_read_subjects WHERE title_en = 'Daily Greetings'), 3, 'How was your weekend?', '你周末过得怎么样？', '/haʊ wɒz jɔːr ˈwiːkend/', '["was", "weekend"]', '["weekend stress on first syllable"]'),
((SELECT id FROM asset_read_subjects WHERE title_en = 'Daily Greetings'), 4, 'See you later! Have a great day!', '回头见！祝你有美好的一天！', '/siː juː ˈleɪtər hæv ə ɡreɪt deɪ/', '["later", "great"]', '["later vs letter", "great long a"]'),
((SELECT id FROM asset_read_subjects WHERE title_en = 'Daily Greetings'), 5, 'Thank you so much for your help.', '非常感谢你的帮助。', '/θæŋk juː soʊ mʌtʃ fɔːr jɔːr help/', '["thank", "much"]', '["th sound", "much not match"]')
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

-- Business Introductions sentences
INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, phonetic_transcription, focus_sounds, common_mistakes) VALUES
((SELECT id FROM asset_read_subjects WHERE title_en = 'Business Introductions'), 1, 'Allow me to introduce myself. I am the marketing director.', '请允许我自我介绍。我是市场总监。', NULL, '["introduce", "director"]', '["introduce stress on third syllable"]'),
((SELECT id FROM asset_read_subjects WHERE title_en = 'Business Introductions'), 2, 'It is a pleasure to meet you. I have heard great things about your company.', '很高兴认识你。我听说过很多关于贵公司的好消息。', NULL, '["pleasure", "company"]', '["pleasure zh sound", "company stress on first syllable"]'),
((SELECT id FROM asset_read_subjects WHERE title_en = 'Business Introductions'), 3, 'I am responsible for overseeing the sales department.', '我负责监管销售部门。', NULL, '["responsible", "overseeing"]', '["responsible stress pattern"]'),
((SELECT id FROM asset_read_subjects WHERE title_en = 'Business Introductions'), 4, 'Our company specializes in software development.', '我们公司专门从事软件开发。', NULL, '["specializes", "development"]', '["specializes s vs z sound"]'),
((SELECT id FROM asset_read_subjects WHERE title_en = 'Business Introductions'), 5, 'I would be happy to schedule a follow-up meeting.', '我很乐意安排一次后续会议。', NULL, '["schedule", "follow-up"]', '["schedule British vs American pronunciation"]')
ON CONFLICT (subject_id, sentence_order) DO NOTHING;

-- Tongue Twisters
INSERT INTO asset_read_sentences (subject_id, sentence_order, content_en, content_zh, phonetic_transcription, focus_sounds, common_mistakes) VALUES
((SELECT id FROM asset_read_subjects WHERE title_en = 'Tongue Twisters'), 1, 'She sells seashells by the seashore.', '她在海边卖贝壳。', '/ʃiː selz ˈsiːʃelz baɪ ðə ˈsiːʃɔːr/', '["sh", "s"]', '["distinguish sh and s sounds"]'),
((SELECT id FROM asset_read_subjects WHERE title_en = 'Tongue Twisters'), 2, 'Peter Piper picked a peck of pickled peppers.', 'Peter Piper摘了一配克腌辣椒。', '/ˈpiːtər ˈpaɪpər pɪkt ə pek əv ˈpɪkld ˈpepərz/', '["p"]', '["p sound aspiration"]'),
((SELECT id FROM asset_read_subjects WHERE title_en = 'Tongue Twisters'), 3, 'How much wood would a woodchuck chuck if a woodchuck could chuck wood?', '如果土拨鼠能扔木头，它能扔多少木头？', NULL, '["w", "ch"]', '["w vs v", "ch sound"]'),
((SELECT id FROM asset_read_subjects WHERE title_en = 'Tongue Twisters'), 4, 'Red lorry, yellow lorry, red lorry, yellow lorry.', '红卡车，黄卡车，红卡车，黄卡车。', NULL, '["r", "l"]', '["r vs l distinction"]'),
((SELECT id FROM asset_read_subjects WHERE title_en = 'Tongue Twisters'), 5, 'The thirty-three thieves thought that they thrilled the throne throughout Thursday.', '三十三个小偷认为他们在周四一整天都让王座兴奋不已。', NULL, '["th"]', '["th voiced vs unvoiced"]')
ON CONFLICT (subject_id, sentence_order) DO NOTHING;
