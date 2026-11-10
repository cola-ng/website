const api = require("../../services/api");
const media = require("../../services/media");
const practice = require("../../services/practice");
const definePage = require("../../services/page");
const names = {
  smart: "智能复习",
  pronunciation: "发音练习",
  meaning: "词义理解",
  spelling: "拼写练习",
  recognition: "单词识别",
  phrase: "短语用法",
};
definePage({
  routeFields: ["type", "name"],
  studyType: "review",
  data: {
    type: "smart",
    name: "智能复习",
    items: [],
    index: 0,
    current: null,
    answer: "",
    revealed: false,
    checked: false,
    correct: false,
    recording: false,
    evaluation: null,
    done: false,
    completed: 0,
    saved: false,
  },
  onLoad(query) {
    const type = names[query.type] ? query.type : "smart";
    this.setData({ type, name: names[type] });
  },
  onShow() {
    if (api.token()) this.load();
  },
  cancelMedia() {
    media.stop(this);
  },
  load() {
    this.run(async () => {
      const version = this._version;
      const value = await api.request(
        `/api/learn/review/items?type=${this.data.type}&limit=20`,
      );
      if (!this.active(version)) return;
      const items = api.items(value).map((item) =>
        Object.assign({}, item, {
          typeLabel: names[item.review_type] || "复习",
        }),
      );
      this.setData({
        items,
        index: 0,
        current: items[0] || null,
        answer: "",
        revealed: false,
        checked: false,
        done: false,
        completed: 0,
        evaluation: null,
        saved: false,
      });
      this._started = Date.now();
    }, "loading");
  },
  reveal() {
    this.setData({ revealed: true });
  },
  check() {
    this.setData({
      correct:
        practice.normalize(this.data.answer) ===
        practice.normalize(this.data.current.word),
      checked: true,
      revealed: true,
    });
  },
  record() {
    practice.record(this, this.data.current.word);
  },
  submit(e) {
    this.run(async () => {
      const item = this.data.current;
      if (!item || this.data.saved) return;
      if (!this.data.revealed && item.review_type !== "pronunciation")
        throw new Error("请先作答或查看答案");
      let result = e.currentTarget.dataset.result;
      if (item.review_type === "spelling") {
        if (!this.data.checked) throw new Error("请先检查拼写");
        result = this.data.correct ? "correct" : "incorrect";
      }
      const score = this.data.evaluation && this.data.evaluation.overall_score;
      if (item.review_type === "pronunciation") {
        if (score == null) throw new Error("请先录音完成发音评估");
        result =
          score >= 80 ? "correct" : score >= 60 ? "partial" : "incorrect";
      }
      if (!["correct", "partial", "incorrect"].includes(result))
        throw new Error("请选择掌握程度");
      const version = this._version;
      const value = await api.request("/api/learn/review/submit", "POST", {
        source: item.source,
        source_id: item.source_id,
        review_type: item.review_type,
        result,
        score: score == null ? null : score,
        time_spent_ms: Math.max(0, Date.now() - this._started),
      });
      if (!this.active(version)) return;
      if (!value.success) throw new Error("复习记录保存失败，请重试");
      this.setData({
        saved: true,
        completed: this.data.completed + 1,
        notice: "复习已保存",
      });
    });
  },
  next() {
    if (!this.data.saved) return;
    media.stop(this);
    const index = this.data.index + 1;
    this.setData({
      index,
      current: this.data.items[index] || null,
      answer: "",
      revealed: false,
      checked: false,
      correct: false,
      evaluation: null,
      saved: false,
      notice: "",
      done: index >= this.data.items.length,
    });
    this._started = Date.now();
  },
});
