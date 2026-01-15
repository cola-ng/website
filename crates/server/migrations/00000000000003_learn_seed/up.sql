-- Seed data for learning content
-- This migration populates the database with initial learning content

-- ============================================================================
-- SCENARIOS
-- ============================================================================

INSERT INTO scenes (name_en, name_zh, description_en, description_zh, icon_emoji, difficulty_level, category, display_order, is_active) VALUES
('Airport Check-in', '机场值机', 'Practice learn_conversations at airport check-in counters', '练习机场值机柜台对话', '✈️', 'beginner', 'travel', 1, true),
('Hotel Reservation', '酒店预订', 'Learn to book rooms and handle hotel situations', '学习预订房间和处理酒店情况', '🏨', 'beginner', 'travel', 2, true),
('Restaurant Ordering', '餐厅点餐', 'Order food and interact with restaurant staff', '点餐和与餐厅员工互动', '🍽️', 'beginner', 'daily', 3, true),
('Job Interview', '求职面试', 'Prepare for professional job interviews', '准备专业求职面试', '💼', 'advanced', 'business', 4, true),
('Doctor Visit', '看医生', 'Describe symptoms and understand medical advice', '描述症状和理解医疗建议', '🏥', 'intermediate', 'daily', 5, true),
('Shopping', '购物', 'Shop for clothes, electronics, and negotiate prices', '购买衣服、电子产品和讨价还价', '🛍️', 'beginner', 'daily', 6, true),
('Business Meeting', '商务会议', 'Participate in professional meetings and presentations', '参加专业会议和演示', '📊', 'advanced', 'business', 7, true),
('Asking for Directions', '问路', 'Ask for and give directions in various situations', '在各种情况下问路和指路', '🗺️', 'beginner', 'travel', 8, true),
('Phone Call', '电话沟通', 'Handle phone learn_conversations professionally', '专业处理电话交流', '📞', 'intermediate', 'business', 9, true),
('Coffee Shop', '咖啡店', 'Order drinks and have casual learn_conversations', '点饮料和进行日常交谈', '☕', 'beginner', 'daily', 10, true);

-- ============================================================================
-- SCENE DIALOGUES
-- ============================================================================

-- Airport Check-in dialogues
INSERT INTO asset_dialogues (scene_id, title_en, title_zh, description_en, description_zh, total_turns, estimated_duration_seconds, difficulty_level) VALUES
((SELECT id FROM scenes WHERE name_en = 'Airport Check-in'), 'Basic Check-in', '基础值机', 'A simple check-in conversation', '简单的值机对话', 8, 120, 'beginner'),
((SELECT id FROM scenes WHERE name_en = 'Airport Check-in'), 'Overweight Luggage', '行李超重', 'Handling overweight baggage situation', '处理行李超重的情况', 10, 180, 'intermediate'),
((SELECT id FROM scenes WHERE name_en = 'Airport Check-in'), 'Seat Upgrade Request', '升舱请求', 'Requesting a seat upgrade', '请求升舱', 8, 150, 'intermediate');

-- Hotel Reservation dialogues
INSERT INTO asset_dialogues (scene_id, title_en, title_zh, description_en, description_zh, total_turns, estimated_duration_seconds, difficulty_level) VALUES
((SELECT id FROM scenes WHERE name_en = 'Hotel Reservation'), 'Making a Reservation', '预订房间', 'Booking a hotel room', '预订酒店房间', 8, 120, 'beginner'),
((SELECT id FROM scenes WHERE name_en = 'Hotel Reservation'), 'Checking In', '办理入住', 'Hotel check-in process', '酒店入住流程', 6, 90, 'beginner'),
((SELECT id FROM scenes WHERE name_en = 'Hotel Reservation'), 'Room Complaint', '房间投诉', 'Handling issues with the room', '处理房间问题', 10, 180, 'intermediate');

