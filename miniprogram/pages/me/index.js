const api = require("../../services/api");
const definePage = require("../../services/page");
definePage({
  data: { summary: null, access: null, achievements: [] },
  onShow() {
    if (api.token()) this.load();
    else this.setData({ summary: null, access: null, achievements: [] });
  },
  load() {
    this.run(async () => {
      const version = this._version;
      const [user, summary, access, achievements] = await Promise.all([
        api.request("/api/me"),
        api.request("/api/learn/summary"),
        api.request("/api/ai-providers/access"),
        api.request("/api/achievements/my"),
      ]);
      if (!this.active(version)) return;
      api.updateUser(user);
      this.setData({
        user,
        summary,
        access,
        achievements: api.items(achievements),
      });
    }, "loading");
  },
  logout() {
    this.run(async () => {
      if (await this.confirm("退出当前账户？")) api.logout();
    });
  },
});
