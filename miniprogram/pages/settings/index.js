const api = require("../../services/api");
const definePage = require("../../services/page");
definePage({
  data: {
    name: "",
    handle: "",
    phone: "",
    oldPassword: "",
    newPassword: "",
    hasPassword: true,
  },
  onShow() {
    if (api.token()) this.load();
  },
  onHide() {
    this.setData({ oldPassword: "", newPassword: "" });
  },
  load() {
    this.run(async () => {
      const version = this._version;
      const [user, account] = await Promise.all([
        api.request("/api/me"),
        api.request("/api/me/account-status"),
      ]);
      if (!this.active(version)) return;
      api.updateUser(user);
      this.setData({
        user,
        name: user.name,
        handle: user.handle,
        phone: user.phone || "",
        hasPassword: account.has_password,
      });
    }, "loading");
  },
  save() {
    this.run(async () => {
      const version = this._version;
      const user = await api.request("/api/me", "PUT", {
        name: this.data.name.trim(),
        handle: this.data.handle.trim(),
        phone: this.data.phone.trim(),
      });
      if (!this.active(version)) return;
      api.updateUser(user);
      this.setData({ user, handle: user.handle, notice: "资料已保存" });
    });
  },
  password() {
    this.run(async () => {
      if (this.data.newPassword.length < 8) throw new Error("新密码至少 8 位");
      const version = this._version;
      await api.request("/api/me/change-password", "POST", {
        current_password: this.data.oldPassword || null,
        new_password: this.data.newPassword,
      });
      if (this.active(version))
        this.setData({
          oldPassword: "",
          newPassword: "",
          notice: "密码已更新",
          hasPassword: true,
        });
    });
  },
});
