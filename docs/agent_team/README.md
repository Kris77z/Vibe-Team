# Agent Team 主文档（重建版）

## 1. 当前目标

当前唯一目标：以 `Spacebot-only` 路线稳定交付可重复闭环。

执行顺序：

1. 跑通单仓闭环
2. 连续 3 次稳定通过
3. 扩展到 4 仓串行闭环
4. 再评估是否需要恢复编排底座

## 2. 当前架构基线

主链仅保留：

1. `Spacebot`

边界约束：

1. 成功标准是“闭环可交付”，不是“平台完整度”
2. 质量控制依赖阶段 Gate，不依赖复杂调度系统
3. 优先减少系统边界与联调变量

## 3. 文档地图

1. 路线与原则：
   - [spacebot_only_direction.md](./spacebot_only_direction.md)
2. 推进记录：
   - [progress.md](./progress.md)
3. 4 仓闭环流程：
   - [workflow_web_4repos.md](./workflow_web_4repos.md)
   - [workflow_web_4repos_checklist.md](./workflow_web_4repos_checklist.md)
   - [workflow_web_4repos_checklist.aicoin_web.md](./workflow_web_4repos_checklist.aicoin_web.md)
4. 单仓闭环执行手册：
   - [single_repo_closure.md](./single_repo_closure.md)
5. 部署与运行手册：
   - [deployment_and_integration_runbook.md](../deployment_and_integration_runbook.md)

## 4. 阶段门槛

进入“4 仓 + 测试”前必须满足：

1. 单仓闭环连续通过 3 次
2. 每个阶段有机械可判断 Gate
3. 失败能定位到明确阶段并可人工恢复

## 5. 当前状态

更新时间：2026-03-06。

状态：已进入 `Spacebot-only` 重建阶段，单仓闭环已累计 3 次稳定通过，下一步切换 4 仓串行闭环。
