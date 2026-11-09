# cola.ng 部署说明

服务器通过 `root@cola.ng` 管理。现有应用的 Compose 位于 `/repo/colang/deploy-prod`，仅更新 `colang_api` 和 `colang_wui`；PostgreSQL 及同机其他服务保持原有部署。

- API 容器内部端口为 `6019`，主机端口为 `6019`。
- 前端通过主机端口 `6018` 提供服务；容器设置 `API_UPSTREAM=colang_api:6019`。
- OAuth 环境变量放在 Compose 目录内权限为 `600` 的 `.oauth.env`，通过后端的 `env_file` 加载。该文件不提交仓库，也不作为镜像构建上下文。
- 使用 `AI_PROVIDER_DEFAULT=zhipu` 和 `ZHIPU_API_KEY` 配置平台 AI 服务；可通过 `ZHIPU_CHAT_MODEL` 指定模型。

生产前端使用与 `front/Dockerfile` 相同的 Nginx 运行时模板。带内容哈希的 WASM、JS、CSS 使用长期不可变缓存；发布新版本时保留旧哈希资源，避免已打开的页面在切换期间加载失败。HTML 使用 `Cache-Control: no-cache`。

外层 Caddy 的对应路由为：

```caddyfile
cola.ng {
    encode zstd gzip
    request_body {
        max_size 14MB
    }
    @api path /api /api/*
    handle @api {
        reverse_proxy 127.0.0.1:6019
    }
    handle {
        reverse_proxy 127.0.0.1:6018
    }
}
```

`app.cola.ng` 的旧 API 来源保留，非 API 页面跳转到 `https://cola.ng{uri}`，让网页登录及 OAuth Cookie 保持同一来源。修改 Caddy 后先验证配置，再平滑加载。

两个站点均限制请求体为 14 MB；生产实测超大请求返回 413，正常请求继续由应用执行更细的大小及权限校验。该边界限制缓解旧版表单解析的无界内存风险，依赖本身仍需升级，不能将代理限制视为完整修复。配置依据：[Caddy request_body](https://caddyserver.com/docs/caddyfile/directives/request_body)。

此次发布目录为 `/repo/colang/releases/oauth-20261109`，其中 `backup` 保存变更前 Compose、Caddy 配置及数据库备份，权限仅限 root。更新镜像前保存当前 Compose 和环境文件；API 与前端通过健康检查后才完成切换。保留上一版 `colang-api:oauth-20261109`、`colang-wui:oauth-20261109` 镜像供回退。回退代码时无需回退本次仅新增可空验证时间字段的迁移。

不要将完整 `docker inspect`、`docker compose config` 或环境文件输出到公开日志，它们可能包含密钥。验证压缩时请求真实 WASM URL，检查 `Content-Encoding`、`Vary`、MIME、实际响应字节，并比较解压后内容；只检查配置文件不能证明浏览器获得了压缩响应。

账户 handle 和自备 AI 规则的发布目录为 `/repo/colang/releases/profiles-20261109`。其 `backup` 保存迁移前数据库和 Compose；迁移 25 添加并回填 handle，迁移 26 强制每个账户最多一条启用的 AI 配置。平台环境密钥继续只留在后端，服务端资格判断决定能否使用；上线检查必须同时验证非付费拦截和个人配置流程，不能用给生产测试账户伪造支付记录的方式通过验证。

例句音频与多平台配置发布新增迁移 28、29：分别启用文本/TTS/ASR，以及候选音频、平台白名单。迁移 27 按此前移除豆包/火山引擎的要求清理退休平台凭据。上线前备份 PostgreSQL 与 Compose；新表和新增布尔列不要求丢弃用户数据。音频持久保存在 `SPACE_PATH/sentence-audio`，需继续挂载现有 `/data/colang`。默认白名单为 OpenAI、智谱、MiniMax，可在后台「句子发音审核」修改。
