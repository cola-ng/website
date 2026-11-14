# 服务端性能、并发、数据一致性专项评审

评审日期：2026-11-14。范围：`website` 当前工作树（含未提交变更）；静态调用链与迁移核对，未连接生产数据库、未执行负载测试，未改业务源码。以下“确定”指代码路径成立，不代表已在运行环境复现。

## R-PERF-01 / P1 / 学习时长 flush 失败后永久丢失

- 证据：`D:/Works/colang/website/crates/server/src/services/study_time.rs:96` 通过 `mem::take` 清空 pending；`D:/Works/colang/website/crates/server/src/tasks/study_time_flush.rs:40` 先取走全部增量，105 行 upsert 失败只写日志，115 行仍输出 completed 并返回 Ok。
- 触发：任一 flush 遇到数据库断连、池超时或单行写入失败。失败条目不会出现在下一轮 pending，因此无法重试。若进程在清空后、写完前退出，尚未写入项同样丢失。
- 影响：学习统计少计；上层的“flush失败”错误处理实际上不能接收到这些写入错误。普通退出也没有最终 flush：`main.rs:77` 直接 serve，后台任务没有保存 JoinHandle/关闭协议。
- 建议：失败项按加法合并回 pending（不能覆盖 flush 期间的新增量），传播失败统计；关闭时停止接收心跳、等待在途 flush 并持久化剩余增量。若要求跨进程崩溃不丢，使用持久化事件/队列并设计批次幂等键，而不能仅靠内存重试。
- 验收：预置多用户增量，在第 k 个写入注入失败；恢复后各用户累计值严格等于输入，成功记录不重复；覆盖 flush 与新心跳交错及关闭期间 flush。

## R-PERF-02 / P1 / 固定每请求加 20 秒，心跳频率直接改变学习时长

- 证据：`D:/Works/colang/website/crates/server/src/services/study_time.rs:53` 仅检查 gap < 120 秒与页面类型相同；72 行每次增加固定 20 秒，没有最小间隔、实际经过时间或多标签去重。入口为 `routing/learn/heartbeat.rs:35`。
- 触发：同一账号开两个同类页面，各自每 20 秒心跳；或者连续快速发送合法心跳。第一次以后的请求无论间隔多短都增加 20 秒。即使没有恶意操作，多标签也会重复计时。
- 影响：统计失真，依赖时长的学习趋势/奖励不可靠；客户端节流也不能作为服务端正确性保障。
- 建议：按用户的单调时钟计算真实可计时间，设合理上限并保证同一时间段只计一次；明确页面切换与跨日期归属；接口速率限制作为额外保护。
- 验收：用可注入时钟覆盖同时间戳 100 次心跳、双标签交错、20/60/120 秒间隔和跨午夜；总计不超过实际有效经过时间。

## R-PERF-03 / P1 / 订阅先查后写，无数据库约束保证单一有效订阅

- 证据：`D:/Works/colang/website/crates/server/src/routing/payment.rs:295` 查询 existing，306 行判断，325/347 行分别直接 insert；整段未使用事务或账号行锁。`migrations/00000000000017_payment_system/up.sql:86` 定义订阅表，104–107 行仅普通索引；全 migrations 未找到有效订阅的唯一约束。
- 触发：同用户同时提交两个 subscribe，请求均在插入前读到无 active/trial。
- 影响：生成多条有效订阅；current 按创建时间选一条，而 cancel（400 行）无序 first 后仅取消一条，用户操作后仍有另一有效记录。后续扣费/权益逻辑会面对歧义。
- 建议：迁移前清理重复记录；按业务定义建立 `user_id WHERE status IN ('active','trial')` 部分唯一索引，在订阅事务中锁账号/串行化状态转换，唯一冲突返回 409；结合到期状态归一化处理，避免过期记录挡住新订阅。
- 验收：同用户并发 20 次订阅，仅一次创建，其余返回明确冲突；取消后无残留有效记录；不同用户之间不互相阻塞。

## R-PERF-04 / P1 / 到期状态未统一处理，过期试用仍被视作订阅且阻止重新订阅

- 证据：`D:/Works/colang/website/crates/server/src/routing/payment.rs:205` current 仅查 active/trial，231 行返回 has_subscription=true，即使 days_remaining=0；295 行 subscribe 同样据此拒绝。`D:/Works/colang/website/crates/server/src/hoops/subscription.rs:39` 的通用订阅鉴权没有 expires_at 条件；`tasks/mod.rs:8` 仅启动学习时长任务，没有到期归档任务。
- 触发：trial/active 的 expires_at 已过去，status 仍为原值。
- 影响：展示与实际日期矛盾；用户无法直接续订。通用 require_subscribed 检查实现也会接受过期记录，但当前路由未调用它，此部分属于潜在复用缺陷，不能声称当前付费路由已被绕过。`services/mod.rs` 平台密钥另有到期/已付款校验，不能推导为可无限免费使用平台 AI。
- 建议：统一有效订阅谓词为状态 + 有效期，并让查询、鉴权与订阅状态转换复用；后台归档只负责维护，不应成为鉴权正确性的前提。
- 验收：构造昨天到期、未来到期、无到期日三类订阅，在 current、subscribe、require_subscribed 中行为一致；平台密钥测试继续保持独立付款门控。

## R-PERF-05 / P2 / 钱包首次读取存在并发初始化竞争

- 证据：`D:/Works/colang/website/crates/server/src/routing/payment.rs:64` 钱包先 select，不存在则 insert；`migrations/00000000000017_payment_system/up.sql:11` 对 user_id 有 UNIQUE，但插入路径没有 ON CONFLICT 或冲突后重读。
- 触发：新账号从两个标签/并行组件首次请求 wallet，两者都读到不存在。
- 影响：其中一个请求唯一冲突并返回 500；不会创建重复钱包，但会出现可避免的初次访问失败。
- 建议：INSERT ON CONFLICT DO NOTHING 后统一读取，或用返回行的原子 upsert。
- 验收：空钱包账号并发 GET 20 次全部成功，数据库仅一行。

## 优化项 / P2 / 汇总接口全量读取词汇 mastery 值

`D:/Works/colang/website/crates/server/src/routing/learn/summary.rs:139` 为求均值将用户全部 mastery_level 载入 Vec，再在应用内求和；同接口前后多次独立扫描词汇表。确定存在 O(N) 数据传输/应用内存，不宣称已有性能故障。可改数据库 AVG，并把 count、sum、条件 count 合并为一次聚合，保持 NULL 语义。验收时在 1 千/10 万词汇数据上对照结果、EXPLAIN (ANALYZE, BUFFERS)、返回字节量与 p95，不预设收益百分比。

## 已见到的正向设计

Amigo 网关已有消息尺寸上限、有限 mpsc 通道、发送背压超时和 Drop 中 abort；设备租约通过 UUID fencing 防止旧连接释放新租约；profile 修改使用事务及 FOR UPDATE。应保留并在负载/取消场景扩展测试，不能以全局锁或无界队列代替这些机制。

## 后续性能基线建议（待测，不作为现存缺陷）

用本地可回放的 mock AI 服务建立并发对话/慢客户端/上游超时/数据库短暂断连基线；记录首音频时间、首 token 时间、p50/p95/p99、每在线设备内存、DB pool 等待时间、队列长度和取消后存活任务。优化前后使用相同输入、并发与构建配置。现有代码证据不足以判断生产吞吐上限。
