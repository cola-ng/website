const definePage = require("../../services/page");
const content = require("../../content/legal");
definePage({
  routeFields: ["title", "content"],
  data: { title: "", content: "" },
  onLoad(query) {
    const privacy = query.kind === "privacy";
    this.setData({
      title: privacy ? "隐私政策" : "服务条款",
      content: privacy ? content.privacy : content.terms,
    });
  },
});
