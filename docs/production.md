# cola.ng 部署说明

服务器通过 `root@cola.ng` 管理。现有应用的 Compose 位于 `/repo/colang/deploy-prod`，仅更新 `colang_api` 和 `colang_wui`；PostgreSQL 及同机其他服务保持原有部署。

- API 容器内部端口为 `6019`，主机端口为 `6019`。
- 前端通过主机端口 `6018` 提供服务；容器设置 `API_UPSTREAM=colang_api:6019`。
- OAuth 环境变量放在 Compose 目录内权限为 `600` 的 `.oauth.env`，通过后端的 `env_file` 加载。该文件不提交仓库，也不作为镜像构建上下文。
- 显式设置 `DOUBAO_CHAT_MODEL=doubao-seed-2-0-mini-260428`，替代已关闭的旧默认模型。

生产前端使用与 `front/Dockerfile` 相同的 Nginx 运行时模板。带内容哈希的 WASM、JS、CSS 使用长期不可变缓存；发布新版本时保留旧哈希资源，避免已打开的页面在切换期间加载失败。HTML 使用 `Cache-Control: no-cache`。

外层 Caddy 的对应路由为：

```caddyfile
cola.ng {
    encode zstd gzip
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

此次发布目录为 `/repo/colang/releases/oauth-20261109`，其中 `backup` 保存变更前 Compose、Caddy 配置及数据库备份，权限仅限 root。更新镜像前保存当前 Compose 和环境文件；API 与前端通过健康检查后才完成切换。保留上一版 `colang-api:oauth-20261109`、`colang-wui:oauth-20261109` 镜像供回退。回退代码时无需回退本次仅新增可空验证时间字段的迁移。

不要将完整 `docker inspect`、`docker compose config` 或环境文件输出到公开日志，它们可能包含密钥。验证压缩时请求真实 WASM URL，检查 `Content-Encoding`、`Vary`、MIME、实际响应字节，并比较解压后内容；只检查配置文件不能证明浏览器获得了压缩响应。
