const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const compiler = require("miniprogram-compiler");
const { setup } = require("./helpers.cjs");
const root = path.resolve(__dirname, "..");
function files(dir) {
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .flatMap((entry) =>
      entry.name === "node_modules"
        ? []
        : entry.isDirectory()
          ? files(path.join(dir, entry.name))
          : [path.join(dir, entry.name)],
    );
}
const allFiles = files(root);
for (const file of allFiles) {
  if (file.endsWith(".json")) JSON.parse(fs.readFileSync(file, "utf8"));
  if (file.endsWith(".js") || file.endsWith(".cjs"))
    new vm.Script(fs.readFileSync(file, "utf8"), { filename: file });
}
const app = require("../app.json");
assert.equal(new Set(app.pages).size, app.pages.length);
for (const page of app.pages)
  for (const ext of ["js", "json", "wxml"])
    assert.ok(
      fs.existsSync(path.join(root, page + "." + ext)),
      page + "." + ext,
    );
for (const item of app.tabBar.list)
  assert.ok(app.pages.includes(item.pagePath));
const source = compiler.wxmlToJs(root, { maxBuffer: 4 * 1024 * 1024 });
const context = vm.createContext({ window: {}, console });
const render = vm.runInContext(
  "(function(global){" + source + "})({})",
  context,
);
const platform =
  process.platform === "win32"
    ? "windows/wcsc.exe"
    : process.platform === "darwin"
      ? "mac/wcsc"
      : "linux/wcsc";
const cssCompiler = path.join(
  path.dirname(require.resolve("miniprogram-compiler")),
  "bin",
  platform,
);
const cssFiles = allFiles
  .filter((f) => f.endsWith(".wxss"))
  .map((f) => path.relative(root, f).replace(/\\/g, "/"));
const css = spawnSync(cssCompiler, ["-db", ...cssFiles], {
  cwd: root,
  encoding: "utf8",
  maxBuffer: 4 * 1024 * 1024,
});
assert.equal(css.status, 0, css.stderr);
function walk(node, visitor) {
  if (!node || typeof node !== "object") return;
  visitor(node);
  (node.children || []).forEach((child) => walk(child, visitor));
}
let states = 0;
function checkRender(pagePath, instance, data) {
  const tree = render(pagePath + ".wxml")(
    Object.assign({}, instance.data, data),
  );
  assert.equal(tree.tag, "wx-page");
  walk(tree, (node) => {
    for (const [key, value] of Object.entries(node.attr || {})) {
      if (/^(bind|catch)/.test(key) && value)
        assert.equal(
          typeof instance[value],
          "function",
          pagePath + ": missing handler " + value,
        );
      if (key === "data-url")
        assert.ok(
          app.pages.includes(String(value).split("?")[0].replace(/^\//, "")),
          "Broken page: " + value,
        );
    }
  });
  states++;
  return tree;
}
for (const pagePath of app.pages) {
  const h = setup(),
    instance = h.page(pagePath.split("/")[1]);
  checkRender(pagePath, instance, { user: null });
  checkRender(pagePath, instance, {
    user: {
      id: 1,
      name: "学习者",
      handle: "learner",
      email: "test@example.com",
    },
  });
}
const h = setup();
const chat = h.page("chat");
checkRender("pages/chat/index", chat, {
  user: { id: 1 },
  chatId: 1,
  turns: [
    {
      id: 1,
      speaker: "assistant",
      content_en: "Hello",
      content_zh: "你好",
      audio_path: "audio.wav",
      issues: [
        {
          id: 1,
          original_text: "I has",
          suggested_text: "I have",
          description_zh: "注意动词",
        },
      ],
    },
  ],
  live: { reply_en: "Hello", reply_zh: "你好", phase: "生成中" },
});
for (const type of [
  "pronunciation",
  "meaning",
  "spelling",
  "recognition",
  "phrase",
]) {
  const review = h.page("review");
  for (const revealed of [false, true])
    checkRender("pages/review/index", review, {
      user: { id: 1 },
      current: { word: "hello", word_zh: "你好", review_type: type },
      items: [{ id: 1 }],
      revealed,
      evaluation: { overall_score: 88, feedback: [{ message: "继续练习" }] },
    });
}
const provider = h.page("providers");
const models = ["chat", "tts", "asr"].map((cap) => ({
  cap,
  supported: true,
  enabled: true,
  model: cap,
  label: cap,
  suggestions: ["model-a", "model-b"],
}));
const tree = checkRender("pages/providers/index", provider, {
  user: { id: 1 },
  editor: true,
  models,
  catalog: [{ label: "智谱" }],
  regions: ["cn", "global"],
});
const suggestionIndices = [];
walk(tree, (node) => {
  if (node.attr && node.attr.bindtap === "suggest")
    suggestionIndices.push(node.attr["data-index"]);
});
assert.deepEqual(suggestionIndices.map(Number), [0, 0, 1, 1, 2, 2]);
console.log(
  `检查通过：${app.pages.length} 个页面、${states} 个模板状态、全部 JavaScript/JSON/WXML/WXSS、事件处理器与站内路由。`,
);
