const api = require("../../services/api");
const definePage = require("../../services/page");
definePage({
  data: { summary: null, tasks: [], insights: [], stages: [], word: "" },
  onShow() {
    this.load();
  },
  load() {
    return this.run(async () => {
      const version = this._version;
      const jobs = [api.request("/api/asset/stages?limit=4")];
      if (api.token())
        jobs.push(
          api.request("/api/learn/summary"),
          api.request("/api/learn/daily"),
        );
      const values = await Promise.allSettled(jobs);
      if (!this.active(version)) return;
      const patch = { summary: null, tasks: [], insights: [], stages: [] };
      if (values[0].status === "fulfilled")
        patch.stages = api.items(values[0].value);
      if (values[1] && values[1].status === "fulfilled")
        patch.summary = values[1].value;
      if (values[2] && values[2].status === "fulfilled") {
        patch.tasks = values[2].value.tasks;
        patch.insights = values[2].value.insights;
      }
      patch.error = values
        .filter((v) => v.status === "rejected")
        .map((v) => v.reason.message)
        .join("；");
      this.setData(patch);
    }, "loading");
  },
  lookup() {
    const word = this.data.word.trim();
    if (!word) return;
    getApp().globalData.dictionaryWord = word;
    wx.switchTab({ url: "/pages/dictionary/index" });
  },
  task(e) {
    this.run(async () => {
      const task = this.data.tasks.find(
        (v) => v.id === Number(e.currentTarget.dataset.id),
      );
      if (!task) return;
      const done = task.status !== "completed";
      await api.request(`/api/learn/daily/tasks/${task.id}`, "PUT", {
        status: done ? "completed" : "pending",
        current_value: done ? task.target_value : 0,
      });
      this.load();
    });
  },
});
