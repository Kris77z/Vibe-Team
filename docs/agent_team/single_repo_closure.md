# 单仓闭环执行手册（Spacebot-Only）

## 1. 目标

把“首轮单仓闭环”变成可重复执行、可审计留痕的固定流程。

闭环包含 3 个阶段：

1. 阶段 A：契约冻结
2. 阶段 B：实现与仓内验证
3. 阶段 C：集成 smoke 与总结

## 2. 输入要求

执行前必须明确：

1. `TARGET_REPO`：目标仓绝对路径
2. `TARGET_BRANCH`：目标分支（可为空，默认当前分支）
3. `BUILD_CMD`：构建命令（可为空）
4. `TEST_CMD`：测试命令（可为空）
5. `SMOKE_CMD`：smoke 命令（可为空）
6. `CONTRACT_NOTE`：契约冻结说明（建议填写）

## 3. 执行命令

```bash
TARGET_REPO=/abs/path/to/repo \
TARGET_BRANCH=main \
BUILD_CMD='bun run build' \
TEST_CMD='bun run test' \
SMOKE_CMD='bun run test:smoke' \
CONTRACT_NOTE='冻结 /wallet 响应字段与错误码' \
/Users/applychart/Desktop/vibe-team/drafts/run-single-repo-closure.sh
```

说明：

1. 所有命令都在 `TARGET_REPO` 下执行
2. 执行产物会写到 `docs/agent_team/runs/<run_id>/`
3. 失败即停，`SUMMARY.md` 会标记失败阶段

## 4. 产物结构

```text
docs/agent_team/runs/<run_id>/
├── env.txt
├── contract.txt
├── 01_git_status.log
├── 02_build.log
├── 03_test.log
├── 04_smoke.log
└── SUMMARY.md
```

## 5. Gate 通过标准

1. A Gate：`contract.txt` 非空且关键信息完整
2. B Gate：`BUILD_CMD`、`TEST_CMD` 返回码均为 0
3. C Gate：`SMOKE_CMD` 返回码为 0（若配置）

## 6. 结果回填

每次执行后，至少把以下内容回填到 `progress.md`：

1. `run_id`
2. 仓库与分支
3. Gate 结果（A/B/C）
4. 失败阶段（若失败）
5. 下一步动作
