const api = require("../../services/api");
const definePage = require("../../services/page");
definePage({
  data: { stats: null },
  onShow() {
    this.load();
  },
  load() {
    if (!api.token()) return;
    this.run(async () => {
      const version = this._version;
      const stats = await api.request("/api/learn/review/stats");
      if (this.active(version)) this.setData({ stats });
    }, "loading");
  },
});
