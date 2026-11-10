const path = require("node:path");
const root = path.resolve(__dirname, "..");
function setup() {
  for (const key of Object.keys(require.cache))
    if (
      key.startsWith(root) &&
      !key.includes("node_modules") &&
      !key.includes("tests")
    )
      delete require.cache[key];
  const storage = new Map(),
    calls = [],
    pending = [];
  const wx = {
    getAccountInfoSync: () => ({ miniProgram: { envVersion: "develop" } }),
    getStorageSync: (key) => storage.get(key),
    setStorageSync: (key, value) => storage.set(key, structuredClone(value)),
    removeStorageSync: (key) => storage.delete(key),
    request(options) {
      calls.push(options);
      pending.push(options);
      return {
        onChunkReceived(fn) {
          options.chunk = fn;
        },
        abort() {
          options.aborted = true;
          if (options.fail) options.fail({});
        },
      };
    },
    showModal(options) {
      calls.push(options);
      options.success({ confirm: true, content: "新标题" });
    },
    showToast(options) {
      calls.push(options);
    },
    navigateTo(options) {
      calls.push(options);
    },
    navigateBack() {},
    switchTab(options) {
      calls.push(options);
    },
    authorize(options) {
      options.success();
    },
    openSetting() {},
    getFileSystemManager: () => ({
      readFile(options) {
        options.success({ data: "UklGRg==" });
      },
    }),
    getRecorderManager() {
      return recorder;
    },
    downloadFile(options) {
      calls.push(options);
      pending.push(options);
    },
    createInnerAudioContext() {
      return {
        onError() {},
        play() {
          calls.push("play");
        },
        destroy() {
          calls.push("destroy");
        },
      };
    },
  };
  const recorder = {
    stops: new Set(),
    errors: new Set(),
    start(options) {
      this.options = options;
    },
    stop() {
      this.stops.forEach((fn) =>
        fn({ duration: 1200, tempFilePath: "/recording.wav" }),
      );
    },
    onStop(fn) {
      this.stops.add(fn);
    },
    offStop(fn) {
      this.stops.delete(fn);
    },
    onError(fn) {
      this.errors.add(fn);
    },
    offError(fn) {
      this.errors.delete(fn);
    },
  };
  global.wx = wx;
  global.getCurrentPages = () => [];
  global.getApp = () => ({ globalData: {} });
  const api = require("../services/api");
  api.initialize();
  function login(id = 1) {
    api.saveSession({
      access_token: "token-" + id,
      user: { id, name: "用户" + id, handle: "user_" + id },
    });
  }
  function page(name, query = {}) {
    let spec;
    global.Page = (value) => {
      spec = value;
    };
    const modulePath = path.join(root, "pages", name, "index.js");
    delete require.cache[require.resolve(modulePath)];
    require(modulePath);
    const instance = Object.assign({}, spec, {
      data: structuredClone(spec.data),
      setData(patch) {
        for (const [key, value] of Object.entries(patch)) {
          const keys = key.replace(/\[(\d+)\]/g, ".$1").split(".");
          let target = this.data;
          keys.slice(0, -1).forEach((part) => {
            if (!target[part]) target[part] = {};
            target = target[part];
          });
          target[keys[keys.length - 1]] = value;
        }
      },
    });
    instance.onLoad(query);
    instance._visible = true;
    instance._version = 1;
    instance.data.user = api.user();
    return instance;
  }
  async function flush() {
    await new Promise((resolve) => setImmediate(resolve));
  }
  function respond(value, statusCode = 200) {
    const call = pending.shift();
    if (!call) throw new Error("No pending request");
    call.success({ statusCode, data: value });
    return call;
  }
  return {
    api,
    wx,
    storage,
    calls,
    pending,
    recorder,
    login,
    page,
    flush,
    respond,
  };
}
module.exports = { setup };
