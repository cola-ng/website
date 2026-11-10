const api = require("../../services/api");
const media = require("../../services/media");
const definePage = require("../../services/page");
definePage({
  studyType: "dictionary",
  data: {
    mode: "word",
    word: "",
    result: null,
    history: [],
    text: "",
    translated: null,
    target: "zh",
    favorite: false,
    inVocabulary: false,
  },
  onShow() {
    this.loadHistory();
    const word = getApp().globalData.dictionaryWord;
    if (word) {
      getApp().globalData.dictionaryWord = "";
      this.setData({ word, mode: "word" });
      this.lookup();
    }
  },
  onSessionChange() {
    this.loadHistory();
  },
  cancelMedia() {
    media.stop(this);
  },
  switchMode(e) {
    this.setData({ mode: e.currentTarget.dataset.mode, error: "" });
  },
  async loadHistory() {
    const version = this._version;
    try {
      const history = api.token()
        ? api.items(await api.request("/api/dict/words/searched?limit=20"))
        : (wx.getStorageSync(api.storageKey("history")) || []).map((word) => ({
            word,
          }));
      if (this.active(version)) this.setData({ history });
    } catch (e) {
      if (this.active(version)) this.setData({ error: e.message });
    }
  },
  historyWord(e) {
    this.setData({ word: e.currentTarget.dataset.word });
    this.lookup();
  },
  lookup() {
    return this.run(async () => {
      const word = this.data.word.trim();
      if (!word) throw new Error("请输入单词");
      media.stop(this);
      const version = this._version;
      this.setData({ result: null });
      const result = await api.request(
        "/api/dict/lookup?word=" + encodeURIComponent(word),
      );
      if (!this.active(version)) return;
      this.setData({ result, favorite: false, inVocabulary: false });
      if (api.token()) {
        await api.request("/api/dict/words/searched", "POST", { word });
        const [saved, vocabulary] = await Promise.all([
          api.request("/api/learn/favorites/check", "POST", {
            entity: "word",
            record_ids: [result.word.id],
          }),
          api.request("/api/learn/vocabulary/check", "POST", {
            words: [result.word.word],
          }),
        ]);
        if (this.active(version))
          this.setData({
            favorite: saved.includes(result.word.id),
            inVocabulary: (vocabulary.favorites || []).includes(
              result.word.word.toLowerCase(),
            ),
          });
      } else {
        const old = wx.getStorageSync(api.storageKey("history")) || [];
        wx.setStorageSync(
          api.storageKey("history"),
          [word].concat(old.filter((v) => v !== word)).slice(0, 30),
        );
      }
      if (this.active(version)) this.loadHistory();
    }, "loading");
  },
  clearHistory() {
    return this.run(async () => {
      const version = this._version;
      if (!(await this.confirm("清空查词历史？"))) return;
      if (!this.active(version)) return;
      if (api.token()) await api.request("/api/dict/words/searched", "DELETE");
      else wx.removeStorageSync(api.storageKey("history"));
      if (this.active(version)) this.setData({ history: [] });
    });
  },
  favoriteWord() {
    if (!api.requireLogin()) return;
    this.run(async () => {
      const version = this._version;
      const value = await api.request("/api/learn/favorites/toggle", "POST", {
        entity: "word",
        record_id: this.data.result.word.id,
      });
      if (this.active(version))
        this.setData({
          favorite: value.is_favorited,
          notice: value.is_favorited ? "已收藏" : "已取消收藏",
        });
    });
  },
  vocabulary() {
    if (!api.requireLogin()) return;
    this.run(async () => {
      const version = this._version;
      const value = await api.request("/api/learn/vocabulary/toggle", "POST", {
        word: this.data.result.word.word,
      });
      if (this.active(version))
        this.setData({
          inVocabulary: value.added,
          notice: value.added ? "已加入生词本" : "已移出生词本",
        });
    });
  },
  pronunciation(e) {
    this.run(() =>
      media.play(
        this,
        `/api/dict/pronunciations/${e.currentTarget.dataset.id}/audio`,
      ),
    );
  },
  sentence(e) {
    this.run(() => media.sentence(this, "dict", e.currentTarget.dataset.id));
  },
  target(e) {
    this.setData({ target: e.detail.value ? "en" : "zh" });
  },
  translate() {
    if (!api.requireLogin()) return;
    this.run(async () => {
      const text = this.data.text.trim();
      if (!text) throw new Error("请输入要翻译的内容");
      const target = this.data.target,
        version = this._version;
      this.setData({ translated: null });
      const result = await api.request("/api/dict/translate", "POST", {
        text,
        source: "auto",
        target,
        options: {
          tone: "original",
          auto_correct: true,
          preserve_formatting: true,
        },
      });
      if (this.active(version))
        this.setData({
          translated: Object.assign({}, result, {
            source_text: text,
            target_lang: target,
          }),
        });
    });
  },
  copy() {
    wx.setClipboardData({ data: this.data.translated.translated });
  },
  favoriteTranslation() {
    this.run(async () => {
      const version = this._version;
      const value = this.data.translated;
      const result = await api.request("/api/dict/translate/favorite", "POST", {
        source_text: value.source_text,
        translated_text: value.translated,
        source_lang:
          value.detected_source || (value.target_lang === "zh" ? "en" : "zh"),
        target_lang: value.target_lang,
        tone: "original",
      });
      if (this.active(version))
        this.setData({
          notice: result.is_favorited ? "译文已收藏" : "已取消收藏译文",
        });
    });
  },
  retry() {
    if (this.data.mode === "word") this.lookup();
    else this.translate();
  },
});
