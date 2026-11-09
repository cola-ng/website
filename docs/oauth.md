# Google / GitHub 登录

网站使用服务端 Authorization Code + PKCE 流程。第三方 access token 和客户端密钥只在服务端使用，不写入 WASM、前端存储或回调 URL。浏览器最终收到的是 60 秒内有效、绑定当前浏览器且只能使用一次的本站登录结果。

## 服务端配置

在未跟踪的 `.env` 或部署平台的服务端环境变量中设置以下内容，然后重启后端：

```dotenv
PUBLIC_BASE_URL=https://cola.ng
GITHUB_CLIENT_ID=...
GITHUB_CLIENT_SECRET=...
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
```

`PUBLIC_BASE_URL` 必须是完整的 HTTPS 来源，不能包含路径、查询参数或用户名。仅本地开发允许 `http://127.0.0.1:5173` / `http://localhost:5173`。后端不会从不可信的 Host / Forwarded 请求头推断回调地址。客户端 ID 和密钥同时配置后，登录页才显示对应入口。

Docker Compose 已透传这些变量；镜像构建排除 `.env`。不要将真实密钥写入 `.env.example`、源码或构建参数。

前端 Nginx 通过容器启动模板读取 `API_UPSTREAM`，默认 `colang:8119`。线上旧 Compose 使用 `colang_api:6019` 时须配置对应值。静态资源开启压缩，HTML 重新验证缓存；WASM 尚未下载完时显示加载提示。

## 控制台设置

GitHub OAuth App 归属组织 `cola-ng`，名称 `Colang Web`，主页 `https://cola.ng`。回调地址：

- `https://cola.ng/api/auth/github/callback`
- `http://127.0.0.1:5173/api/auth/github/callback`

Google Auth Platform 使用 Web application 客户端；授权范围只有 `openid email profile`，不需要启用 Gmail、Drive 或付费 API。回调地址：

- `https://cola.ng/api/auth/google/callback`
- `http://127.0.0.1:5173/api/auth/google/callback`

如使用 `localhost`，需额外注册完全一致的 localhost 回调并把本地 `PUBLIC_BASE_URL` 一起修改。Google 品牌主页、隐私政策、条款地址分别为 `https://cola.ng`、`https://cola.ng/privacy`、`https://cola.ng/terms`。应用受众若仍为 Testing，只有配置的测试用户能够登录；面向所有用户前应在控制台完成发布及 Google 要求的品牌/域名验证。

## 账户规则

- 首次登录以提供方验证过的邮箱创建账户，后续以稳定的 provider subject 识别。
- 相同邮箱已有本站账户时不会自动合并；先使用原账户登录，再到「我的学习 → 账户安全」绑定 Google / GitHub。
- 绑定必须在已登录状态下完成第三方授权；不接受客户端直接声明 provider user ID。
- 迁移为旧关联记录添加空的 `verified_at`，保留记录但不将其当作登录凭据。只有通过真实提供方回调验证的记录才能用于登录，避免历史上客户端直接提交 subject 造成账户误关联。
- 解除绑定时至少保留一种登录方式，校验与删除在同一数据库事务中完成。
- 被停用或锁定的账户不能通过第三方登录绕过限制。

## 复检

`tests/run.ps1` 使用隔离的本地数据库、假客户端凭据验证 OAuth 跳转、PKCE、浏览器/state 绑定、取消授权、结果有效期、并发重复消费及解除绑定竞态。该套件不向第三方发送请求。真实控制台注册和真实第三方授权需分别在浏览器中验证，不能用本地测试结果替代。

实现依据：[GitHub 授权流程](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps)、[Google 服务端 OAuth](https://developers.google.com/identity/protocols/oauth2/web-server)、[Google OpenID Connect](https://developers.google.com/identity/openid-connect/openid-connect)。
