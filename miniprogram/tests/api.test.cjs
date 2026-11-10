const { test } = require("node:test");
const assert = require("node:assert/strict");
const { setup } = require("./helpers.cjs");
test("登录令牌通过 header 发送，成功响应保留原有 API 结构", async () => {
  const h = setup();
  h.login();
  const promise = h.api.request("/api/me");
  assert.equal(h.pending[0].header.Authorization, "Bearer token-1");
  assert.equal(h.pending[0].url, "http://127.0.0.1:8119/api/me");
  h.respond({ id: 1, handle: "user_1" });
  assert.equal((await promise).id, 1);
});
test("401 清理当前登录态，402 保留登录与服务端说明", async () => {
  const h = setup();
  h.login();
  let promise = h.api.request("/api/learn/chats");
  h.respond({ error: { brief: "先配置个人 AI" } }, 402);
  await assert.rejects(promise, /先配置个人 AI/);
  assert.equal(h.api.token(), "token-1");
  promise = h.api.request("/api/me");
  h.respond({}, 401);
  await assert.rejects(promise, /登录/);
  assert.equal(h.api.token(), null);
  assert.equal(h.storage.has(h.api.storageKey("session")), false);
});
test("旧请求的 401 不登出新账户；旧成功响应不泄漏给新账户", async () => {
  const h = setup();
  h.login();
  let promise = h.api.request("/api/me");
  h.login(2);
  h.respond({}, 401);
  await assert.rejects(promise);
  assert.equal(h.api.token(), "token-2");
  promise = h.api.request("/api/me");
  h.login(3);
  h.respond({ id: 2 });
  await assert.rejects(promise, /账户状态已变化/);
});
test("拒绝跨域 API、反斜杠与控制字符", () => {
  const h = setup();
  for (const path of [
    "https://evil.test/api/me",
    "//evil.test",
    "/api/\\evil",
    "/api/me\n",
  ])
    assert.throws(() => h.api.url(path));
});
test("体验和发布环境未配置时不能回退到本机或开发地址", async () => {
  const h = setup();
  h.wx.getAccountInfoSync = () => ({ miniProgram: { envVersion: "release" } });
  h.api.initialize();
  await assert.rejects(h.api.request("/api/me"), /config.js/);
  assert.equal(h.pending.length, 0);
});
test("列表结构兼容网站直接数组、分页和词典响应", () => {
  const h = setup();
  for (const value of [[1], { items: [1] }, { words: [1] }, { contexts: [1] }])
    assert.deepEqual(h.api.items(value), [1]);
});
