const api = require("./services/api");
App({
  onLaunch() {
    api.initialize();
  },
  globalData: { dictionaryWord: "" },
});
