const { test } = require("node:test");
const assert = require("node:assert/strict");
const { setup } = require("./helpers.cjs");
test("查词按服务端收藏 ID 数组、favorites 单词数组和 added 字段更新", async () => {
  const h = setup();
  h.login();
  const page = h.page("dictionary");
  page.data.word = "Ice cream";
  const promise = page.lookup();
  h.respond({ word: { id: 42, word: "Ice cream" }, definitions: [] });
  await h.flush();
  h.respond({});
  await h.flush();
  h.respond([42]);
  h.respond({ favorites: ["ice cream"] });
  await h.flush();
  h.respond([]);
  await promise;
  assert.equal(page.data.favorite, true);
  assert.equal(page.data.inVocabulary, true);
  page.vocabulary();
  h.respond({ added: false });
  await h.flush();
  assert.equal(page.data.inVocabulary, false);
  page.onUnload();
});
test("游客查词历史仅保存本机，并限制记录数量", async () => {
  const h = setup();
  const page = h.page("dictionary");
  page.data.word = "hello";
  const promise = page.lookup();
  h.respond({ word: { id: 1, word: "hello" } });
  await promise;
  assert.deepEqual(h.storage.get(h.api.storageKey("history")), ["hello"]);
  assert.equal(h.pending.length, 0);
  page.onUnload();
});
test("翻译沿用服务端支持的 original 语气，译文收藏使用同一语气", async () => {
  const h = setup();
  h.login();
  const page = h.page("dictionary");
  page.data.text = "Hello";
  page.translate();
  assert.equal(h.pending[0].data.options.tone, "original");
  h.respond({ translated: "你好", detected_source: "en" });
  await h.flush();
  page.favoriteTranslation();
  assert.equal(h.pending[0].data.tone, "original");
  assert.equal(h.pending[0].data.translated_text, "你好");
  h.respond({ is_favorited: true });
  await h.flush();
  page.onUnload();
});
test("拼写练习使用实际题型、自动核对、保存后才允许前进", async () => {
  const h = setup();
  h.login();
  const page = h.page("review");
  page.data.current = {
    source: "vocabulary",
    source_id: 9,
    review_type: "spelling",
    word: "Ice cream",
  };
  page.data.items = [page.data.current];
  page._started = Date.now();
  page.data.answer = "  ICE   cream ";
  page.check();
  assert.equal(page.data.correct, true);
  page.next();
  assert.equal(page.data.index, 0);
  page.submit({ currentTarget: { dataset: {} } });
  assert.equal(h.pending[0].data.result, "correct");
  assert.equal(h.pending[0].data.review_type, "spelling");
  assert.equal(h.pending[0].data.source_id, 9);
  h.respond({ success: true });
  await h.flush();
  assert.equal(page.data.completed, 1);
  page.next();
  assert.equal(page.data.done, true);
  page.onUnload();
});
test("复习保存失败保留答案和结果，不能前进或虚增完成数量", async () => {
  const h = setup();
  h.login();
  const page = h.page("review");
  page.data.current = {
    source: "issue_word",
    source_id: 2,
    review_type: "pronunciation",
  };
  page.data.evaluation = { overall_score: 66 };
  page._started = Date.now();
  page.submit({ currentTarget: { dataset: {} } });
  assert.equal(h.pending[0].data.result, "partial");
  h.respond({}, 503);
  await h.flush();
  assert.equal(page.data.saved, false);
  assert.equal(page.data.completed, 0);
  assert.equal(page.data.evaluation.overall_score, 66);
  page.onUnload();
});
test("AI 模型配置使用目录默认值，编辑留空密钥时不提交掩码", async () => {
  const h = setup();
  h.login();
  const page = h.page("providers");
  page.data.catalog = [
    {
      id: "zhipu",
      label: "智谱",
      chat_models: ["glm-4-flash"],
      tts_models: ["glm-tts"],
      asr_models: ["glm-asr-2512"],
      voice: "tongtong",
      regions: ["cn"],
    },
  ];
  page.defaults(0);
  assert.equal(page.data.models[2].model, "glm-asr-2512");
  page.data.editingId = 3;
  page.load = async () => {};
  page.save();
  const body = h.pending[0].data;
  assert.deepEqual(body.activate_for, ["chat", "tts", "asr"]);
  assert.equal("api_key" in body.settings, false);
  h.respond({ id: 3 });
  await h.flush();
  assert.equal(page.data.apiKey, "");
  page.onUnload();
});
test("跟读评估结果与唯一 practice_id 保存，保存失败后可单独重试", async () => {
  const h = setup();
  h.login();
  const page = h.page("read");
  page.data.current = { id: 8 };
  page.data.evaluation = {
    overall_score: 91,
    pronunciation_score: 92,
    fluency_score: 88,
    intonation_score: 90,
  };
  page.data.practiceId = "wx-record-1";
  page.saveAgain();
  assert.equal(h.pending[0].data.practice_id, "wx-record-1");
  assert.equal(h.pending[0].data.sentence_id, 8);
  h.respond({}, 503);
  await h.flush();
  assert.equal(page.data.saved, false);
  assert.equal(page.data.evaluation.overall_score, 91);
  page.saveAgain();
  assert.equal(h.pending[0].data.practice_id, "wx-record-1");
  h.respond({ success: true });
  await h.flush();
  assert.equal(page.data.saved, true);
  page.onUnload();
});
test("隐藏的 tab 在切换账户后清空旧账户私有内容", () => {
  const h = setup();
  h.login();
  const page = h.page("me");
  page.data.summary = { learning_days: 123 };
  page.onHide();
  h.api.logout();
  page.onShow();
  assert.equal(page.data.summary, null);
  assert.equal(page.data.user, null);
  page.onUnload();
});
test("登录回来保留复习类型和场景路由，同时清空私有数据", async () => {
  const h = setup();
  const review = h.page("review", { type: "spelling" });
  review.onHide();
  h.login();
  review.onShow();
  assert.equal(review.data.type, "spelling");
  assert.equal(review.data.name, "拼写练习");
  h.respond({ recorded: false });
  h.respond({ items: [] });
  await h.flush();
  review.onUnload();
  const stage = h.page("stage", { id: "42" });
  stage.onHide();
  h.api.logout();
  stage.onShow();
  assert.equal(stage.data.id, 42);
  assert.equal(stage.data.user, null);
  h.respond({ id: 42 });
  h.respond([]);
  await h.flush();
  stage.onUnload();
});
test("离开页面后迟到的查询结果不会覆盖页面数据", async () => {
  const h = setup();
  const page = h.page("stages");
  page.load();
  page.onHide();
  h.respond([{ id: 9 }]);
  await h.flush();
  assert.deepEqual(page.data.stages, []);
  page.onUnload();
});
test("恢复生成中的对话有界轮询，离开页面清理定时器", async () => {
  const h = setup();
  h.login();
  const page = h.page("chat");
  page.data.chatId = 3;
  page.data.turns = [{ id: 5, status: "processing" }];
  page.recover();
  assert.equal(page.data.recovering, true);
  assert.equal(page._timers.size, 1);
  page.onHide();
  assert.equal(page._timers.size, 0);
  assert.equal(page.data.recovering, false);
});

test("复习切后台保留当前题目、答案与提交进度", async () => {
  const h = setup(); h.login(); const page = h.page("review");
  page.onShow();
  // onShow emits the heartbeat before loading the review items.
  h.respond({ recorded: false });
  h.respond([{source:"vocabulary",source_id:1,word:"hello",review_type:"spelling"}]);
  await h.flush();
  page.setData({answer:"hello",index:4,completed:4,saved:true});
  page.onHide(); page.onShow();
  assert.equal(page.data.answer,"hello"); assert.equal(page.data.index,4);
  assert.equal(page.data.completed,4); assert.equal(page.data.saved,true);
  assert(!h.pending.some(r=>r.url.includes("review/items")));
  page.onUnload();
});
