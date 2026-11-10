const api = require("../../services/api");
const media = require("../../services/media");
const practice = require("../../services/practice");
const definePage = require("../../services/page");
definePage({
  studyType: "read",
  data: {
    subjects: [],
    subjectIndex: 0,
    sentences: [],
    index: 0,
    current: null,
    level: 1,
    allCompleted: false,
    recording: false,
    evaluation: null,
    saved: false,
    practiceId: "",
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
      const subject = this.data.subjects[this.data.subjectIndex];
      const suffix = subject && subject.id ? "&subject_id=" + subject.id : "";
      const [subjects, value] = await Promise.all([
        api.request("/api/asset/read/subjects?per_page=100"),
        api.request("/api/asset/read/recommended?limit=10" + suffix),
      ]);
      if (!this.active(version)) return;
      const sentences = api.items(value);
      this.setData({
        subjects: [{ id: 0, title_zh: "全部主题" }].concat(api.items(subjects)),
        sentences,
        current: sentences[0] || null,
        index: 0,
        level: value.user_level,
        allCompleted: value.all_completed,
        evaluation: null,
        saved: false,
        practiceId: practice.practiceId(),
      });
    }, "loading");
  },
  subject(e) {
    media.stop(this);
    this.setData({ subjectIndex: Number(e.detail.value) });
    this.load();
  },
  next() {
    media.stop(this);
    const index = this.data.index + 1;
    if (index >= this.data.sentences.length) {
      this.load();
      return;
    }
    this.setData({
      index,
      current: this.data.sentences[index],
      evaluation: null,
      saved: false,
      error: "",
      notice: "",
      practiceId: practice.practiceId(),
    });
  },
  play() {
    if (this.data.current)
      this.run(() => media.sentence(this, "read", this.data.current.id));
  },
  record() {
    if (!this.data.current) return;
    this.setData({ saved: false });
    if (!this.data.recording)
      this.setData({ practiceId: practice.practiceId() });
    practice.record(this, this.data.current.content_en, () => this.save());
  },
  async save() {
    if (!this.data.evaluation || this.data.saved) return;
    const version = this._version;
    const score = this.data.evaluation;
    const value = await api.request(
      "/api/asset/read/practice",
      "POST",
      Object.assign(
        {
          sentence_id: this.data.current.id,
          practice_id: this.data.practiceId,
        },
        {
          overall_score: score.overall_score,
          pronunciation_score: score.pronunciation_score,
          fluency_score: score.fluency_score,
          intonation_score: score.intonation_score,
        },
      ),
    );
    if (!this.active(version)) return;
    if (!value.success) throw new Error("练习记录保存失败，请重试保存");
    this.setData({
      saved: true,
      notice: value.leveled_up
        ? `练习已保存，已升至 ${value.new_level} 级`
        : "练习已保存",
    });
  },
  saveAgain() {
    this.run(() => this.save());
  },
  favorite() {
    if (!api.requireLogin() || !this.data.current) return;
    this.run(async () => {
      const version = this._version;
      const value = await api.request("/api/learn/favorites/toggle", "POST", {
        entity: "sentence",
        record_id: this.data.current.id,
      });
      if (this.active(version))
        this.setData({
          notice: value.is_favorited ? "句子已收藏" : "已取消收藏",
        });
    });
  },
  advance() {
    this.run(async () => {
      await api.request("/api/asset/read/advance-level", "POST", {
        skill_type: "pronunciation",
      });
      this.load();
    });
  },
  retry() {
    if (this.data.evaluation && !this.data.saved) this.saveAgain();
    else if (api.token()) this.load();
  },
});
