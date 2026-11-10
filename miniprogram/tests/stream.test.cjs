const { test } = require("node:test");
const assert = require("node:assert/strict");
const { setup } = require("./helpers.cjs");
function frame(event, data, newline = "\n") {
  return Buffer.from(
    `data: ${JSON.stringify({ event, data })}${newline}${newline}`,
  );
}
test("SSE 逐字节分片保留中文、emoji、转义字符和 CRLF", () => {
  setup();
  const { createParser } = require("../services/stream");
  const values = [];
  const parser = createParser((value) => values.push(value));
  const bytes = Buffer.concat([
    Buffer.from(": keep-alive\r\n\r\n"),
    frame(
      "delta",
      { reply_en: 'Hello "friend"\n', reply_zh: "你好🌍" },
      "\r\n",
    ),
    frame("done", { id: 1 }),
  ]);
  for (const byte of bytes) parser.push(Uint8Array.of(byte).buffer);
  parser.finish();
  assert.equal(values.length, 2);
  assert.equal(values[0].data.reply_zh, "你好🌍");
  assert.equal(values[0].data.reply_en, 'Hello "friend"\n');
});
test("SSE 缺少 done、非法 JSON、超长和坏 UTF-8 会报错", () => {
  setup();
  const { createParser } = require("../services/stream");
  let parser = createParser(() => {});
  parser.push(frame("delta", {}));
  assert.throws(() => parser.finish(), /中断/);
  parser = createParser(() => {});
  assert.throws(() => parser.push(Buffer.from("data: broken\n\n")), /格式/);
  parser = createParser(() => {});
  assert.throws(() => parser.push(new Uint8Array(2 * 1024 * 1024 + 1)), /长度/);
  parser = createParser(() => {});
  assert.throws(() => parser.push(Uint8Array.from([255, 10])), /编码/);
});
test("流式发送仅发送一次 POST，done 结束，不轮询或重发消息", async () => {
  const h = setup();
  h.login();
  const { send } = require("../services/stream");
  const events = [];
  const connection = send(42, { type: "text", message: "你好" }, (e) =>
    events.push(e),
  );
  const call = h.pending[0];
  assert.equal(call.enableChunked, true);
  assert.equal(call.header.Authorization, "Bearer token-1");
  call.chunk({ data: frame("accepted", { user_turn: { id: 1 } }) });
  call.chunk({ data: frame("delta", { reply_en: "Hello" }) });
  call.chunk({ data: frame("done", { ai_turn: { id: 2 } }) });
  await connection.promise;
  assert.equal(events.length, 3);
  assert.equal(h.pending.length, 1);
});
test("流式错误保留服务端信息，并支持不提供 chunk 的完整响应", async () => {
  const h = setup();
  const { send } = require("../services/stream");
  let connection = send(1, {}, () => {});
  h.pending
    .shift()
    .chunk({ data: frame("error", { message: "提供方调用失败" }) });
  await assert.rejects(connection.promise, /提供方/);
  connection = send(1, {}, () => {});
  h.respond(frame("done", {}));
  await connection.promise;
});
test("取消流式请求会中止连接；账户切换不再分发旧回复", async () => {
  const h = setup();
  h.login();
  const { send } = require("../services/stream");
  let connection = send(1, {}, () => {});
  connection.abort();
  await assert.rejects(connection.promise, /离开/);
  assert.equal(h.pending.shift().aborted, true);
  let count = 0;
  connection = send(1, {}, () => count++);
  h.login(2);
  h.pending.shift().chunk({ data: frame("done", {}) });
  await assert.rejects(connection.promise, /账户/);
  assert.equal(count, 0);
});
