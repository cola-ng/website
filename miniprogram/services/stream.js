const api = require("./api");
// 在字节层按行拆帧，只有完整 UTF-8 行才解码；不依赖小程序缺少的 TextDecoder。
function utf8(bytes) {
  let encoded = "";
  for (const byte of bytes) encoded += "%" + byte.toString(16).padStart(2, "0");
  try {
    return decodeURIComponent(encoded);
  } catch (_) {
    throw new Error("回复编码无效");
  }
}
function createParser(onEvent) {
  let line = [],
    data = [],
    size = 0,
    closed = false;
  function newline() {
    if (line[line.length - 1] === 13) line.pop();
    const text = utf8(line);
    line = [];
    if (!text) {
      if (data.length) {
        let frame;
        try {
          frame = JSON.parse(data.join("\n"));
        } catch (_) {
          throw new Error("回复格式无效");
        }
        data = [];
        if (frame.event === "done" || frame.event === "error") closed = true;
        onEvent(frame);
      }
    } else if (text.startsWith("data:"))
      data.push(text.slice(5).replace(/^ /, ""));
  }
  return {
    push(buffer) {
      if (closed) return;
      const bytes = new Uint8Array(buffer);
      size += bytes.length;
      if (size > 2 * 1024 * 1024) throw new Error("回复超过长度限制");
      for (const byte of bytes) {
        if (byte === 10) newline();
        else line.push(byte);
        if (closed) break;
      }
    },
    finish() {
      if (!closed) throw new Error("回复连接中断，请刷新对话查看已保存内容");
    },
  };
}
function send(chat, body, onEvent) {
  let task,
    settled = false,
    received = false,
    resolvePromise,
    rejectPromise;
  const epoch = api.identity(),
    sentToken = api.token();
  const promise = new Promise((resolve, reject) => {
    resolvePromise = resolve;
    rejectPromise = reject;
  });
  function settle(error) {
    if (settled) return;
    settled = true;
    if (error) rejectPromise(error);
    else resolvePromise();
  }
  const parser = createParser((frame) => {
    if (api.identity() !== epoch) return settle(new Error("账户状态已变化"));
    onEvent(frame);
    if (frame.event === "done") settle();
    if (frame.event === "error")
      settle(new Error(frame.data.message || "回复失败"));
  });
  try {
    task = wx.request({
      url: api.url(`/api/learn/chats/${chat}/send/stream`),
      method: "POST",
      data: body,
      header: Object.assign(api.headers(sentToken), {
        Accept: "text/event-stream",
      }),
      enableChunked: true,
      responseType: "arraybuffer",
      dataType: "text",
      timeout: 120000,
      success(res) {
        if (res.statusCode < 200 || res.statusCode >= 300) {
          let value;
          try {
            value = JSON.parse(utf8(new Uint8Array(res.data)));
          } catch (_) {
            value = null;
          }
          return settle(api.responseError(res.statusCode, value, sentToken));
        }
        try {
          if (!received && res.data) parser.push(res.data);
          parser.finish();
        } catch (e) {
          settle(e);
        }
      },
      fail() {
        settle(new Error("回复连接中断，请刷新后查看已保存内容"));
      },
    });
    task.onChunkReceived((res) => {
      if (settled) return;
      received = true;
      try {
        parser.push(res.data);
      } catch (e) {
        settle(e);
        task.abort();
      }
    });
  } catch (e) {
    settle(e);
  }
  return {
    promise,
    abort() {
      settle(new Error("已离开对话"));
      if (task) task.abort();
    },
  };
}
module.exports = { utf8, createParser, send };
