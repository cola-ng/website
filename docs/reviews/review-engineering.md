# 工程、固件与验证评审

评审日期：2026-11-14（Asia/Shanghai）。以当前工作树为准；未修改业务源码。

## E01 · P1 · 两项数据库集成测试遗漏异步初始化，执行即 panic

- 证据：`website/crates/server/src/routing/auth/social.rs:673`、`website/crates/server/src/routing/learn/chat/streaming.rs:670` 调用 `crate::db::init(&config.database)` 未 `.await`。`website/crates/server/src/db.rs:30` 的初始化为 async，随后 `db.rs:70` 要求全局池已经设置。
- 触发：单独运行 `oauth_database_flow` 或 `chat_stream_database_flow` ignored 测试；`website/tests/run.ps1:72`、`:73` 正在运行这两项。
- 实测：仅设置隔离测试名所需的环境变量，数据库地址使用 `127.0.0.1:1/colang_qa`，两项都在连接数据库之前以 `horme pool should set` panic。该验证没有创建数据库或访问生产数据。
- 影响：一键 QA 在这两项处必然中断；普通 `cargo test --lib` 跳过它们，因此绿灯无法证明 OAuth/流式对话的数据库集成正确。
- 修复：等待初始化；在独立进程中运行依赖全局 OnceLock 的场景；将这类未使用 Future 的警告升级为错误。
- 验收：两项在真实隔离 PostgreSQL 上通过，数据库不可用时报告连接错误；CI 确实执行这两项。

## E02 · P1 · CI 没有运行现有数据库与浏览器集成回归

- 证据：`website/.gitea/workflows/ci.yml:30` 只运行后端默认 lib 测试；`:52` 只运行前端 host 测试。仓库已有 `website/tests/run.ps1` 和 `website/tests/README.md`，包含数据库、提供方替身、OAuth、真实 WASM 浏览器和响应式检查。
- 影响：E01 这种必然失败的集成测试可被默认 CI 放行。前端 host 测试还不会编译 `cfg(target_arch = "wasm32")` 内的页面逻辑；目前 WASM Clippy 会检查编译，但不验证页面行为。
- 修复：在 CI 提供隔离 PostgreSQL、假上游和 Chromium，拆出跨平台测试编排；先纳入鉴权、订阅、取消/流式、设备绑定等关键路径，再加入完整响应式回归。付费外部 AI 探针继续显式可选。
- 验收：PR 修改关键路径时对应集成测试会执行，故障能阻断合并；日志标出 passed/skipped 原因。

## E03 · P2 · 跨仓库依赖没有锁定到确定提交

- 证据：`website/crates/server/Cargo.toml:25` 通过相邻目录引用 amigo；`website/.gitea/workflows/ci.yml:20`、`:43`、`:69`，以及 `docker-build.yml:25`、`release.yml:26` checkout amigo 时没有 `ref`。
- 触发：amigo 默认分支推进后重跑同一 website 提交或发布标签。
- 影响：源码、协议与镜像内容可能不同；Cargo.lock 不会锁住外部 path dependency 的文件内容。测试 job 和构建 job 之间也存在版本漂移窗口。
- 修复：维护 amigo 提交锁定文件，各 job 和发布读取同一个 SHA；产物记录两个仓库 SHA、锁文件摘要及工具链。
- 验收：同一发布标签解析到相同的双仓库提交；升级协议通过显式变更更新锁定版本并跑兼容性测试。

## E04 · P2 · 固件 CI 没有编译真实硬件适配层

- 证据：`amigo/.github/workflows/ci.yml` 只在 Windows host 执行 test/check/clippy；`amigo/firmware/korvo2/src/lib.rs:4`、`:9`、`:11` 将 device、provisioning、wire 限制为 `target_os = "espidf"`。板级 C 与 ESP-IDF 链接也不属于这些 host 检查。
- 影响：状态机测试通过仍可能留下 FFI、TLS、NVS、驱动编译或链接错误。本次 host 绿灯不代表可烧录，更不代表声学或触摸体验通过。
- 修复：增加真实 Xtensa 编译与板型矩阵；将 host 状态机、目标编译、上板验收分成明确的三个门禁。板级 C 文本测试纳入自动化。
- 验收：Korvo-2/立创板都产生可校验 ELF、分区/模型清单；无法接硬件的 CI 只声明编译通过，硬件验收保留独立记录。

## E05 · P2 · 一部分固件脚本仍硬编码产物根路径

- 证据：`amigo/scripts/Setup-AmigoFirmware.ps1:7`、`Build-AmigoFirmware.ps1:18`、`Package-AmigoFirmware.ps1:2` 使用 `../target/verification/amigo/firmware`；新 `Build-AmigoBoard.ps1` 则通过 `Get-NormalCargoTarget` 查询实际固件 workspace。
- 影响：调用方使用正常 Cargo 自定义 target 配置时，工具、清单和固件包不在实际 workspace target 下；存在两套构建入口与产物定位约定，容易读取旧清单。
- 修复：统一从所属 workspace 的 `cargo metadata --no-deps --format-version 1` 取 `target_directory`，将工具与清单作为其子目录，明确支持的构建/打包入口；不修改 Cargo home、target 或 TEMP/TMP。
- 验收：在调用方提供的正常 target 配置下，build/package/flash 读取同一份清单，并拒绝板型、提交或分区不匹配的产物。

## E06 · P2 · 后端启动与库入口重复声明模块，测试入口与生产入口存在覆盖差异

- 证据：`website/crates/server/src/main.rs:1` 和 `src/lib.rs:1` 分别声明相同的 auth/config/db/routing/services 等模块，main 额外声明 tasks。CI 只执行 `cargo test --lib`，生产启动与后台刷盘任务不在这个入口内。
- 影响：相同模块在两个 crate target 重复编译，入口行为较难统一验证；后台任务逻辑容易遗漏测试。单纯的大文件不是缺陷，但 gateway、admin 等模块职责集中，也提高定位成本。
- 修复：由 lib 暴露 app 构造与任务生命周期，main 仅装配配置和启动；后台任务使用可注入的存储/时钟、可取消句柄，并增加停机及失败恢复测试。按会话、媒体、持久化职责逐步拆分，而非一次性重写。
- 验收：生产和集成测试使用相同 app 构造路径；停机、任务失败可被测试和观察。

## 本次实测

| 命令 | 结果 |
| --- | --- |
| amigo: `cargo test --locked -p amigo --all-features` | 10 passed |
| amigo: `cargo check --locked -p amigo --no-default-features --lib` | 通过 |
| amigo/firmware/korvo2: `cargo test --locked --features lichuang --lib --tests` | 36 passed |
| amigo/firmware/korvo2: `cargo clippy --locked --features lichuang --all-targets -- -D warnings` | 通过 |
| website: `cargo test --locked -p colang-front` | 3 passed |
| website: `cargo clippy --locked -p colang-front --target wasm32-unknown-unknown -- -D warnings` | 通过 |
| website: `cargo test --locked -p colang --lib` | 78 passed / 9 ignored；43 warnings |
| 两项 ignored 数据库测试的初始化检查 | 各 1 failed，均为未设置连接池 panic |

Cargo metadata 确认的产物目录分别为 `D:/Works/colang/website/target`、`D:/Works/colang/amigo/target`、`D:/Works/colang/amigo/firmware/korvo2/target`。未重定向缓存或临时目录，未执行 Docker 构建，未烧录硬件。
