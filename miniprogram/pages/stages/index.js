const api = require("../../services/api");
const definePage = require("../../services/page");
definePage({
  data: { stages: [] },
  onShow() {
    this.load();
  },
  load() {
    this.run(async () => {
      const version = this._version;
      const value = await api.request("/api/asset/stages?limit=200");
      if (this.active(version)) this.setData({ stages: api.items(value) });
    }, "loading");
  },
});