-- Restaurant dialogues
INSERT INTO asset_dialogues (scene_id, title_en, title_zh, description_en, description_zh, total_turns, estimated_duration_seconds, difficulty_level) VALUES
((SELECT id FROM scenes WHERE name_en = 'Restaurant Ordering'), 'Ordering a Meal', '点餐', 'Basic restaurant ordering', '基础餐厅点餐', 8, 120, 'beginner'),
((SELECT id FROM scenes WHERE name_en = 'Restaurant Ordering'), 'Special Dietary Needs', '特殊饮食需求', 'Explaining allergies and preferences', '解释过敏和偏好', 10, 150, 'intermediate'),
((SELECT id FROM scenes WHERE name_en = 'Restaurant Ordering'), 'Paying the Bill', '结账', 'Asking for the check and paying', '要账单和付款', 6, 90, 'beginner');

-- ============================================================================
-- DIALOGUE TURNS
-- ============================================================================

-- Basic Check-in dialogue turns
INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, speaker_name, content_en, content_zh, phonetic_transcription, asset_phrases, notes) VALUES
((SELECT id FROM asset_dialogues WHERE title_en = 'Basic Check-in'), 1, 'npc', 'Agent', 'Good morning! May I see your passport and ticket, please?', '早上好！请出示您的护照和机票好吗？', '/ɡʊd ˈmɔːrnɪŋ meɪ aɪ siː jɔːr ˈpæspɔːrt ænd ˈtɪkɪt pliːz/', '["May I see", "please"]', 'Polite request format'),
((SELECT id FROM asset_dialogues WHERE title_en = 'Basic Check-in'), 2, 'user', 'Traveler', 'Here you go. I have a flight to New York.', '给您。我有一班飞往纽约的航班。', '/hɪr juː ɡoʊ aɪ hæv ə flaɪt tuː nuː jɔːrk/', '["Here you go", "flight to"]', NULL),
((SELECT id FROM asset_dialogues WHERE title_en = 'Basic Check-in'), 3, 'npc', 'Agent', 'Would you like a window or aisle seat?', '您想要靠窗还是靠过道的座位？', '/wʊd juː laɪk ə ˈwɪndoʊ ɔːr aɪl siːt/', '["Would you like", "window or aisle"]', 'Common seat preference question'),
((SELECT id FROM asset_dialogues WHERE title_en = 'Basic Check-in'), 4, 'user', 'Traveler', 'A window seat, please.', '请给我靠窗的座位。', '/ə ˈwɪndoʊ siːt pliːz/', '["please"]', NULL),
((SELECT id FROM asset_dialogues WHERE title_en = 'Basic Check-in'), 5, 'npc', 'Agent', 'How many bags are you checking in today?', '您今天要托运几件行李？', '/haʊ ˈmeni bæɡz ɑːr juː ˈtʃekɪŋ ɪn təˈdeɪ/', '["How many", "checking in"]', NULL),
((SELECT id FROM asset_dialogues WHERE title_en = 'Basic Check-in'), 6, 'user', 'Traveler', 'Just one suitcase.', '只有一个行李箱。', '/dʒʌst wʌn ˈsuːtkeɪs/', '["Just one"]', NULL),
((SELECT id FROM asset_dialogues WHERE title_en = 'Basic Check-in'), 7, 'npc', 'Agent', 'Here is your boarding pass. Gate 15, boarding at 10:30.', '这是您的登机牌。15号登机口，10:30开始登机。', '/hɪr ɪz jɔːr ˈbɔːrdɪŋ pæs ɡeɪt fɪfˈtiːn ˈbɔːrdɪŋ æt ten ˈθɜːrti/', '["boarding pass", "Gate", "boarding at"]', NULL),
((SELECT id FROM asset_dialogues WHERE title_en = 'Basic Check-in'), 8, 'user', 'Traveler', 'Thank you very much!', '非常感谢！', '/θæŋk juː ˈveri mʌtʃ/', '["Thank you very much"]', NULL);

