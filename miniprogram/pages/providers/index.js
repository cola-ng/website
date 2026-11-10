const api = require("../../services/api");
const definePage = require("../../services/page");
const labels = { chat: "文本对话", tts: "语音合成 TTS", asr: "语音识别 ASR" };
definePage({
  data: {
    providers: [],
    catalog: [],
    access: null,
    editor: false,
    editingId: 0,
    providerIndex: 0,
    apiKey: "",
    models: [],
    regions: [],
    regionIndex: 0,
    voice: "",
    testResults: {},
  },
  onShow() {
    if (api.token()) this.load();
  },
  onHide() {
    this.setData({ apiKey: "", editor: false });
  },
  load() {
    return this.run(async () => {
      const version = this._version;
      const [rows, catalog, access] = await Promise.all([
        api.request("/api/ai-providers"),
        api.request("/api/ai-providers/catalog"),
        api.request("/api/ai-providers/access"),
      ]);
      if (!this.active(version)) return;
      const definitions = api.items(catalog);
      this.setData({
        providers: api
          .items(rows)
          .map((row) =>
            Object.assign({}, row, {
              label:
                (definitions.find((v) => v.id === row.provider_type) || {})
                  .label || row.provider_type,
            }),
          ),
        catalog: definitions,
        access,
      });
    }, "loading");
  },
  defaults(index, row) {
    const def = this.data.catalog[index];
    if (!def) return;
    const settings = row ? row.settings : {};
    const models = ["chat", "tts", "asr"].map((cap) => {
      const suggestions = def[cap + "_models"] || [];
      const model = row ? settings[cap + "_model"] || "" : suggestions[0] || "";
      return {
        cap,
        label: labels[cap],
        supported: suggestions.length > 0,
        enabled: Boolean(model),
        model,
        suggestions,
      };
    });
    const regions = def.regions || [];
    this.setData({
      models,
      regions,
      regionIndex: Math.max(0, regions.indexOf(settings.region)),
      voice: settings.tts_voice || def.voice || "",
      providerIndex: index,
      apiKey: "",
    });
  },
  add() {
    this.defaults(0);
    this.setData({ editor: true, editingId: 0, error: "", notice: "" });
  },
  edit(e) {
    const row = this.data.providers.find(
      (v) => v.id === Number(e.currentTarget.dataset.id),
    );
    if (!row) return;
    this.defaults(
      this.data.catalog.findIndex((v) => v.id === row.provider_type),
      row,
    );
    this.setData({ editor: true, editingId: row.id, error: "", notice: "" });
  },
  close() {
    this.setData({ editor: false, apiKey: "" });
  },
  provider(e) {
    this.defaults(Number(e.detail.value));
  },
  region(e) {
    this.setData({ regionIndex: Number(e.detail.value) });
  },
  model(e) {
    this.setData({
      [`models[${e.currentTarget.dataset.index}].model`]: e.detail.value,
    });
  },
  toggleModel(e) {
    this.setData({
      [`models[${e.currentTarget.dataset.index}].enabled`]: e.detail.value,
    });
  },
  suggest(e) {
    this.setData({
      [`models[${e.currentTarget.dataset.index}].model`]:
        e.currentTarget.dataset.model,
    });
  },
  save() {
    this.run(async () => {
      const def = this.data.catalog[this.data.providerIndex];
      if (!def) throw new Error("请选择 AI 平台");
      if (!this.data.editingId && !this.data.apiKey.trim())
        throw new Error("请输入 API Key");
      const settings = {
          region: this.data.regions[this.data.regionIndex],
          tts_voice: this.data.voice.trim(),
        },
        activate_for = [];
      for (const model of this.data.models) {
        if (model.enabled && !model.model.trim())
          throw new Error("请填写已启用能力的模型 ID");
        settings[model.cap + "_model"] = model.enabled
          ? model.model.trim()
          : "";
        if (model.enabled) activate_for.push(model.cap);
      }
      if (!activate_for.length) throw new Error("请至少配置一种能力");
      if (this.data.apiKey.trim()) settings.api_key = this.data.apiKey.trim();
      const id = this.data.editingId,
        version = this._version;
      await api.request(
        id ? `/api/ai-providers/${id}` : "/api/ai-providers",
        id ? "PUT" : "POST",
        id
          ? { settings, activate_for }
          : { provider_type: def.id, settings, activate_for },
      );
      if (!this.active(version)) return;
      this.setData({
        apiKey: "",
        editor: false,
        notice: "AI 配置已保存并启用",
        testResults: {},
      });
      await this.load();
    });
  },
  activate(e) {
    this.run(async () => {
      await api.request(
        `/api/ai-providers/${e.currentTarget.dataset.id}/activate`,
        "POST",
        {},
      );
      await this.load();
    });
  },
  deactivate(e) {
    this.run(async () => {
      await api.request(
        `/api/ai-providers/${e.currentTarget.dataset.id}`,
        "PUT",
        { activate_for: [] },
      );
      await this.load();
    });
  },
  test(e) {
    this.run(async () => {
      if (
        !(await this.confirm(
          "连接测试会调用本配置的文本、TTS 和 ASR 模型，可能产生少量费用。继续？",
        ))
      )
        return;
      const version = this._version,
        id = e.currentTarget.dataset.id;
      const value = await api.request(
        `/api/ai-providers/${id}/test`,
        "POST",
        {},
      );
      if (!this.active(version)) return;
      const detail = (value.capabilities || [])
        .map(
          (v) =>
            `${labels[v.capability] || v.capability}：${v.success ? "通过" : v.error || "失败"}`,
        )
        .join("；");
      this.setData({
        [`testResults.${id}`]:
          detail || value.error || (value.success ? "连接通过" : "连接失败"),
      });
      if (!value.success)
        throw new Error(value.error || "部分能力连接失败，请检查配置");
    });
  },
  remove(e) {
    this.run(async () => {
      if (!(await this.confirm("删除此 AI 配置？对应的默认能力将停用。")))
        return;
      await api.request(
        `/api/ai-providers/${e.currentTarget.dataset.id}`,
        "DELETE",
      );
      await this.load();
    });
  },
});
