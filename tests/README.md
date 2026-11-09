# 回归测试

`api.py` 使用真实 API 和 PostgreSQL，创建独立测试用户与内容，校验注册登录、权限、词典、收藏、生词、复习、场景/跟读 CRUD、学习统计、钱包与桌面授权。

`browser.cjs` 在实际发布的 WASM 页面执行浏览器回归。登录、资料保存、收藏、复习、内容管理及成绩保存使用真实 API/数据库。AI 翻译、回复及评分使用 Playwright 测试替身，校验请求参数、流式响应、错误提示、文本安全和录音生成的单声道 16 kHz PCM WAV；它不验证外部 AI 服务的可用性或付费凭据。

`ui.cjs` 使用确定性接口数据检查全部 39 个页面入口的桌面与移动端显示，检查主题、字号、导航高亮和横向溢出，并覆盖表单错误、处理中锁定、分页恢复、录音评分和键盘焦点状态。登录卡片标题为桌面 26px / 移动端 24px，普通页面标题为 32px / 26px，正文和输入内容为 16px，标签与辅助文字分别为 15px / 14px。

`parity.cjs` 将迁移前 `b486031a` 的 React 参考界面与实际 WASM 界面注入同一份 `ui-fixtures.cjs` 数据，输出桌面 / 手机对照截图，并验证恢复的页面布局与交互。见 [界面对照说明](../docs/ui-parity.md)。React 参考文件只保存在忽略的构建产物目录中。

`responsive.cjs` 检查 9 种手机、平板、桌面及横屏尺寸，覆盖 81 项页面检查和 318 项布局／交互检查，包括虚拟键盘可见区域模拟、长表单、卡片式表格和缩放后的状态保留。见 [响应式复检说明](../docs/responsive-ui.md)。

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

提供方权限回归由 `tests/providers.py` 执行。`run.ps1` 额外启动只监听本机的 `provider_stub.py`（默认端口 8134，可用 `-ProviderPort` 调整），明确覆盖所有测试 API Key 和上游地址。测试检查真实 SDK 发送的个人/平台凭据、付费条件、配置归属和失败不回退，不消耗真实 API 额度。handle、头像和 AI 配置的规则见 [账户与 AI 说明](../docs/account-and-ai.md)。

正式环境临时账户未付费时，测试凭据文件设置 `expect_personal_required: true`：验证 handle、头像和个人配置的真实操作，并确认没有自备配置时无法发送 AI 请求。不向临时账户提供平台密钥，也不在生产数据库伪造支付记录。

OAuth 检查包含 `tests/oauth.py` 和隔离数据库中的 `oauth_database_flow` Rust 测试。测试服务器使用明确覆盖的假 Google/GitHub 凭据，验证 PKCE 跳转、错误提供方、state/浏览器绑定、取消授权、过期/重复授权结果、账户关联规则及并发解除绑定；不会调用真实第三方 API。真实应用配置与登录应另外在浏览器中验证，详见 `docs/oauth.md`。

`node tests/production.cjs` 在独立的无凭据浏览器中检查正式域名的公开页面、两家的授权跳转、HTTPS Cookie、PKCE 和取消授权。它只读取页面并创建随后取消的 OAuth 会话，不读取用户浏览器或修改生产内容。报告和截图位于 `target/qa-production`。

需要真实对话检查时，可额外设置 `PRODUCTION_QA_FIXTURE` 为专门创建的临时测试账户文件，包含 `email`、`password`、`chat_id`；邮箱必须符合 `qa-stream-数字@example.invalid`。该可选流程会登录这个测试账户并发送一句英语，测量文字提前显示、确认回复保存；需先在该测试对话完成一次 API 生成作为对照，测试后清理专属测试数据。不要提供真实用户的账户、令牌或浏览器资料。