-- Ordering a Meal dialogue turns
INSERT INTO asset_dialogue_turns (dialogue_id, turn_number, speaker_role, speaker_name, content_en, content_zh, phonetic_transcription, asset_phrases, notes) VALUES
((SELECT id FROM asset_dialogues WHERE title_en = 'Ordering a Meal'), 1, 'npc', 'Waiter', 'Welcome! Here is your menu. Can I get you something to drink?', '欢迎光临！这是您的菜单。要点些喝的吗？', NULL, '["Can I get you", "something to drink"]', NULL),
((SELECT id FROM asset_dialogues WHERE title_en = 'Ordering a Meal'), 2, 'user', 'Customer', 'Yes, I will have a glass of water, please.', '好的，请给我一杯水。', NULL, '["I will have", "a glass of"]', NULL),
((SELECT id FROM asset_dialogues WHERE title_en = 'Ordering a Meal'), 3, 'npc', 'Waiter', 'Are you ready to order, or do you need a few more minutes?', '您准备好点餐了吗，还是需要再看一会儿？', NULL, '["ready to order", "a few more minutes"]', NULL),
((SELECT id FROM asset_dialogues WHERE title_en = 'Ordering a Meal'), 4, 'user', 'Customer', 'I am ready. I would like the grilled salmon, please.', '我准备好了。我想要烤三文鱼。', NULL, '["I would like", "grilled salmon"]', NULL),
((SELECT id FROM asset_dialogues WHERE title_en = 'Ordering a Meal'), 5, 'npc', 'Waiter', 'Excellent choice! Would you like any sides with that?', '很好的选择！您要配菜吗？', NULL, '["Excellent choice", "any sides"]', NULL),
((SELECT id FROM asset_dialogues WHERE title_en = 'Ordering a Meal'), 6, 'user', 'Customer', 'Yes, I will have the mashed potatoes and a salad.', '好的，我要土豆泥和一份沙拉。', NULL, '["I will have", "mashed potatoes"]', NULL),
((SELECT id FROM asset_dialogues WHERE title_en = 'Ordering a Meal'), 7, 'npc', 'Waiter', 'Perfect. Your order will be ready in about 15 minutes.', '好的。您的菜大约15分钟后上。', NULL, '["will be ready", "in about"]', NULL),
((SELECT id FROM asset_dialogues WHERE title_en = 'Ordering a Meal'), 8, 'user', 'Customer', 'That sounds great, thank you!', '太好了，谢谢！', NULL, '["That sounds great"]', NULL);

-- ============================================================================
-- CLASSIC DIALOGUE SOURCES
-- ============================================================================

INSERT INTO asset_classic_sources (source_type, title, year, description_en, description_zh, thumbnail_url, imdb_id, difficulty_level) VALUES
('movie', 'The Shawshank Redemption', 1994, 'A powerful drama about hope and perseverance', '一部关于希望和坚持的强大剧情片', NULL, 'tt0111161', 'intermediate'),
('movie', 'Forrest Gump', 1994, 'Life lessons through the eyes of a simple man', '通过一个单纯男人的视角讲述人生', NULL, 'tt0109830', 'beginner'),
('movie', 'The Social Network', 2010, 'The story of Facebook creation', 'Facebook创建的故事', NULL, 'tt1285016', 'advanced'),
('tv_show', 'Friends', 1994, 'Classic sitcom about six friends in New York', '关于六个纽约朋友的经典情景喜剧', NULL, 'tt0108778', 'beginner'),
('tv_show', 'The Office', 2005, 'Mockumentary about office life', '关于办公室生活的伪纪录片', NULL, 'tt0386676', 'intermediate'),
('tv_show', 'Breaking Bad', 2008, 'Drama about a chemistry teacher turned criminal', '关于化学老师变成罪犯的剧情片', NULL, 'tt0903747', 'advanced'),
('ted_talk', 'The Power of Vulnerability', 2010, 'Brené Brown on human connection', 'Brené Brown谈人际联系', NULL, NULL, 'intermediate'),
('ted_talk', 'How Great Leaders Inspire Action', 2009, 'Simon Sinek on leadership', 'Simon Sinek谈领导力', NULL, NULL, 'intermediate'),
('ted_talk', 'Your Body Language May Shape Who You Are', 2012, 'Amy Cuddy on body language', 'Amy Cuddy谈肢体语言', NULL, NULL, 'beginner');

