const api = require("./api");
// 所有页面共享可见期、账户切换和计时清理，避免后台记时或迟到请求写入新账户页面。
function definePage(options) {
  const hooks = {};
  ["onLoad", "onShow", "onHide", "onUnload"].forEach((key) => {
    hooks[key] = options[key];
  });
  const page = Object.assign(
    {
      input(e) {
        this.setData({ [e.currentTarget.dataset.field]: e.detail.value });
      },
      open(e) {
        wx.navigateTo({ url: e.currentTarget.dataset.url });
      },
      login() {
        wx.navigateTo({ url: "/pages/login/index" });
      },
      async run(task, key = "busy") {
        if (this.data[key]) return;
        this.setData({ [key]: true, error: "", notice: "" });
        const version = this._version;
        try {
          return await task();
        } catch (e) {
          if (this.active(version))
            this.setData({ error: e.message || "操作失败，请重试" });
        } finally {
          if (this.active(version)) this.setData({ [key]: false });
        }
      },
      active(version) {
        return this._visible && version === this._version;
      },
      later(fn, ms) {
        const timer = setTimeout(() => {
          this._timers.delete(timer);
          if (this._visible) fn();
        }, ms);
        this._timers.add(timer);
        return timer;
      },
      toast(title) {
        wx.showToast({ title, icon: "none" });
      },
      async confirm(content) {
        return new Promise((resolve) =>
          wx.showModal({
            title: "请确认",
            content,
            success: (r) => resolve(r.confirm),
            fail: () => resolve(false),
          }),
        );
      },
      retry() {
        if (this.load) this.load();
      },
    },
    options,
  );
  page.data = Object.assign(
    { busy: false, loading: false, error: "", notice: "", user: null },
    options.data,
  );
  page.onLoad = function (query) {
    this._version = 0;
    this._sessionRevision = api.identity();
    this._visible = false;
    this._timers = new Set();
    if (hooks.onLoad) hooks.onLoad.call(this, query);
    this._routeData = {};
    (options.routeFields || []).forEach((key) => {
      this._routeData[key] = this.data[key];
    });
  };
  page.onShow = function () {
    this._visible = true;
    this._version++;
    if (this._sessionRevision !== api.identity()) {
      this.setData(
        Object.assign(JSON.parse(JSON.stringify(page.data)), this._routeData),
      );
      if (this.onSessionChange) this.onSessionChange();
    }
    this._sessionRevision = api.identity();
    this.setData({ user: api.user(), busy: false, loading: false, error: "" });
    this._unsubscribe = api.subscribe((value) => {
      this._version++;
      this._sessionRevision = api.identity();
      this._timers.forEach(clearTimeout);
      this._timers.clear();
      this.setData(
        Object.assign(JSON.parse(JSON.stringify(page.data)), this._routeData, {
          user: value,
        }),
      );
      if (this.cancelMedia) this.cancelMedia();
      if (this.onSessionChange) this.onSessionChange();
    });
    const beat = () => {
      if (options.studyType && api.token())
        api
          .request("/api/learn/heartbeat", "POST", {
            page_type: options.studyType,
          })
          .catch(() => {});
      if (options.studyType) this.later(beat, 30000);
    };
    beat();
    if (hooks.onShow) hooks.onShow.call(this);
  };
  const stop = function () {
    this._visible = false;
    this._version++;
    this._timers.forEach(clearTimeout);
    this._timers.clear();
    if (this._unsubscribe) this._unsubscribe();
    if (this.cancelMedia) this.cancelMedia();
    this.setData({
      busy: false,
      loading: false,
      recording: false,
      playing: false,
    });
  };
  page.onHide = function () {
    stop.call(this);
    if (hooks.onHide) hooks.onHide.call(this);
  };
  page.onUnload = function () {
    stop.call(this);
    if (hooks.onUnload) hooks.onUnload.call(this);
  };
  Page(page);
}
module.exports = definePage;
