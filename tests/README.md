# 回归测试

`api.py` 使用真实 API 和 PostgreSQL，创建独立测试用户与内容，校验注册登录、权限、词典、收藏、生词、复习、场景/跟读 CRUD、学习统计、钱包与桌面授权。

`browser.cjs` 在实际发布的 WASM 页面执行浏览器回归。登录、资料保存、收藏、复习、内容管理及成绩保存使用真实 API/数据库。AI 翻译、回复及评分使用 Playwright 测试替身，校验请求参数、异步轮询、错误提示、文本安全和录音生成的单声道 16 kHz PCM WAV；它不验证外部 AI 服务的可用性或付费凭据。

`ui.cjs` 使用确定性接口数据检查全部 37 个页面入口的桌面与移动端显示，检查主题、字号、导航高亮和横向溢出，并覆盖表单错误、处理中锁定、分页恢复、录音评分和键盘焦点状态。登录卡片标题为桌面 26px / 移动端 24px，普通页面标题为 32px / 26px，正文和输入内容为 16px，标签与辅助文字分别为 15px / 14px。

## Windows 一键运行

需要 PowerShell 7.4+、Rust、Trunk、Python 3、Node.js 和 PostgreSQL 18。将 PostgreSQL 的 `bin` 加入 PATH；前端工具安装步骤见 `../front/README.md`。

```powershell
cd tests
npm ci
npx playwright install chromium
cd ..
pwsh -File tests/run.ps1
```

默认占用本地端口 55439（数据库）、8129（API）和 5173（页面）；可使用脚本参数更改。运行前关闭占用这些端口的进程。脚本通过 `cargo metadata` 获取所属 workspace 的正常 `target_directory`，在其中创建新的隔离数据库和音频目录，不读取真实数据库，不调用外部 AI，也不运行 Docker。结束时停止它创建的进程，保留日志、截图和测试数据供检查。

浏览器截图保存在 `target/qa-screenshots`。`target/qa-fixture.json` 只包含本地测试账户的数据，不应提交。

也可以分别运行：设置 `QA_DATABASE_URL` 指向本地 `/colang_qa` 数据库，启动使用该数据库的 API 后执行 `python tests/api.py`，再启动 `python scripts/serve.py --api http://127.0.0.1:8129`，执行 `node tests/browser.cjs`。测试不会接受远程数据库地址。

开发机已有 Playwright 时，可通过 `PLAYWRIGHT_MODULE` 指向其模块目录，无需重复安装。测试依赖只用于验证，不参与前端构建或运行。