-- ============================================================================
-- CLASSIC DIALOGUE CLIPS
-- ============================================================================

INSERT INTO asset_classic_clips (source_id, clip_title_en, clip_title_zh, start_time_seconds, end_time_seconds, transcript_en, transcript_zh, key_vocabulary, cultural_notes, grammar_points, difficulty_vocab, difficulty_speed, difficulty_slang, popularity_score) VALUES
((SELECT id FROM asset_classic_sources WHERE title = 'Friends'), 'How You Doin?', '你好吗？', 0, 30,
'Joey: How you doin''?
Rachel: I''m doing great, thanks for asking!
Joey: You know, that''s my line.',
'Joey: 你好吗？
Rachel: 我很好，谢谢关心！
Joey: 你知道的，那是我的台词。',
'["How you doin''", "my line"]',
'Joey''s signature pickup line became a cultural phenomenon in the 90s',
'["informal greeting", "present continuous"]',
2, 2, 3, 95),

((SELECT id FROM asset_classic_sources WHERE title = 'The Social Network'), 'A Million Dollars', '一百万美元', 0, 45,
'Sean Parker: A million dollars isn''t cool. You know what''s cool? A billion dollars.
Eduardo: Is he for real?
Mark: He''s for real.',
'Sean Parker: 一百万美元不酷。你知道什么才酷吗？十亿美元。
Eduardo: 他是认真的吗？
Mark: 他是认真的。',
'["cool", "billion", "for real"]',
'Reflects the ambitious mindset of Silicon Valley entrepreneurs',
'["rhetorical question", "emphasis"]',
3, 3, 2, 88),

((SELECT id FROM asset_classic_sources WHERE title = 'Forrest Gump'), 'Life is Like a Box of Chocolates', '人生就像一盒巧克力', 0, 30,
'Forrest: My mama always said, life was like a box of chocolates. You never know what you''re gonna get.',
'Forrest: 我妈妈总是说，人生就像一盒巧克力。你永远不知道你会得到什么。',
'["life", "box of chocolates", "gonna"]',
'One of the most famous movie quotes in American cinema',
'["simile", "gonna = going to"]',
2, 2, 2, 98),

((SELECT id FROM asset_classic_sources WHERE title = 'The Office'), 'That is What She Said', '她就是这么说的', 0, 20,
'Michael: That''s what she said!
Jim: Michael, please.
Michael: I couldn''t resist.',
'Michael: 她就是这么说的！
Jim: Michael，别这样。
Michael: 我忍不住。',
'["that''s what she said", "couldn''t resist"]',
'A classic double entendre joke popularized by the show',
'["past tense", "modal verbs"]',
2, 2, 4, 90);

-- ============================================================================
-- READING EXERCISES
-- ============================================================================

INSERT INTO asset_read_exercises (title_en, title_zh, description_en, description_zh, difficulty_level, exercise_type) VALUES
('Daily Greetings', '日常问候', 'Practice common greeting phrases', '练习常见问候短语', 'beginner', 'sentence'),
('Business Introductions', '商务介绍', 'Professional introduction phrases', '专业介绍短语', 'intermediate', 'sentence'),
('Travel Conversations', '旅行对话', 'Useful phrases for traveling', '旅行中有用的短语', 'beginner', 'dialogue'),
('Tongue Twisters', '绕口令', 'Fun pronunciation practice', '有趣的发音练习', 'intermediate', 'tongue_twister'),
('News Reading', '新闻阅读', 'Practice reading news articles', '练习阅读新闻文章', 'advanced', 'paragraph');

