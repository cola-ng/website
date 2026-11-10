const api = require("../../services/api");
const media = require("../../services/media");
const stream = require("../../services/stream");
const definePage = require("../../services/page");
function merge(list, turn) {
  if (!turn || !turn.id) return list;
  return list
    .filter((v) => v.id !== turn.id)
    .concat(turn)
    .sort((a, b) => a.id - b.id);
}
definePage({
  studyType: "chat",
  data: {
    chats: [],
    contexts: [],
    contextIndex: 0,
    chatId: 0,
    title: "",
    turns: [],
    message: "",
    recording: false,
    playing: false,
    live: null,
    hasPrev: false,
    anchor: "",
    showZh: true,
    recovering: false,
  },
  onLoad(query) {
    this._initialId = /^\d+$/.test(query.id || "") ? Number(query.id) : 0;
  },
  onSessionChange() {
    this._initialId = 0;
  },
  onShow() {
    this.setData({
      showZh: wx.getStorageSync(api.storageKey("chatZh")) !== false,
    });
    if (!api.token()) return;
    if (this._initialId) this.setData({ chatId: this._initialId });
    this.load();
  },
  cancelMedia() {
    media.stop(this);
    if (this._stream) this._stream.abort();
    this._stream = null;
    this._recoverKey = null;
    this.setData({ live: null, recording: false, recovering: false });
  },
  async load() {
    return this.run(async () => {
      const version = this._version;
      const [chats, contexts] = await Promise.all([
        api.request("/api/learn/chats?limit=200"),
        api.request("/api/asset/contexts?limit=200"),
      ]);
      if (!this.active(version)) return;
      this.setData({
        chats: api.items(chats),
        contexts: [{ id: null, name_zh: "自由对话" }].concat(
          api.items(contexts),
        ),
      });
      if (this.data.chatId) await this.loadTurns();
    }, "loading");
  },
  selectContext(e) {
    this.setData({ contextIndex: Number(e.detail.value) });
  },
  create() {
    if (!api.requireLogin()) return;
    this.run(async () => {
      const context = this.data.contexts[this.data.contextIndex] || {
        id: null,
        name_zh: "自由对话",
      };
      const version = this._version;
      const chat = await api.request("/api/learn/chats", "POST", {
        title: context.name_zh,
        context_id: context.id,
      });
      if (!this.active(version)) return;
      this.setData({
        chatId: chat.id,
        title: chat.title,
        turns: [],
        message: "",
        hasPrev: false,
      });
      this._initialId = chat.id;
    });
  },
  select(e) {
    this.cancelMedia();
    this._version++;
    const id = Number(e.currentTarget.dataset.id);
    this._initialId = id;
    const chat = this.data.chats.find((v) => v.id === id);
    this.setData({
      chatId: id,
      title: chat ? chat.title : "英语对话",
      turns: [],
      message: "",
      error: "",
      busy: false,
    });
    this.run(() => this.loadTurns(), "loading");
  },
  async loadTurns(older = false) {
    const version = this._version,
      id = this.data.chatId;
    const first = this.data.turns[0];
    const value = await api.request(
      `/api/learn/chats/${id}/turns?limit=30&from_latest=true${older && first ? "&before_id=" + first.id : ""}`,
    );
    if (!this.active(version) || id !== this.data.chatId) return;
    const turns = older
      ? api.items(value).concat(this.data.turns)
      : api.items(value);
    const title =
      (this.data.chats.find((v) => v.id === id) || {}).title ||
      this.data.title ||
      "英语对话";
    this.setData({
      turns,
      title,
      hasPrev: value.has_prev,
      anchor: older && first ? `turn-${first.id}` : "bottom",
    });
    if (turns.some((v) => v.status === "processing")) this.recover();
  },
  older() {
    this.run(() => this.loadTurns(true), "loading");
  },
  recover() {
    if (this._recoverKey || this.data.busy) return;
    const key = {},
      version = this._version;
    this._recoverKey = key;
    this.setData({ recovering: true });
    let attempts = 0;
    const poll = async () => {
      if (this._recoverKey !== key || !this.active(version)) return;
      try {
        const pending = this.data.turns.filter(
          (v) => v.status === "processing",
        );
        let turns = this.data.turns;
        for (const turn of pending) {
          const value = await api.request(`/api/learn/chats/turns/${turn.id}`);
          if (!this.active(version) || this._recoverKey !== key) return;
          turns = merge(turns, value);
        }
        this.setData({ turns });
        if (turns.some((v) => v.status === "processing") && ++attempts < 90)
          this.later(poll, 2000);
        else {
          this._recoverKey = null;
          this.setData({
            recovering: false,
            error: attempts >= 90 ? "回复仍未完成，请稍后刷新" : "",
          });
        }
      } catch (e) {
        this._recoverKey = null;
        if (this.active(version))
          this.setData({ recovering: false, error: e.message });
      }
    };
    this.later(poll, 2000);
  },
  sendText() {
    const text = this.data.message.trim();
    if (text) this.send({ type: "text", message: text });
  },
  async send(body) {
    if (
      !api.requireLogin() ||
      this.data.busy ||
      this.data.recovering ||
      !this.data.chatId
    )
      return;
    const version = this._version;
    this.setData({
      busy: true,
      error: "",
      live: { phase: "正在连接 AI…" },
      anchor: "bottom",
    });
    let accepted = false;
    const connection = stream.send(this.data.chatId, body, (frame) => {
      if (!this.active(version)) return;
      const data = frame.data;
      if (frame.event === "phase")
        this.setData({ live: { phase: data.message } });
      if (frame.event === "accepted") {
        accepted = true;
        this.setData({
          turns: merge(this.data.turns, data.user_turn),
          message: "",
          live: { phase: "AI 正在回复…" },
        });
      }
      if (frame.event === "delta")
        this.setData({
          live: {
            reply_en: data.reply_en,
            reply_zh: data.reply_zh,
            phase: "正在生成回复与学习反馈…",
          },
          anchor: "bottom",
        });
      if (frame.event === "done")
        this.setData({
          turns: merge(merge(this.data.turns, data.user_turn), data.ai_turn),
          live: null,
          anchor: "bottom",
        });
    });
    this._stream = connection;
    try {
      await connection.promise;
    } catch (e) {
      if (this.active(version)) {
        this.setData({ error: e.message });
        if (accepted) {
          try {
            await this.loadTurns();
          } catch (_) {}
        }
      }
    } finally {
      if (this._stream === connection) this._stream = null;
      if (this.active(version)) {
        this.setData({ busy: false, live: null });
        if (this.data.turns.some((v) => v.status === "processing"))
          this.recover();
      }
    }
  },
  record() {
    if (this.data.recording) return media.finish(this);
    if (!this.data.busy && !this.data.recovering)
      media.record(this, (audio) =>
        this.send({ type: "audio", audio_base64: audio }),
      );
  },
  play(e) {
    this.run(
      () =>
        media.play(
          this,
          `/api/learn/chats/turns/${e.currentTarget.dataset.id}/audio`,
        ),
      "playing",
    );
  },
  showTranslation(e) {
    this.setData({ showZh: e.detail.value });
    wx.setStorageSync(api.storageKey("chatZh"), e.detail.value);
  },
  backToList() {
    this.cancelMedia();
    this._version++;
    this._initialId = 0;
    this.setData({ chatId: 0, turns: [], busy: false });
    this.load();
  },
  rename() {
    this.run(async () => {
      const version = this._version;
      const value = await new Promise((resolve) =>
        wx.showModal({
          title: "修改对话标题",
          editable: true,
          placeholderText: this.data.title,
          success: resolve,
          fail: () => resolve({ confirm: false }),
        }),
      );
      if (!value.confirm || !this.active(version)) return;
      const title = (value.content || "").trim();
      if (!title) throw new Error("请输入标题");
      await api.request(`/api/learn/chats/${this.data.chatId}`, "PUT", {
        title,
      });
      if (this.active(version)) this.setData({ title });
    });
  },
  retry() {
    if (api.requireLogin()) this.load();
  },
});
