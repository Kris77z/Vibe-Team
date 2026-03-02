# Spacebot x Antfarm 挂载位置草案

本文件用于回答一个具体问题：

**`Spacebot -> Antfarm` 适配层，应该挂在 `Spacebot` 的哪一层？**

结论先行：

1. **不要挂在 `agent/channel.rs`**
2. **不要挂在 worker runtime 内核**
3. **优先挂在 API/service 层**
4. **第一版通过 API handler + integration service + 现有 SSE 事件总线完成**

---

## 1. 推荐挂载结构

建议后续真实接入时采用下面这组模块边界：

```text
spacebot/src/
├── integrations/
│   └── antfarm.rs          # Antfarm adapter/service
├── api/
│   └── antfarm.rs          # HTTP handlers
└── api/state.rs           # 增加 antfarm service / run bindings / event helpers
```

推荐职责划分：

1. `integrations/antfarm.rs`
   - 对外提供统一接口
   - 封装 workflow trigger、run summary、final result 读取
   - 屏蔽 CLI / HTTP / mock 差异

2. `api/antfarm.rs`
   - 提供 HTTP endpoint 给 Web UI 或前端调用
   - 不做复杂业务逻辑
   - 只调用 service 并返回 JSON / SSE 事件

3. `api/state.rs`
   - 保存 adapter 实例
   - 保存 `conversationId <-> runId` 绑定
   - 通过现有 `ApiEvent` 流把 run 状态变化发给前端

---

## 2. 为什么不应该挂在 `channel` 层

`spacebot/src/agent/channel.rs` 的职责是：

1. 用户对话
2. 分支和 worker 生命周期
3. 对话内状态维护
4. LLM retrigger / relay

而 `Antfarm` workflow 是一个外部编排系统，语义上更接近：

1. 外部任务系统
2. 长任务 orchestration
3. 非 channel-local 的运行记录

如果把它直接塞进 `channel.rs`，会出现几个问题：

1. channel 层会知道太多外部 workflow 细节
2. worker / branch / antfarm run 三种运行体会混在一起
3. 后面做非 webchat 场景复用时会变得很难抽离

因此，`channel` 层最多只应该：

1. 发起 trigger
2. 接收摘要结果
3. 向用户回传消息

不应该直接承担 `Antfarm` 适配职责。

---

## 3. 为什么不应该挂在 worker runtime 内核

`Spacebot` 现有 worker 体系已经有自己的：

1. `ProcessEvent`
2. `StatusBlock`
3. `worker_runs`
4. transcript
5. SSE 映射

这些是给 `Spacebot` 自己的 worker / branch / cortex 用的。

如果把 `Antfarm` run 直接硬塞进这套 worker runtime，会产生语义错位：

1. `Antfarm run` 不是 `Spacebot worker`
2. `Antfarm step` 不是 `Spacebot tool call`
3. `Antfarm event` 不是 `Spacebot transcript`

所以第一版最合理的做法是：

1. 复用 `ApiEvent` 总线
2. 不复用 `worker_runs` 作为持久化表
3. 把 `Antfarm` 看成“外部 workflow source”

这样不会污染现有 worker 抽象。

---

## 4. 为什么 API/service 层最合适

当前 `Spacebot` 已经有几条现成能力非常适合复用：

1. `api/state.rs`
   - 已经是全局 API 状态中心
   - 已经管理 event bus、channel status、webchat adapter、cortex chat session

2. `api/system.rs`
   - 已经提供全局 SSE `/api/events`

3. `api/webchat.rs`
   - 已经有“发消息 -> 通过 SSE 回响应”的路径

4. `api/cortex.rs`
   - 已经有“SSE 流式响应”的 handler 范式

这说明：

`Antfarm` 集成应该被视作一个新的 API-backed capability，而不是 agent runtime 的一部分。

---

## 5. 第一版推荐的接入点

### A. service 层

新增一个 service，例如：

