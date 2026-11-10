# 开朗英语前端

前端使用 Rust + Glory 0.7.0 + Sapid 0.7.1，编译为 WebAssembly。
不再依赖 React、React Router、Radix、Recharts、Vite、TypeScript 或 Tailwind。
少量原生浏览器适配位于 `src/browser/media.js`（录音、WAV 编码、音频播放与图片裁剪），不包含前端框架。

## 本地开发

需要 Rust 1.95+、`wasm32-unknown-unknown` 和 Trunk。推荐与 CI 一致使用 Rust 1.99。

```powershell
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.22.1 --locked
# 在 website 启动 API（需要 PostgreSQL，参见 .env.example）
cargo run -p colang --bin colang
# 另一个终端
cd front
trunk serve
```

访问 http://127.0.0.1:5173 。Trunk 将 `/api/` 代理到 `127.0.0.1:8119`。

## 发布构建

```powershell
cd front
trunk build --release --locked
```

产物为 `front/dist`。可设置 `STATIC_DIR=front/dist` 由 API 直接托管，或使用 `front/nginx.conf` 配置的 Nginx。API 请求和音频使用同源代理；嵌套路由刷新返回 `index.html`。

站内链接及程序跳转通过 `browser/routing.rs` 使用 History API，URL、页面内容和导航高亮随前进后退同步更新；登录状态在同一应用中更新。外部链接和 `/api/` OAuth 授权仍使用浏览器导航，Ctrl/Command 点击保留新标签页行为。词典查询同样经过路由，可直接分享和重新打开单词地址。

```powershell
# 在 website 预览构建产物（API 默认 8119）
python scripts/serve.py
```

Sapid 的结构样式保存在 `public/sapid.css`，来源为 Sapid 0.7.1，许可证见 `public/SAPID-LICENSE.txt`。升级 Sapid 时同步核对样式。

应用配色与控件字号统一定义在 `src/browser/theme.rs`，页面排版层级在 `app.css`。新增按钮、文本框和复选框请使用 `action_button`、`text_control` 和 `checkbox_control`；删除、重置、撤权等破坏性操作使用 `danger_button`。处理中、错误、成功和空数据提示使用共享组件，保留橙色操作、红色错误、绿色成功和灰色禁用的语义区分。

## 检查与回归

```powershell
cargo fmt -p colang-front --check
cargo test -p colang-front
cargo clippy -p colang-front --target wasm32-unknown-unknown -- -D warnings
```

完整数据库及浏览器测试说明见 `../tests/README.md`。

邮箱登录、注册及 Google/GitHub OAuth 已接入服务端。配置对应服务端凭据后显示第三方登录和账户绑定入口，配置方法见 [OAuth 说明](../docs/oauth.md)。用户可修改 handle、设置头像并配置自己的 AI 服务；非付费账户需自备密钥，平台 AI 仅向已支付且订阅有效的用户提供，详见 [账户与 AI 说明](../docs/account-and-ai.md)。浏览器朗读使用系统语音，跟读原音使用服务端音频。