-- ============================================================================
-- READING SENTENCES
-- ============================================================================

-- Daily Greetings sentences
INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, phonetic_transcription, focus_sounds, common_mistakes) VALUES
((SELECT id FROM asset_read_exercises WHERE title_en = 'Daily Greetings'), 1, 'Good morning! How are you doing today?', '早上好！你今天怎么样？', '/ɡʊd ˈmɔːrnɪŋ haʊ ɑːr juː ˈduːɪŋ təˈdeɪ/', '["morning", "doing"]', '["mor-ning not mourning", "today stress on second syllable"]'),
((SELECT id FROM asset_read_exercises WHERE title_en = 'Daily Greetings'), 2, 'Nice to meet you. My name is John.', '很高兴认识你。我叫John。', '/naɪs tuː miːt juː maɪ neɪm ɪz dʒɑːn/', '["meet", "name"]', '["meet vs mit", "name long a sound"]'),
((SELECT id FROM asset_read_exercises WHERE title_en = 'Daily Greetings'), 3, 'How was your weekend?', '你周末过得怎么样？', '/haʊ wɒz jɔːr ˈwiːkend/', '["was", "weekend"]', '["weekend stress on first syllable"]'),
((SELECT id FROM asset_read_exercises WHERE title_en = 'Daily Greetings'), 4, 'See you later! Have a great day!', '回头见！祝你有美好的一天！', '/siː juː ˈleɪtər hæv ə ɡreɪt deɪ/', '["later", "great"]', '["later vs letter", "great long a"]'),
((SELECT id FROM asset_read_exercises WHERE title_en = 'Daily Greetings'), 5, 'Thank you so much for your help.', '非常感谢你的帮助。', '/θæŋk juː soʊ mʌtʃ fɔːr jɔːr help/', '["thank", "much"]', '["th sound", "much not match"]');

-- Business Introductions sentences
INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, phonetic_transcription, focus_sounds, common_mistakes) VALUES
((SELECT id FROM asset_read_exercises WHERE title_en = 'Business Introductions'), 1, 'Allow me to introduce myself. I am the marketing director.', '请允许我自我介绍。我是市场总监。', NULL, '["introduce", "director"]', '["introduce stress on third syllable"]'),
((SELECT id FROM asset_read_exercises WHERE title_en = 'Business Introductions'), 2, 'It is a pleasure to meet you. I have heard great things about your company.', '很高兴认识你。我听说过很多关于贵公司的好消息。', NULL, '["pleasure", "company"]', '["pleasure zh sound", "company stress on first syllable"]'),
((SELECT id FROM asset_read_exercises WHERE title_en = 'Business Introductions'), 3, 'I am responsible for overseeing the sales department.', '我负责监管销售部门。', NULL, '["responsible", "overseeing"]', '["responsible stress pattern"]'),
((SELECT id FROM asset_read_exercises WHERE title_en = 'Business Introductions'), 4, 'Our company specializes in software development.', '我们公司专门从事软件开发。', NULL, '["specializes", "development"]', '["specializes s vs z sound"]'),
((SELECT id FROM asset_read_exercises WHERE title_en = 'Business Introductions'), 5, 'I would be happy to schedule a follow-up meeting.', '我很乐意安排一次后续会议。', NULL, '["schedule", "follow-up"]', '["schedule British vs American pronunciation"]');

