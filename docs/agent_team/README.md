# Agent Team 主文档

## 1. 当前目标（2026-03）

当前目标已经从“主链是否可跑”切换为“主链是否可稳定重复运行”。

具体目标：

1. 在权限分离模型下，让 `feature-dev` 可重复 unattended 跑到 terminal
2. 降低对人工恢复动作（尤其 `kickstart`）的依赖
3. 在稳定性达标后，进入结果输出契约收敛
4. 基于稳定基线，落地多仓 Web 闭环流程（4 库 + 测试）

## 2. 架构基线

核心架构不变：

1. `Spacebot`：统一前台入口、任务触发、状态摘要与结果展示
2. `Antfarm`：工作流编排
3. `OpenClaw`：执行底座

说明：

- `Spacebot` 用来解决“前台可持续对话 + 后台异步施工”
- `Antfarm Dashboard` 是辅助调试视图，不是主入口

## 3. 文档地图

后续请按分文档维护，不再把所有内容堆进单一 runbook。

1. 架构与原则
   - [agile_agent_team_architecture.md](/Users/applychart/Desktop/vibe-team/docs/agile_agent_team_architecture.md)
2. 部署、联调、运行排障
   - [deployment_and_integration_runbook.md](/Users/applychart/Desktop/vibe-team/docs/deployment_and_integration_runbook.md)
3. 推进过程与关键节点（阶段记录）
   - [progress.md](/Users/applychart/Desktop/vibe-team/docs/agent_team/progress.md)
4. 当前多仓闭环流程（4 库 + 测试）
   - [workflow_web_4repos.md](/Users/applychart/Desktop/vibe-team/docs/agent_team/workflow_web_4repos.md)

## 4. 当前执行顺序

1. 继续 unattended soak，验证稳定可重复 terminal
2. 达到稳定性基线后，收敛 terminal 输出契约
3. 基于契约稳定版，推进多仓闭环 workflow
4. 稳定后再考虑分工流程扩展（如 `feature-dev-split`）

## 5. 阶段门槛

进入“输出契约收敛”前，至少满足：

1. 连续 3 次 unattended run 到 terminal
2. 无需人工 `step claim`
3. 不依赖每轮 `kickstart` 才能推进
4. `Spacebot` 面板能稳定展示 terminal 结果
