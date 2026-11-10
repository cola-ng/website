const api = require("../../services/api");
const media = require("../../services/media");
const practice = require("../../services/practice");
const definePage = require("../../services/page");
definePage({
  routeFields: ["id"],
  studyType: "stage",
  data: {
    id: 0,
    stage: null,
    scripts: [],
    scriptId: 0,
    turns: [],
    recording: false,
    evaluatingId: 0,
    evaluation: null,
  },
  onLoad(query) {
    this.setData({ id: /^\d+$/.test(query.id || "") ? Number(query.id) : 0 });
  },
  onShow() {
    this.load();
  },
  cancelMedia() {
    media.stop(this);
  },
  load() {
    this.run(async () => {
      if (!this.data.id) throw new Error("场景地址无效");
      const version = this._version;
      const [stage, scripts] = await Promise.all([
        api.request(`/api/asset/stages/${this.data.id}`),
        api.request(`/api/asset/stages/${this.data.id}/scripts`),
      ]);
      if (this.active(version))
        this.setData({ stage, scripts: api.items(scripts) });
    }, "loading");
  },
  selectScript(e) {
    this.run(async () => {
      const id = Number(e.currentTarget.dataset.id),
        version = this._version;
      media.stop(this);
      const turns = await api.request(`/api/asset/scripts/${id}/turns`);
      if (this.active(version))
        this.setData({
          scriptId: id,
          turns: api.items(turns),
          evaluation: null,
          evaluatingId: 0,
        });
    });
  },
  play(e) {
    this.run(() => media.sentence(this, "script", e.currentTarget.dataset.id));
  },
  record(e) {
    const id = Number(e.currentTarget.dataset.id);
    const turn = this.data.turns.find((v) => v.id === id);
    if (!turn || (this.data.recording && this.data.evaluatingId !== id)) return;
    if (!api.requireLogin()) return;
    this.setData({ evaluatingId: id });
    practice.record(this, turn.content_en);
  },
});
