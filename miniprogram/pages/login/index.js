const api = require("../../services/api");
const definePage = require("../../services/page");
definePage({
  data: { register: false, name: "", email: "", password: "", agreed: false },
  mode() {
    this.setData({ register: !this.data.register, password: "", error: "" });
  },
  agree(e) {
    this.setData({ agreed: e.detail.value.length > 0 });
  },
  submit() {
    this.run(async () => {
      const { register, name, email, password, agreed } = this.data;
      if (!email.trim() || !password) throw new Error("请输入邮箱和密码");
      if (register && (!name.trim() || password.length < 8))
        throw new Error("请填写昵称，密码至少 8 位");
      if (register && !agreed)
        throw new Error("请阅读并同意服务条款和隐私政策");
      const value = await api.request(
        register ? "/api/register" : "/api/login",
        "POST",
        { name: name.trim(), email: email.trim(), password },
      );
      api.saveSession(value);
      this.setData({ password: "" });
      if (getCurrentPages().length > 1) wx.navigateBack();
      else wx.switchTab({ url: "/pages/me/index" });
    });
  },
  retry() {
    this.submit();
  },
});