-- Tongue Twisters
INSERT INTO asset_read_sentences (exercise_id, sentence_order, content_en, content_zh, phonetic_transcription, focus_sounds, common_mistakes) VALUES
((SELECT id FROM asset_read_exercises WHERE title_en = 'Tongue Twisters'), 1, 'She sells seashells by the seashore.', '她在海边卖贝壳。', '/ʃiː selz ˈsiːʃelz baɪ ðə ˈsiːʃɔːr/', '["sh", "s"]', '["distinguish sh and s sounds"]'),
((SELECT id FROM asset_read_exercises WHERE title_en = 'Tongue Twisters'), 2, 'Peter Piper picked a peck of pickled peppers.', 'Peter Piper摘了一配克腌辣椒。', '/ˈpiːtər ˈpaɪpər pɪkt ə pek əv ˈpɪkld ˈpepərz/', '["p"]', '["p sound aspiration"]'),
((SELECT id FROM asset_read_exercises WHERE title_en = 'Tongue Twisters'), 3, 'How much wood would a woodchuck chuck if a woodchuck could chuck wood?', '如果土拨鼠能扔木头，它能扔多少木头？', NULL, '["w", "ch"]', '["w vs v", "ch sound"]'),
((SELECT id FROM asset_read_exercises WHERE title_en = 'Tongue Twisters'), 4, 'Red lorry, yellow lorry, red lorry, yellow lorry.', '红卡车，黄卡车，红卡车，黄卡车。', NULL, '["r", "l"]', '["r vs l distinction"]'),
((SELECT id FROM asset_read_exercises WHERE title_en = 'Tongue Twisters'), 5, 'The thirty-three thieves thought that they thrilled the throne throughout Thursday.', '三十三个小偷认为他们在周四一整天都让王座兴奋不已。', NULL, '["th"]', '["th voiced vs unvoiced"]');

-- ============================================================================
-- KEY PHRASES
-- ============================================================================

INSERT INTO asset_phrases (phrase_en, phrase_zh, phonetic_transcription, usage_context, example_sentence_en, example_sentence_zh, category, formality_level, frequency_score) VALUES
('How are you doing?', '你好吗？', '/haʊ ɑːr juː ˈduːɪŋ/', 'Casual greeting', 'Hey John, how are you doing?', '嘿John，你好吗？', 'greeting', 'casual', 95),
('Nice to meet you', '很高兴认识你', '/naɪs tuː miːt juː/', 'First meeting', 'Nice to meet you. I am Sarah.', '很高兴认识你。我是Sarah。', 'greeting', 'neutral', 98),
('Could you please...', '你能...吗？', '/kʊd juː pliːz/', 'Polite request', 'Could you please pass me the salt?', '你能把盐递给我吗？', 'request', 'formal', 90),
('I would like to...', '我想要...', '/aɪ wʊd laɪk tuː/', 'Expressing desire', 'I would like to order the steak.', '我想点牛排。', 'request', 'formal', 88),
('In my opinion...', '在我看来...', '/ɪn maɪ əˈpɪnjən/', 'Expressing opinion', 'In my opinion, we should wait.', '在我看来，我们应该等待。', 'opinion', 'neutral', 75),
('I completely agree', '我完全同意', '/aɪ kəmˈpliːtli əˈɡriː/', 'Agreement', 'I completely agree with your point.', '我完全同意你的观点。', 'opinion', 'formal', 70),
('Excuse me', '打扰一下', '/ɪkˈskjuːz miː/', 'Getting attention', 'Excuse me, where is the bathroom?', '打扰一下，洗手间在哪里？', 'request', 'neutral', 95),
('I am sorry to hear that', '听到这个消息我很抱歉', '/aɪ æm ˈsɒri tuː hɪr ðæt/', 'Expressing sympathy', 'I am sorry to hear that you are sick.', '听说你生病了，我很抱歉。', 'opinion', 'neutral', 65),
('Would you mind...', '你介意...吗？', '/wʊd juː maɪnd/', 'Polite request', 'Would you mind closing the window?', '你介意关上窗户吗？', 'request', 'formal', 80),
('As far as I know', '据我所知', '/æz fɑːr æz aɪ noʊ/', 'Expressing uncertainty', 'As far as I know, the meeting is at 3 PM.', '据我所知，会议在下午3点。', 'opinion', 'neutral', 72);
