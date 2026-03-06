# Agent Team 推进记录（重建版）

## 1. 记录规则

本文件只记录：

1. 阶段目标
2. 里程碑
3. Gate 结果
4. 阻塞与处置
5. 下一步动作

不记录逐条调试噪音。

## 2. 基线重启记录

### 2026-03-06：R0 重建启动

结论：

1. 方向统一为 `Spacebot-only`
2. 三系统链路降级为历史参考，不作为当前主链
3. 启动“从零基线重建”

已完成：

1. 路线文档重写
2. 主文档重写
3. 部署手册重写（待同步验证）
4. 启动脚本与配置模板重建（待执行验证）
5. `spacebot/scripts/gate-pr.sh` 已改为兼容 macOS 默认 Bash 3.2
6. `spacebot/scripts/preflight.sh` 与 `gate-pr.sh` 已增加 rustup toolchain 优先
7. Gate 回归通过：`preflight --ci` + `gate-pr --ci --fast`（428 tests passed）
8. 修复本轮 Gate 阻塞：
   - config 测试环境隔离补齐 `ANTHROPIC_AUTH_TOKEN`
   - metadata search 测试去除 embedding 模型下载依赖
9. 新增单仓闭环执行手册：`docs/agent_team/single_repo_closure.md`
10. 新增单仓闭环一键留痕脚本：`drafts/run-single-repo-closure.sh`

### 2026-03-06：R1 单仓闭环样例（Spacebot）

run_id：`single-20260306-174729`

证据目录：`docs/agent_team/runs/single-20260306-174729/`

Gate 结果：

1. Gate A：PASS（契约说明已记录）
2. Gate B：PASS（`preflight --ci` + `gate-pr --ci --fast`）
3. Gate C：PASS（本轮未配置 smoke，按空命令跳过）

### 2026-03-06：R2 / R3 单仓闭环复跑（Spacebot）

run_id：

1. `single-20260306-174913`
2. `single-20260306-174922`

证据目录：

1. `docs/agent_team/runs/single-20260306-174913/`
2. `docs/agent_team/runs/single-20260306-174922/`

Gate 结果：

1. 两轮均为 A/B/C 全 PASS
2. `gate-pr` 两轮均通过（`428 passed, 0 failed`）
3. 累计单仓闭环通过次数达到 3 次（R1 + R2 + R3）
4. 新增 4 仓执行清单模板：`docs/agent_team/workflow_web_4repos_checklist.md`
5. 新增 4 仓串行执行脚本：`drafts/run-4repo-closure.sh`
6. 4 仓执行器 dry-run 通过：`fourrepo-20260306-175155`
7. 4 仓真实路径预填清单：`workflow_web_4repos_checklist.aicoin_web.md`
8. 代理环境下 4 仓实跑：`fourrepo-20260306-180052`（collector test 失败）

## 3. 当前阻塞

当前阻塞（4 仓阶段）：

1. collector (`data-spider`) `go test ./...` 失败
2. 失败类型包含编译错误、Redis 依赖缺失、业务测试断言失败

## 4. 下一步

1. 切换到 4 仓串行闭环（collector -> shared-data -> api -> frontend -> integration-test）
2. 为 4 仓流程补每仓命令与通过标准清单
3. 执行首轮 4 仓闭环并沉淀证据目录
