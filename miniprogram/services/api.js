const config = require("../config");
let base = "";
let session = null;
let revision = 0;
const listeners = new Set();

function initialize() {
  const env = wx.getAccountInfoSync().miniProgram.envVersion || "develop";
  base = (config[env] || "").replace(/\/+$/, "");
  if (
    base &&
    !/^https:\/\/[^/?#@]+$/.test(base) &&
    !(
      env === "develop" &&
      /^http:\/\/(127\.0\.0\.1|localhost)(:\d+)?$/.test(base)
    )
  ) {
    base = "";
  }
  session = wx.getStorageSync(storageKey("session")) || null;
  revision++;
}
function storageKey(key) {
  return `colang:${base}:${key}`;
}
function token() {
  return session && session.access_token;
}
function user() {
  return session && session.user;
}
function identity() {
  return revision;
}
function baseUrl() {
  return base;
}
function subscribe(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}
function notify() {
  revision++;
  listeners.forEach((fn) => fn(user()));
}
function saveSession(value) {
  if (!value || !value.access_token || !value.user)
    throw new Error("登录响应无效");
  session = { access_token: value.access_token, user: value.user };
  wx.setStorageSync(storageKey("session"), session);
  notify();
}
function updateUser(value) {
  if (!session) return;
  session.user = value;
  wx.setStorageSync(storageKey("session"), session);
}
function logout() {
  session = null;
  wx.removeStorageSync(storageKey("session"));
  notify();
}
function url(path) {
  if (!base) throw new Error("请先在 config.js 配置当前版本的 API 地址");
  if (
    typeof path !== "string" ||
    !path.startsWith("/api/") ||
    /[\\\r\n]/.test(path)
  ) {
    throw new Error("API 地址无效");
  }
  return base + path;
}
function headers(sentToken = token()) {
  return Object.assign(
    { "content-type": "application/json" },
    sentToken ? { Authorization: `Bearer ${sentToken}` } : {},
  );
}
function responseError(status, body, sentToken) {
  if (status === 401 && sentToken && sentToken === token()) logout();
  const message =
    status === 401
      ? "登录已过期或账号密码错误，请重新登录"
      : body &&
        ((body.error && (body.error.brief || body.error.message)) ||
          body.brief ||
          body.message);
  const err = new Error(
    typeof message === "string"
      ? message
      : {
          403: "没有权限执行此操作",
          404: "未找到内容",
          429: "操作太频繁，请稍后重试",
        }[status] || "请求失败，请稍后重试",
  );
  err.status = status;
  return err;
}
function request(path, method = "GET", data) {
  return new Promise((resolve, reject) => {
    const sentToken = token();
    const epoch = identity();
    wx.request({
      url: url(path),
      method,
      data,
      header: headers(sentToken),
      timeout: 120000,
      success(res) {
        if (res.statusCode < 200 || res.statusCode >= 300)
          return reject(responseError(res.statusCode, res.data, sentToken));
        if (epoch !== identity())
          return reject(new Error("账户状态已变化，请重试"));
        resolve(res.data);
      },
      fail() {
        reject(new Error("网络连接失败，请检查网络与服务地址"));
      },
    });
  });
}
function items(value) {
  if (Array.isArray(value)) return value;
  for (const key of [
    "items",
    "words",
    "stages",
    "scripts",
    "subjects",
    "sentences",
    "contexts",
    "providers",
    "achievements",
  ]) {
    if (value && Array.isArray(value[key])) return value[key];
  }
  return [];
}
function requireLogin() {
  if (token()) return true;
  const pages = getCurrentPages();
  if (!pages.length || pages[pages.length - 1].route !== "pages/login/index")
    wx.navigateTo({ url: "/pages/login/index" });
  return false;
}
module.exports = {
  initialize,
  storageKey,
  token,
  user,
  identity,
  subscribe,
  saveSession,
  updateUser,
  logout,
  baseUrl,
  url,
  headers,
  responseError,
  request,
  items,
  requireLogin,
};
