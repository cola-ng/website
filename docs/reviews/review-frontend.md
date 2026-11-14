# 前端专项评审（2026-11-14）

范围：`website/front` Rust/WASM 与浏览器适配层、`website/miniprogram`。评审基于当前工作树；未改动产品代码。本次没有启动浏览器进行视觉验收，不对实际排版、色彩、读屏兼容性作实测结论。

## 已发现问题

### F1 · P1：录音授权等待期间离开页面，迟到授权仍会启动麦克风

- 位置：[platform.js:64](D:/Works/colang/website/front/src/browser/platform.js:64)、[platform.js:118](D:/Works/colang/website/front/src/browser/platform.js:118)、[platform.js:177](D:/Works/colang/website/front/src/browser/platform.js:177)。
- 触发：调用 `startRecording()` 后 `getUserMedia()` 尚未完成，此时触发 `pagehide`（包括可能进入 BFCache 的导航），随后授权 Promise 返回。
- 根因：`cancelRecording()` 清除 owner 并尝试停止当前 stream；但是 await 后没有验证 owner 是否仍有效，迟到的 stream 随即赋给全局变量并执行 `recorder.start()`。pagehide 时 stream 尚不存在，无法清理它。Rust 的页面清理可以在部分 SPA 场景补救，但不能使适配器自身的 pagehide 路径正确。
- 影响：已离开的页面仍可开始录音，最长到 60 秒计时器触发；对麦克风隐私与用户预期有直接影响。
- 验证：执行实际 platform.js 的 Node VM 沙箱（仅去除 import/export），使用延迟 getUserMedia、MediaRecorder 和 pagehide stub，结果 `recordingStartedAfterPagehide=1, tracksStopped=0`。这是逻辑复现，未做真实浏览器权限/BFCache测试。
- 改进：await 后先核对本次 owner/取消 generation；失效时停止这次返回 stream 的所有 track 并退出，避免写入共享状态。针对实际 BFCache 与权限弹窗补浏览器回归。
- 验收：先触发 pagehide 再 resolve 授权，MediaRecorder.start 调用为 0、返回流的 track 全部停止；普通授权与取消不受影响。

### F2 · P2：浏览器旧会话的迟到 401 会清除新登录

- 位置：[mod.rs:114](D:/Works/colang/website/front/src/browser/mod.rs:114)、[mod.rs:145](D:/Works/colang/website/front/src/browser/mod.rs:145)。
- 触发：旧凭证请求仍在途；用户退出并重新登录或在其他 tab 更新凭证；随后旧请求返回 401。
- 根因：请求发送的 token 没有被保存用于响应时比较，任意非 login 的 401 都无条件 `clear_auth()`。Resource 的生命周期检查发生在 request 返回之后，挡不住此全局副作用。
- 影响：新会话被旧响应登出，当前编辑/语音状态被打断。成功响应也没有统一的会话 epoch 校验。
- 证据等级：静态确定分支与可构造交错；尚未执行浏览器竞态复现。小程序 `services/api.js` 已具备 sentToken/identity 防护，可复用同一策略。
- 改进：发送时记录 token 与 session revision，401 仅失效匹配的当前会话；成功返回同样拒绝跨会话数据。
- 验收：A 请求延迟，切换至 B，再令 A 返回 401，B 凭证与界面保留；A 的成功响应不能写入 B 页面。

### F3 · P2：小程序从后台返回会重置复习答案和本轮进度

- 位置：[review/index.js:36](D:/Works/colang/website/miniprogram/pages/review/index.js:36)、[review/index.js:54](D:/Works/colang/website/miniprogram/pages/review/index.js:54)。
- 触发：答题过程中临时切后台、查看其他页面，之后返回同一 review 实例且账号没有变化。
- 根因：每次 onShow 都执行 load，成功后无条件把 index、answer、completed、evaluation、saved 清零并重取题组。
- 影响：未提交答案和发音评估丢失，用户需重复操作；已提交的后端记录不会回滚，但本轮完成计数归零。
- 验证：通过仓库现有 tests/helpers.cjs 构造 review 页面，把 index/completed 设为 4、answer 设为 hello；执行 onHide/onShow 并返回题组，实际变成 `index=0, answer="", completed=0`。
- 改进：仅首次进入、明确点击新一轮或会话身份变化时重建题组；普通恢复保留题目、答案、评估，计时单独暂停/恢复。
- 验收：中途切后台返回仍是原题原答案；提交过的项不会因恢复再次变为可提交；账号切换仍清空私有数据。

### F4 · P2：设备 AI 配置草稿在切换设备详情 tab 后丢失

- 位置：[amigo.rs:286](D:/Works/colang/website/front/src/browser/amigo.rs:286)、[amigo.rs:291](D:/Works/colang/website/front/src/browser/amigo.rs:291)、[device_ai.rs:119](D:/Works/colang/website/front/src/browser/device_ai.rs:119)。
- 触发：编辑设备专用 API Key、模型或音色，未点“保存设备 AI 配置”，切换到调用日志/对话记录，再回配置。
- 根因：AI panel 位于 `when(tab == settings)` 分支；draft 与各字段 cage 归属于 panel 子 scope。Glory 0.7.0 的 Branch.update 明确 drop 旧 mounted 分支；重新进入会重新 GET 已保存配置并创建草稿。其他部分采用常驻容器保存状态，但 AI panel 未采用。
- 影响：填写内容静默消失，且同一页不同区域的草稿保留行为不一致。
- 证据等级：已检查依赖实际分支销毁实现的静态结论；未执行浏览器切 tab 实测。
- 改进：将草稿提升至设备 editor scope，或采用常驻隐藏 panel；离开页面存在 dirty 状态时提示。明确两组保存按钮的作用域，并提示尚未保存的区域。
- 验收：填写 API Key 与模型后切换所有详情 tab，返回值不变且未发出保存；明确放弃/保存后 dirty 状态清除。

## 设计与性能改进建议（不作为已证实缺陷计数）

1. SPA 路由目前执行 pushState 与滚动，但没有显式页面标题更新、导航后主标题聚焦：[routing.rs:31](D:/Works/colang/website/front/src/browser/routing.rs:31)，HTML title 固定。建议为各路由设标题，导航完成后按交互方式管理焦点，验证键盘与读屏能直接获知新页，避免 header 重建后焦点丢失。需真实浏览器/读屏复核。
2. Amigo 列表与日志使用整个 JSON 字符串作为 key，且每 5 秒轮询。字段变化会销毁重建整张卡/行（amigo.rs:124、309），可能使聚焦控件失焦并增加渲染成本；建议以稳定 ID 为 key，用响应式数据更新展示字段。验收需覆盖 focused 详情按钮遇到状态更新的情况。
3. 分开“配置可编辑”和“配置已应用”的反馈，给 dirty 区域增加明确标记；保留现有 revision 同步提示，避免用户把设备在线等同于新配置已生效。
4. 对 320/375/768/1280 宽度、200% 字体缩放、软键盘、横屏、暗背景/高对比模式做真实视觉检查；本报告未声称已经完成这些检查。

## 本次验证与覆盖缺口

- `npm test`（website/miniprogram）：28/28 通过。
- 额外无落盘复现：复习页面恢复重置、浏览器录音授权/pagehide 交错。
- 未运行微信开发者工具或真机、浏览器视觉/读屏验收，也未验证上游语音实际响应。
- 现有小程序测试覆盖晚到响应、账户切换、录音清理、流式分片等，质量基础较好；建议新增上述跨页面恢复与授权竞态场景，而不是仅增加静态断言。
