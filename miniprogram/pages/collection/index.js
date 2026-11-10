const api = require("../../services/api");
const media = require("../../services/media");
const definePage = require("../../services/page");
definePage({
  routeFields: ["type"],
  data: {
    type: "vocabulary",
    filter: "word",
    items: [],
    page: 0,
    hasMore: true,
  },
  onLoad(query) {
    this.setData({
      type: query.type === "favorites" ? "favorites" : "vocabulary",
    });
  },
  onShow() {
    if (api.token()) this.load();
  },
  cancelMedia() {
    media.stop(this);
  },
  load(more = false) {
    this.run(async () => {
      const version = this._version;
      const page = more ? this.data.page + 1 : 1;
      const path =
        this.data.type === "vocabulary"
          ? `/api/learn/vocabulary?page=${page}&per_page=30`
          : `/api/learn/favorites?entity=${this.data.filter}&limit=30&offset=${(page - 1) * 30}`;
      const value = await api.request(path);
      if (!this.active(version)) return;
      const incoming = api.items(value);
      this.setData({
        items: more ? this.data.items.concat(incoming) : incoming,
        page,
        hasMore:
          this.data.type === "vocabulary"
            ? page * 30 < value.total
            : incoming.length === 30,
      });
    }, "loading");
  },
  more() {
    this.load(true);
  },
  filter(e) {
    this.setData({
      filter: e.currentTarget.dataset.filter,
      items: [],
      page: 0,
    });
    this.load();
  },
  lookup(e) {
    getApp().globalData.dictionaryWord = e.currentTarget.dataset.word;
    wx.switchTab({ url: "/pages/dictionary/index" });
  },
  remove(e) {
    this.run(async () => {
      const version = this._version;
      if (!(await this.confirm("将此项移出当前列表？"))) return;
      if (!this.active(version)) return;
      const id = Number(e.currentTarget.dataset.id),
        item = this.data.items.find((v) => v.id === id);
      if (!item) return;
      if (this.data.type === "vocabulary")
        await api.request("/api/learn/vocabulary/toggle", "POST", {
          word: item.word,
        });
      else
        await api.request(
          `/api/learn/favorites?entity=${this.data.filter}&record_id=${item.record_id}`,
          "DELETE",
        );
      if (this.active(version))
        this.setData({
          items: this.data.items.filter((v) => v.id !== id),
          notice: "已移除",
        });
    });
  },
  play(e) {
    this.run(() => media.sentence(this, "read", e.currentTarget.dataset.id));
  },
});
