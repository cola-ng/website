const { test } = require("node:test");
const assert = require("node:assert/strict");
const { setup } = require("./helpers.cjs");
test("跟读录音使用单声道 16k WAV，用户停止后才读取音频", () => {
  const h = setup();
  h.login();
  const page = h.page("read");
  const media = require("../services/media");
  let audio;
  media.record(page, (value) => {
    audio = value;
  });
  assert.equal(page.data.recording, true);
  assert.equal(h.recorder.options.format, "wav");
  assert.equal(h.recorder.options.sampleRate, 16000);
  assert.equal(audio, undefined);
  media.finish(page);
  assert.equal(audio, "UklGRg==");
  assert.equal(page.data.recording, false);
  page.onUnload();
});
test("退出页面取消录音、移除回调，不会自动发送", () => {
  const h = setup();
  h.login();
  const page = h.page("chat");
  const media = require("../services/media");
  let sent = false;
  media.record(page, () => {
    sent = true;
  });
  page.onHide();
  h.recorder.stop();
  assert.equal(sent, false);
  assert.equal(h.recorder.stops.size, 0);
  assert.equal(h.pending.length, 0);
});
test("音频下载带鉴权，不向外部地址发送 token", async () => {
  const h = setup();
  h.login();
  const page = h.page("read");
  const media = require("../services/media");
  const promise = media.play(page, "/api/sentence-audio/read/2/audio");
  assert.equal(h.pending[0].header.Authorization, "Bearer token-1");
  h.pending.shift().success({ statusCode: 200, tempFilePath: "/audio.mp3" });
  await promise;
  assert.ok(h.calls.includes("play"));
  await assert.rejects(media.play(page, "https://evil.test/audio"), /API 地址/);
  page.onUnload();
});
test("离开页面期间完成下载，不自动播放", async () => {
  const h = setup();
  const page = h.page("read");
  const media = require("../services/media");
  const promise = media.play(page, "/api/sentence-audio/read/2/audio");
  page.onHide();
  h.pending.shift().success({ statusCode: 200, tempFilePath: "/audio.mp3" });
  await promise;
  assert.equal(h.calls.includes("play"), false);
});
test("下一条录音开始后，上一条延迟返回的录音文件不会串入新练习", () => {
  const h = setup();
  h.login();
  const page = h.page("read");
  const media = require("../services/media");
  const reads = [];
  const audio = [];
  h.wx.getFileSystemManager = () => ({
    readFile(options) {
      reads.push(options);
    },
  });
  media.record(page, (value) => audio.push("old:" + value));
  media.finish(page);
  media.stop(page);
  media.record(page, (value) => audio.push("new:" + value));
  media.finish(page);
  reads[0].success({ data: "OLD" });
  reads[1].success({ data: "NEW" });
  assert.deepEqual(audio, ["new:NEW"]);
  page.onUnload();
});
test("拒绝隐私授权时不会启动麦克风", () => {
  const h = setup();
  h.login();
  const page = h.page("read");
  const media = require("../services/media");
  h.wx.requirePrivacyAuthorize = (options) => options.fail();
  media.record(page, () => {});
  assert.equal(h.recorder.options, undefined);
  assert.match(page.data.error, /隐私授权/);
  page.onUnload();
});
