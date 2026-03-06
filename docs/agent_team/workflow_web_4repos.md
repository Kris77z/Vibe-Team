# 多仓 Web 闭环流程（4 仓 + 测试，重建版）

## 1. 目标

在 `Spacebot-only` 主链下，跑通一条可重复的 4 仓串行闭环：

1. `collector-repo`
2. `shared-data-repo`
3. `api-repo`
4. `frontend-repo`
5. `integration-test`

## 2. 执行顺序

```text
collector -> shared-data -> api -> frontend -> integration-test -> summary
```

默认串行，先稳后快。

## 3. 每仓输入

每仓至少要有：

1. 绝对路径
2. 目标分支
3. 构建命令
4. 测试命令
5. 通过标准

推荐先填写：
[workflow_web_4repos_checklist.md](./workflow_web_4repos_checklist.md)

推荐执行器：
`/Users/applychart/Desktop/vibe-team/drafts/run-4repo-closure.sh`

## 3.1 流程先行（暂不改业务仓）

在你未下达“改具体仓代码”指令前，只做以下动作：

1. 固定 5 仓路径、分支、命令
2. 校验分支与命令可执行性
3. 先跑非阻塞阶段（必要时跳过已知阻塞仓）
4. 输出标准化 `SUMMARY.md` 证据

约束：流程阶段可推进，但不改业务仓源码。

## 3.2 跳过阶段参数

执行器支持环境变量 `SKIP_STAGES`（逗号分隔）：

1. 可写 stage name：`collector,shared-data,api,frontend,integration-test`
2. 也可写 stage key：`collector,shared,api,fe,integration`
3. 示例：`SKIP_STAGES='collector'`（跳过 `collector`，继续后续阶段）

## 4. 阶段 Gate

1. Collector Gate
   - 契约产出齐全（字段/类型/样例/错误码）
2. Shared Gate
   - 公共模型与转换结果可被 API 仓读取
3. API Gate
   - 契约测试通过，错误处理符合约定
4. FE Gate
   - 关键页面渲染与核心交互 smoke 通过
5. Integration Gate
   - 端到端关键路径通过

## 5. 失败处理

1. 失败即停在当前阶段
2. 标记失败仓与失败步骤
3. 给出可复现命令与错误摘要
4. 修复后从失败阶段重跑，不回滚已通过阶段

## 6. 完成定义

只有 `integration-test` Gate 通过，且存在完整 summary，才算本轮闭环完成。
