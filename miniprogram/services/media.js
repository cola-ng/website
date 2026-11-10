const api = require("./api");
let owner = null,
  player = null,
  manager = null,
  stopHandler = null,
  errorHandler = null,
  operation = 0;
function stop(page) {
  if (owner !== page) return;
  operation++;
  owner = null;
  if (player) {
    player.destroy();
    player = null;
  }
  if (manager) {
    if (stopHandler) manager.offStop(stopHandler);
    if (errorHandler) manager.offError(errorHandler);
    manager.stop();
    manager = null;
  }
}
async function play(page, path) {
  stop(owner);
  owner = page;
  const ticket = ++operation;
  const version = page._version;
  const sentToken = api.token();
  const remote = api.url(path); // 只下载本站 API；JWT 不进入 URL 或外部音频域名。
  const file = await new Promise((resolve, reject) =>
    wx.downloadFile({
      url: remote,
      header: api.headers(sentToken),
      success(res) {
        if (res.statusCode !== 200)
          return reject(api.responseError(res.statusCode, null, sentToken));
        resolve(res.tempFilePath);
      },
      fail() {
        reject(new Error("音频下载失败，请重试"));
      },
    }),
  );
  if (owner !== page || ticket !== operation || !page.active(version)) return;
  player = wx.createInnerAudioContext();
  player.onError(() => {
    if (page.active(version)) page.setData({ error: "音频暂时无法播放" });
  });
  player.src = file;
  player.play();
}
function record(page, onReady) {
  if (!api.requireLogin()) return;
  stop(owner);
  owner = page;
  const ticket = ++operation;
  const version = page._version;
  const authorize = () =>
    wx.authorize({
      scope: "scope.record",
      success() {
        if (owner !== page || ticket !== operation || !page.active(version))
          return;
        manager = wx.getRecorderManager();
        stopHandler = (res) => {
          page.setData({ recording: false });
          if (owner !== page || ticket !== operation || !page.active(version))
            return;
          if (res.duration < 700) {
            page.setData({ error: "录音太短，请至少说一句英语" });
            return;
          }
          wx.getFileSystemManager().readFile({
            filePath: res.tempFilePath,
            encoding: "base64",
            success(file) {
              if (
                owner === page &&
                ticket === operation &&
                page.active(version)
              )
                onReady(file.data);
            },
            fail() {
              if (page.active(version))
                page.setData({ error: "读取录音失败，请重试" });
            },
          });
        };
        errorHandler = () => {
          if (page.active(version))
            page.setData({
              recording: false,
              error: "录音失败，请检查麦克风权限",
            });
        };
        manager.onStop(stopHandler);
        manager.onError(errorHandler);
        page.setData({ recording: true, error: "" });
        manager.start({
          duration: 60000,
          sampleRate: 16000,
          numberOfChannels: 1,
          format: "wav",
        });
      },
      fail() {
        if (!page.active(version)) return;
        page.setData({ error: "请在小程序设置中允许使用麦克风" });
        wx.showModal({
          title: "需要麦克风权限",
          content: "语音对话和跟读需要录音权限。",
          confirmText: "打开设置",
          success(res) {
            if (res.confirm) wx.openSetting({});
          },
        });
      },
    });
  if (wx.requirePrivacyAuthorize)
    wx.requirePrivacyAuthorize({
      success: authorize,
      fail() {
        if (owner === page && ticket === operation && page.active(version))
          page.setData({ error: "需要同意小程序隐私授权后才能录音" });
      },
    });
  else authorize();
}
function finish(page) {
  if (owner === page && manager) manager.stop();
}
async function sentence(page, kind, id) {
  const version = page._version;
  const path = `/api/sentence-audio/${kind}/${id}`;
  let value = await api.request(path);
  if (!page.active(version)) return;
  if (!value.audio_url && value.can_generate) {
    if (
      !(await page.confirm(
        "使用你自己的 TTS API Key 生成发音？费用由服务商收取，音频会提交审核。",
      ))
    )
      return;
    if (!page.active(version)) return;
    value = await api.request(path + "/generate", "POST", {});
    if (!page.active(version)) return;
  }
  if (!value.audio_url)
    throw new Error(
      value.status === "generating"
        ? "发音正在生成，请稍后再试"
        : "暂无可用发音，可在 AI 服务中配置个人 TTS",
    );
  let audio = value.audio_url;
  if (audio.startsWith(api.baseUrl() + "/api/"))
    audio = audio.slice(api.baseUrl().length);
  await play(page, audio);
}
module.exports = { play, stop, record, finish, sentence };