```rust
pub trait AntfarmService {
    async fn trigger_workflow(...);
    async fn get_run_summary(...);
    async fn get_final_result(...);
}
```

它内部可以有三种实现：

1. `CliLauncher + DashboardReader`
2. `MockAdapter`
3. 未来可能的 `NativeHttpAdapter`

### B. API 层

新增几个 endpoint 即可：

1. `POST /api/antfarm/runs`
   - 触发 workflow

2. `GET /api/antfarm/runs/:id`
   - 返回 run summary

3. `GET /api/antfarm/runs/:id/result`
   - 返回 final result

4. 可选 `GET /api/antfarm/runs/:id/events`
   - 返回已经整理后的摘要事件，而不是原始底层事件

### C. SSE 层

不用重造 SSE endpoint。

直接复用：

- `/api/events`

做法是：

1. 新增 `ApiEvent` 变体，例如：
   - `WorkflowRunStarted`
   - `WorkflowRunUpdated`
   - `WorkflowRunCompleted`
   - `WorkflowRunFailed`
2. 当轮询器或 webhook 收到更新时，通过 `ApiState::send_event(...)` 发到现有 event bus

这样前端不需要接第二套 SSE。

---

## 6. 现有代码里最适合复用的位置

### `spacebot/src/api/state.rs`

最适合新增：

1. `antfarm_service`
2. `conversation_run_bindings`
3. `run_latest_status_cache`
4. `run_final_result_cache`

理由：

它已经是 API 的共享状态中心，扩展最自然。

### `spacebot/src/api/server.rs`

最适合新增：

1. `/api/antfarm/runs`
2. `/api/antfarm/runs/:id`
3. `/api/antfarm/runs/:id/result`

理由：

这里已经是所有 HTTP 路由的注册中心。

### `spacebot/src/api/system.rs`

不建议新增新 endpoint。

理由：

已有 `/api/events` 足够用。只需要在 event payload 中补新的事件类型。

### `spacebot/src/api/webchat.rs`

不建议直接塞业务逻辑。

理由：

它当前职责很单纯，就是：

1. 注入消息
2. 读取会话历史

把 `Antfarm` 逻辑塞这里，会把 webchat transport 和 workflow orchestration 绑死。

---

## 7. 第一版推荐的执行链路

推荐链路：

```text
Web UI
-> POST /api/antfarm/runs
-> Antfarm service trigger
-> shell: antfarm workflow run ...
-> save run binding
-> emit ApiEvent::WorkflowRunStarted
-> background poller reads Antfarm dashboard JSON
-> emit ApiEvent::WorkflowRunUpdated / Completed / Failed
-> Web UI listens on existing /api/events SSE
```

这里最重要的点是：

1. trigger 走 API
2. 读取走现有 dashboard JSON
3. 推送走现有 `/api/events`

这样新代码最少。

---

## 8. 第二版再考虑的事情

下面这些都不该放在第一版：

1. 把 `Antfarm run` 映射成 `worker_runs`
2. 为 `Antfarm` 单独做一套 SSE server
3. 在 `channel.rs` 里直接内嵌 workflow orchestration
4. 把 dashboard 原始 step output 全量透传给前端
5. 做完整 dashboard iframe 嵌入

这些都属于后续增强，不是最小可集成路径。

---

## 9. 和当前 draft adapter 的对应关系

当前已有开发稿：

- [adapter.ts](/Users/applychart/Desktop/vibe-team/drafts/spacebot-antfarm/adapter.ts)

它对应的是 service 层协议草案，而不是最终运行时模块。

因此后续迁移关系应是：

1. 先把 `adapter.ts` 中的类型和方法边界迁移为 Rust service trait
2. 再把 `MockSpacebotAntfarmAdapter` 迁移为 Rust 侧 test/mock implementation
3. 最后把 `AntfarmDashboardReader` 的行为迁移为 Rust 侧 HTTP reader

不要把这个 TypeScript 草稿直接视为生产代码来源，它只是接口边界样板。
