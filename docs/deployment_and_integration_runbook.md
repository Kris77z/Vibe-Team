# Vibe-Team 部署与联调手册

> 文档拆分说明：主入口见 [docs/agent_team/README.md](/Users/applychart/Desktop/vibe-team/docs/agent_team/README.md)。  
> 本文聚焦“部署、联调、运行排障”；推进过程与关键节点请写入 [docs/agent_team/progress.md](/Users/applychart/Desktop/vibe-team/docs/agent_team/progress.md)。

## 1. 适用范围

这份文档用于在另一台 Mac 上部署并联调当前方案中的三套系统：

- `OpenClaw`：执行底座
- `Antfarm`：工作流编排器
- `Spacebot`：最终前端与统一入口

目标不是把 `Antfarm Dashboard` 当主界面，而是让用户最终主要在 `Spacebot` 中：

- 触发 workflow
- 查看运行状态
- 查看最终结果

`Antfarm Dashboard` 只作为辅助调试和运维观测面板。

---

## 2. 当前实现边界

在开始部署前，先明确当前代码已经做到什么、还没有做到什么。

已完成：

- `Spacebot` 已提供 Antfarm API：
  - `POST /api/antfarm/runs`
  - `GET /api/antfarm/runs/{run_id}`
  - `GET /api/antfarm/runs/{run_id}/result`
  - `GET /api/antfarm/conversations/{conversation_id}/runs`
- `Spacebot` 已通过现有 `/api/events` SSE 总线推送 workflow 生命周期事件：
  - `workflow_run_started`
  - `workflow_run_updated`
  - `workflow_run_completed`
  - `workflow_run_failed`
- `Spacebot` 已能：
  - 触发 `antfarm workflow run ...`
  - 轮询 Antfarm Dashboard JSON API
  - 在前端显示 workflow 面板
  - 在 WebChat 面板中通过 `Run Workflow` 直接启动 workflow
  - 在普通 channel 详情页中通过 `Run Workflow` 直接启动 workflow
  - 在浏览器中按 agent 保存多个项目 preset，复用 `repo/worktree/workflow` 配置
  - 在刷新后恢复会话绑定的 workflow runs
  - 在 `Spacebot` 重启后恢复已持久化的 run binding，并对未结束 run 重挂 poller

当前限制：

- 真实联调还没有在目标 Mac 上做过一次完整端到端验证
- `Spacebot` 里“通过自然语言自动触发 workflow”的聊天动作还没正式接入，当前稳定入口是 API
- `Final result` 的 `changes/tests/review_decision/branch/pr_url` 仍是 best-effort 提取，依赖 Antfarm 最后一步输出格式
- 当前状态同步是 `polling`，不是 webhook push 或原生 step-level SSE 桥接
- `Run Workflow` 里的项目 preset 当前只保存在浏览器本地，不会自动同步到其他浏览器或其他机器

---

## 2.1 联调准备结论

到当前阶段，可以把准备联调这件事收敛成下面这句话：

- `OpenClaw` 已是部署机前提
- `Spacebot` 是最终操作前端
- `Antfarm` 负责真正执行 workflow
- 第一次联调优先走 `Spacebot UI -> Run Workflow -> Antfarm -> OpenClaw`

也就是说，后续联调时优先使用 `Spacebot` 现有入口，不再把 curl 当成默认操作方式。

---

## 2.2 当前文档的标准部署口径

自 `2026-03-03` 起，这份文档对 `vibe-team` 的标准部署口径统一为：

- 源码 checkout 可放在：`$HOME/Desktop/Dev/Vibe-Team`
- 运行时实例根应放在：`$HOME/instances/vibe-team`
- `OpenClaw Gateway` 标准端口：`18889`
- `Antfarm Dashboard` 标准端口：`3333`
- `Spacebot` 标准端口：`19898`

如果前文的通用示例与第 `21` 节实机落地结果冲突，以这套实例根模型和第 `21` 节为准。

---

## 3. 推荐拓扑

推荐把三者部署在同一台 Mac，同机本地互通：

```text
浏览器
  -> Spacebot UI / API        http://127.0.0.1:19898
  -> Spacebot SSE             http://127.0.0.1:19898/api/events

Spacebot
  -> Antfarm CLI             /Users/kris/instances/vibe-team/bin/antfarm-vibe-team workflow run ...
  -> Antfarm Dashboard API   http://127.0.0.1:3333/api/...

Antfarm
  -> OpenClaw state/config   $HOME/instances/vibe-team/state + config/openclaw.json
  -> OpenClaw runtime        gateway 监听 127.0.0.1:18889
```

当前标准端口：

- `Spacebot API/UI`: `19898`
- `Antfarm Dashboard`: `3333`
- `OpenClaw Gateway`: `18889`

建议第一版全部只监听本机回环地址，不要直接暴露公网。

---

## 3.1 开发机通过 Tailscale 访问部署机

当前标准使用方式不是“开发机运行一部分、部署机运行一部分”，而是：

- `部署机` 运行完整后端链路：
  - `Spacebot`
  - `Antfarm`
  - `OpenClaw`
  - `target-project`
- `开发机` 只作为远程访问端：
  - 打开部署机上的 `Spacebot UI`
  - 通过部署机上的 `Spacebot` 触发 workflow
  - 在开发机浏览器里接收 SSE 状态更新

推荐通过 `Tailscale` 建立开发机到部署机的访问链路。

### 推荐方式：Tailscale + SSH 本地端口转发

第一版最稳的方式仍然是让 `Spacebot` 继续监听部署机本地回环地址：

- `Spacebot`: `127.0.0.1:19898`
- `Antfarm Dashboard`: `127.0.0.1:3333`
- `OpenClaw Gateway`: `127.0.0.1:18889`

然后在开发机执行：

```bash
ssh -N -L 19898:127.0.0.1:19898 <deploy-user>@<deploy-tailnet-host>
```

例如：

```bash
ssh -N -L 19898:127.0.0.1:19898 kris@vibe-team-mac
```

之后在开发机浏览器打开：

```text
http://127.0.0.1:19898
```

说明：

- 这条链路只把 `Spacebot` 暴露给开发机浏览器
- `Antfarm` 和 `OpenClaw` 仍然只在部署机本地互通
- 现有 `Spacebot` SSE 也会走同一条隧道
- 这是当前联调阶段的默认推荐方案

### 为什么不建议第一版直接把 Spacebot 绑定到 Tailscale IP

因为当前阶段优先目标是减少变量，而不是先做对外服务化。

直接绑定到 `Tailscale IP` 或 `0.0.0.0` 会额外引入：

- 监听地址选择
- CORS / Origin 行为差异
- 认证与暴露面控制
- 后续 HTTPS / 反代决策

这些都不属于当前 MVP 的必需条件。

### 长期方案

如果后续要把部署机上的 `Spacebot` 作为长期远程入口，再考虑：

1. 通过 `Tailscale` 直接访问部署机服务
2. 或在部署机前面加反向代理与 HTTPS
3. 或补 `auth_token` / 统一鉴权

但这些都应放在当前联调跑通之后。

---

## 4. 前置依赖

目标 Mac 建议至少具备：

- macOS
- `git`
- 真实 `Node.js >= 22`
- `npm`
- `pnpm`
- `bun`
- `Rust >= 1.85`
- 可用的 LLM Provider 凭据

关键注意：

- `Antfarm` 明确要求真实 `Node.js >= 22`
- 如果 PATH 里优先拿到的是 Bun 的 `node` wrapper，`Antfarm` 会因为 `node:sqlite` 不可用而失败
- `Spacebot` 编译时默认会尝试构建前端；如果没有先安装 `spacebot/interface` 依赖，最终二进制可能只能提供 API 或空 UI

推荐先检查：

```bash
node -v
node -e "require('node:sqlite')"
pnpm -v
bun -v
rustc --version
cargo --version
```

---

## 5. 推荐目录布局

推荐把“源码 checkout”和“运行时实例根”分开：

```text
$HOME/Desktop/Dev/Vibe-Team/
├── openclaw/
├── antfarm/
└── spacebot/

$HOME/instances/vibe-team/
├── config/
│   └── openclaw.json
├── state/
├── spacebot/
├── antfarm/
├── antfarm-home/
└── bin/
```

推荐环境变量：

```bash
export VIBE_TEAM_HOME="$HOME/Desktop/Dev/Vibe-Team"
export INSTANCE_ROOT="$HOME/instances/vibe-team"
export TARGET_PROJECT="$HOME/Desktop/Dev/target-project"
export OPENCLAW_STATE_DIR="$INSTANCE_ROOT/state"
export OPENCLAW_CONFIG_PATH="$INSTANCE_ROOT/config/openclaw.json"
export SPACEBOT_DIR="$INSTANCE_ROOT/spacebot"
```

要求：

- 源码目录可以删掉重拉
- 实例根目录不应依赖 `Desktop` 权限
- `OpenClaw`、`Antfarm`、`Spacebot` 必须看到同一个 `OPENCLAW_STATE_DIR`
- `launchd`、wrapper、运行时二进制都应只依赖实例根

---

## 6. 部署顺序

部署顺序不要打乱，推荐固定为：

1. `OpenClaw`
2. `Antfarm`
3. `Spacebot`
4. 联调与 smoke test

原因：

- `Antfarm` 会依赖 `OPENCLAW_STATE_DIR` 和 `OPENCLAW_CONFIG_PATH`
- 先做 `OpenClaw onboard`，能保证实例根里的状态目录和配置先存在
- `Spacebot` 的真实 Antfarm service 又依赖：
  - 可执行的 `antfarm` CLI
  - 可访问的 `Antfarm Dashboard`

---

## 7. 部署 OpenClaw

### 7.1 方案选择

`OpenClaw` 有两种合理装法。

方案 A，快速稳定：

- 全局安装 OpenClaw
- 适合先把运行底座拉起来

方案 B，源码部署：

- 适合你希望目标 Mac 上使用当前工作区里的源码版本
- 适合后续一起做源码级排查

按当前实机落地经验，目标 Mac 上更推荐方案 A 或“全局 CLI + 实例根状态目录”的组合；源码部署更适合排查和二次开发。

### 7.2 OpenClaw 源码部署

```bash
cd "$VIBE_TEAM_HOME/openclaw"
pnpm install
pnpm ui:build
pnpm build
pnpm openclaw onboard --install-daemon
```

如果你只是想先确认 OpenClaw 可用，也可以用官方推荐的全局安装方式：

```bash
npm install -g openclaw@latest
openclaw onboard --install-daemon
```

### 7.3 OpenClaw 最低验证

执行：

```bash
openclaw doctor
```

如果你要前台观察 Gateway，也可以临时前台启动：

```bash
OPENCLAW_STATE_DIR="$INSTANCE_ROOT/state" \
OPENCLAW_CONFIG_PATH="$INSTANCE_ROOT/config/openclaw.json" \
openclaw gateway --port 18889 --verbose
```

注意：

- `OpenClaw onboard --install-daemon` 会把 Gateway 装成用户级 daemon
- 第一版联调只需要确认：
  - OpenClaw 配置已生成
  - OpenClaw state 目录存在
  - OpenClaw runtime 可以正常启动

---

## 8. 部署 Antfarm

### 8.1 构建

```bash
cd "$VIBE_TEAM_HOME/antfarm"
npm install
npm run build
```

### 8.2 让 `antfarm` 命令可执行

有两种方式。

方式 A，最省事：

```bash
cd "$VIBE_TEAM_HOME/antfarm"
npm link
```

方式 B，不做全局 link，而是在 `Spacebot` 环境变量里显式指定 CLI 路径：

```bash
export SPACEBOT_ANTFARM_CLI_PATH="$INSTANCE_ROOT/bin/antfarm-vibe-team"
```

如果你已经用了 `npm link`，可以不再设置 `SPACEBOT_ANTFARM_CLI_PATH`。

### 8.3 安装 workflow 并启动 Dashboard

```bash
cd "$VIBE_TEAM_HOME/antfarm"
antfarm install
```

`antfarm install` 会：

- 安装 bundled workflows
- 尝试启动 Dashboard

当前仓库里现成的 workflow id 有：

- `feature-dev`
- `bug-fix`
- `security-audit`

### 8.4 手动检查 Dashboard

```bash
antfarm dashboard status
curl http://127.0.0.1:3333/api/runs
curl http://127.0.0.1:3333/api/workflows
```

如果 Dashboard 没有起来，可以手动启动：

```bash
antfarm dashboard start --port 3333
```

### 8.5 Antfarm 关键注意点

- 在当前标准部署中，`Antfarm` 运行时应放在：`$INSTANCE_ROOT/antfarm`
- `Antfarm` 的状态目录跟随 `OPENCLAW_STATE_DIR`，当前标准落点是：
  - `$INSTANCE_ROOT/state/antfarm`
  - `$INSTANCE_ROOT/state/workspaces/workflows`
- `Antfarm` 当前工作流仍大量依赖 prompt 中的 `REPO` / `BRANCH` 信息
- `Spacebot -> Antfarm` 现在已有独立的 `repo_path` / `branch` / `worktree_path` 字段
- 当前 launcher 会把这些结构化字段自动展开成兼容现有 workflow 的任务正文

---

## 9. 部署 Spacebot

### 9.1 构建前端资源

`Spacebot` 的前端资源来自 `spacebot/interface/dist/`，编译 Rust 二进制前先做这一步：

```bash
cd "$VIBE_TEAM_HOME/spacebot/interface"
bun install
bun run build
```

### 9.2 构建 Spacebot

```bash
cd "$VIBE_TEAM_HOME/spacebot"
cargo build --release
```

如果你只想做 API 级验证而暂时不关心 UI，也可以跳过前端构建并显式设置：

```bash
export SPACEBOT_SKIP_FRONTEND_BUILD=1
```

但这不适合作为最终部署方式，因为你最终前端就是 `Spacebot`。

### 9.3 准备最小配置

当前标准实例目录：

- `$INSTANCE_ROOT/spacebot`
- 或 `SPACEBOT_DIR` 指向的目录

创建配置文件：

```bash
mkdir -p "$SPACEBOT_DIR"
cat > "$SPACEBOT_DIR/config.toml" <<'EOF'
[llm.provider.openai-relay]
api_type = "openai_completions"
base_url = "https://ai.co.link/openai"
api_key = "env:OPENAI_AUTH_KEY"

[defaults.routing]
channel = "openai-relay/gpt-5.1"
branch = "openai-relay/gpt-5.1"
worker = "openai-relay/gpt-5.1"
compactor = "openai-relay/gpt-5.1"
cortex = "openai-relay/gpt-5.1"

[api]
enabled = true
bind = "127.0.0.1"
port = 19898

[[agents]]
id = "pm"
EOF
```

说明：

- 这里先给最小配置，目标是把 `Spacebot UI/API + Antfarm integration` 跑起来
- 如果你希望加 API 鉴权，可以再补：

```toml
[api]
enabled = true
bind = "127.0.0.1"
port = 19898
auth_token = "env:SPACEBOT_API_TOKEN"
```

如果加了 `auth_token`：

- `/api/health` 仍可匿名访问
- 其他 `/api/*` 路由需要：

```http
Authorization: Bearer <token>
```

### 9.4 配置 Spacebot 与 Antfarm 的集成环境变量

推荐用一个启动脚本统一注入：

```bash
cat > "$INSTANCE_ROOT/bin/run-spacebot-vibe-team.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

export VIBE_TEAM_HOME="$HOME/Desktop/Dev/Vibe-Team"
export INSTANCE_ROOT="$HOME/instances/vibe-team"
export SPACEBOT_DIR="$INSTANCE_ROOT/spacebot"
export OPENCLAW_STATE_DIR="$INSTANCE_ROOT/state"
export OPENCLAW_CONFIG_PATH="$INSTANCE_ROOT/config/openclaw.json"

export OPENAI_AUTH_KEY="your-key"

export SPACEBOT_ANTFARM_DASHBOARD_URL="http://127.0.0.1:3333"
export SPACEBOT_ANTFARM_CLI_PATH="$INSTANCE_ROOT/bin/antfarm-vibe-team"
export SPACEBOT_ANTFARM_WORKDIR="$HOME/Desktop/Dev"
export NO_PROXY="127.0.0.1,localhost"
export no_proxy="127.0.0.1,localhost"

exec "$INSTANCE_ROOT/bin/spacebot" --config "$SPACEBOT_DIR/config.toml" start --foreground
EOF

chmod +x "$INSTANCE_ROOT/bin/run-spacebot-vibe-team.sh"
```

可选环境变量说明：

- `SPACEBOT_ANTFARM_DASHBOARD_URL`
  - 必填
  - 例如：`http://127.0.0.1:3333`
- `SPACEBOT_ANTFARM_CLI_PATH`
  - 可选
  - 当前标准值是 `$INSTANCE_ROOT/bin/antfarm-vibe-team`
  - 如果没做 wrapper，也可退回 `antfarm`
- `SPACEBOT_ANTFARM_WORKDIR`
  - 可选
  - 用于指定 `antfarm workflow run ...` 的工作目录
- `SPACEBOT_ANTFARM_NOTIFY_URL`
  - 可选
  - 当前可透传给 Antfarm CLI 的 `--notify-url`
- `SPACEBOT_ENABLE_ANTFARM_MOCK`
  - 仅开发用
  - 目标 Mac 的真实部署不要开

### 9.5 启动 Spacebot

```bash
"$INSTANCE_ROOT/bin/run-spacebot-vibe-team.sh"
```

或者不写脚本，直接当前 shell 启动：

```bash
export INSTANCE_ROOT="$HOME/instances/vibe-team"
export SPACEBOT_DIR="$INSTANCE_ROOT/spacebot"
export OPENCLAW_STATE_DIR="$INSTANCE_ROOT/state"
export OPENCLAW_CONFIG_PATH="$INSTANCE_ROOT/config/openclaw.json"
export SPACEBOT_ANTFARM_DASHBOARD_URL="http://127.0.0.1:3333"
export SPACEBOT_ANTFARM_CLI_PATH="$INSTANCE_ROOT/bin/antfarm-vibe-team"
export NO_PROXY="127.0.0.1,localhost"
export no_proxy="127.0.0.1,localhost"
"$INSTANCE_ROOT/bin/spacebot" --config "$SPACEBOT_DIR/config.toml" start --foreground
```

---

## 10. 启动顺序与稳态检查

推荐每次按这个顺序启动：

1. 确认 OpenClaw runtime 已经 ready
2. 确认 Antfarm Dashboard 正常
3. 启动 Spacebot

启动后检查：

```bash
curl http://127.0.0.1:19898/api/health
curl http://127.0.0.1:19898/api/status
open http://127.0.0.1:19898
```

稳态判断标准：

- `Spacebot` 页面可打开
- `/api/health` 返回 200
- `/api/status` 返回实例状态
- `curl http://127.0.0.1:3333/api/runs` 返回 JSON

---

## 11. 联调顺序

不要一上来就做完整业务开发。联调建议分 4 步。

### 11.1 第一步：验证 Antfarm API 读路径

先只验证 `Spacebot -> Antfarm Dashboard` 读路径。

执行：

```bash
curl http://127.0.0.1:3333/api/workflows
curl http://127.0.0.1:3333/api/runs
```

目标：

- Spacebot 所依赖的 Dashboard JSON API 可访问
- `feature-dev` 等 workflow 已安装

### 11.2 第二步：验证 Spacebot Antfarm service 已启用

如果 `SPACEBOT_ANTFARM_DASHBOARD_URL` 没设，`/api/antfarm/*` 会返回 `503`。

所以要先验证：

```bash
curl http://127.0.0.1:19898/api/antfarm/conversations/portal:chat:pm/runs
```

预期：

- 如果服务已启用且当前 conversation 还没有 run，返回空 `runs`
- 如果返回 `503`，说明 `Spacebot` 没拿到 Antfarm service

### 11.3 第三步：手动触发一个 workflow run

这里先不要走真实 UI 操作，先直接调用 API，确保服务链路可控。

示例：

```bash
curl -X POST http://127.0.0.1:19898/api/antfarm/runs \
  -H 'Content-Type: application/json' \
  -d '{
    "request_id": "smoke-feature-001",
    "conversation_id": "portal:chat:pm",
    "workflow_id": "feature-dev",
    "task_title": "为 target-project 增加签到功能",
    "task_body": "需求：实现用户连续签到、断签重置、积分累加、最小可验证测试。",
    "repo_path": "'"$TARGET_PROJECT"'",
    "branch": "feature/checkin",
    "metadata": {}
  }'
```

说明：

- `conversation_id` 这里用 `portal:chat:<agent_id>`，其中 `pm` 要替换成你真实的 agent id
- `repo_path` 和 `branch` 现在建议用独立字段传
- 当前 launcher 会自动把这些字段转换成兼容 Antfarm workflow 的 `REPO:` / `BRANCH:` 文本上下文

预期：

- 返回 `run_id`
- `Spacebot` 发出 `workflow_run_started`
- 后台自动为该 run 起 poller

### 11.4 第四步：观察状态更新与最终结果

查询单个 run：

```bash
curl http://127.0.0.1:19898/api/antfarm/runs/<run_id>
```

查询 conversation 绑定的 runs：

```bash
curl http://127.0.0.1:19898/api/antfarm/conversations/portal:chat:pm/runs
```

如果 run 已结束，再取最终结果：

```bash
curl http://127.0.0.1:19898/api/antfarm/runs/<run_id>/result
```

同时在浏览器里看：

- `http://127.0.0.1:19898`
- 当前会话页面里是否出现 workflow 面板
- 状态是否随 run 变化而更新

### 11.5 第五步：改用 Spacebot UI 做真实联调

API smoke test 通过后，不要继续长期停留在 curl 模式，直接切到 `Spacebot` 主前端。

当前 UI 入口已经有两处：

- `WebChat` 页面中的 `Run Workflow`
- 普通 `channel` 详情页右上角的 `Run Workflow`

这两个入口都支持填写：

- `workflow_id`
- `repo_path`
- `branch`
- `worktree_path`
- `task_title`
- `task_body`

并且已经支持按 `agent` 保存多个项目 preset。

推荐操作方式：

1. 先在 `Run Workflow` dialog 中把目标项目保存成一个 preset
2. 之后联调只需要选择 preset
3. 每次只改：
   - `task_title`
   - `task_body`
   - 如有必要，改 `branch`

这样可以明显减少联调时重复填路径和 workflow id 的错误率。

---

## 12. 前端联调检查点

因为最终前端是 `Spacebot`，所以前端联调只需要验证这些点：

- 页面能打开 `Spacebot`
- 当前 conversation 能看到 workflow 面板
- workflow 启动后能看到：
  - `workflow_id`
  - `run number`
  - `status`
  - `current step`
  - `current agent`
  - `story progress`
- run 结束后能展开 terminal detail
- 刷新页面后，当前 conversation 还能恢复已绑定的 workflow runs
- 重启 `Spacebot` 进程后，已持久化 binding 还能恢复
- `Run Workflow` dialog 中的 preset 能被正常保存、切换、更新、删除
- `Custom draft` 模式下，未保存的路径输入不会因为误关弹窗而丢失

如果这些都成立，就不要继续打磨 UI，优先转到真实业务 workflow 联调。

---

## 13. SSE 联调检查点

当前 workflow 事件复用的是 `Spacebot` 的现有 SSE 总线，不是第二套通道。

可以直接验证：

```bash
curl -N http://127.0.0.1:19898/api/events
```

观察是否出现：

- `workflow_run_started`
- `workflow_run_updated`
- `workflow_run_completed`
- `workflow_run_failed`

注意：

- 当前终态事件由 poller 发出
- `GET /api/antfarm/runs/{id}` 和 `GET /result` 现在只负责查询，不再兼作事件源

---

## 14. 失败排查

### 14.1 `Antfarm` 启动失败，提示 `node:sqlite` 不可用

原因：

- 不是 Node 22
- 或 PATH 里先命中了 Bun 的 `node`

处理：

```bash
which node
node -v
node -e "require('node:sqlite')"
```

确保真实 Node 22+ 在前。

### 14.2 `Spacebot` 的 `/api/antfarm/*` 返回 `503`

原因：

- 没配 `SPACEBOT_ANTFARM_DASHBOARD_URL`
- 或 `SPACEBOT_ENABLE_ANTFARM_MOCK` 没开且真实 service 没装配成功

处理：

- 确认：
  - `SPACEBOT_ANTFARM_DASHBOARD_URL`
  - `SPACEBOT_ANTFARM_CLI_PATH`
  - `SPACEBOT_ANTFARM_WORKDIR`
- 确认 `http://127.0.0.1:3333/api/runs` 可访问

### 14.3 `Spacebot` 触发 run 失败

原因通常是：

- `antfarm` 不在 PATH
- `SPACEBOT_ANTFARM_CLI_PATH` 指错
- `antfarm workflow run ...` 自身失败

先手动试：

```bash
antfarm workflow run feature-dev "REPO: $TARGET_PROJECT

做一个最小 smoke task"
```

### 14.4 `Spacebot` 页面没有 workflow 面板

先排查：

- 前端是否已构建
- `Spacebot` 是否真的提供了嵌入式 UI
- 当前会话是否产生了 workflow 事件
- 当前 conversation 是否有 run binding

先看：

```bash
curl http://127.0.0.1:19898/api/antfarm/conversations/portal:chat:pm/runs
```

如果是从 UI 发起的 workflow，但页面没有显示，先再确认：

- 当前触发入口是不是对应当前 conversation
- `Run Workflow` dialog 提交后是否已经显示启动提示
- SSE 是否已经收到 `workflow_run_started`

### 14.5 页面刷新后 workflow 面板丢失

先区分两种情况：

- 只是浏览器刷新
- `Spacebot` 进程重启

当前实现里：

- 浏览器刷新可以通过 conversation 恢复接口回填
- `Spacebot` 进程重启可以通过 SQLite 持久化的 `workflow_run_bindings` 恢复

如果仍然恢复失败，重点看：

- `conversation_id` 是否可解析到 agent DB
- `workflow_run_bindings` 表里是否有记录

### 14.6 浏览器里找不到之前保存的项目 preset

原因通常是：

- 换了浏览器
- 换了浏览器 profile
- 清理了 localStorage

当前实现里：

- preset 是浏览器本地保存
- 不是 `Spacebot` 后端配置
- 不是部署机全局共享配置

所以这不是后端丢数据，而是当前设计边界

---

## 15. 第一次真实项目联调建议

第一次对真实目标项目联调时，不要直接上大需求。

推荐三步：

1. `smoke run`
2. `最小改动 run`
3. `真实业务 run`

### A. smoke run

目标：

- 验证 `Spacebot UI -> Antfarm -> OpenClaw` 全链路
- 不追求真实产出

建议任务：

- 读取 repo
- 识别技术栈
- 建立分支
- 跑 baseline build/test

### B. 最小改动 run

目标：

- 验证 workflow 不只是能启动，还能完成一次实际代码修改

建议任务：

- 改一个低风险文案
- 或补一个很小的非业务逻辑测试
- 或新增一个最小开发脚本

要求：

- 改动范围可控
- 容易回滚
- 容易判断成功失败

### C. 真实业务 run

目标：

- 验证这套方案能否支撑真实需求交付

这一步再开始使用真正的 feature 需求。

---

## 16. 数据落点

### 16.1 OpenClaw / Antfarm

当前标准落点：

- `$INSTANCE_ROOT/config/openclaw.json`
- `$INSTANCE_ROOT/state/antfarm/`
- `$INSTANCE_ROOT/state/workspaces/workflows/`

可被这些环境变量覆盖：

- `OPENCLAW_STATE_DIR`
- `OPENCLAW_CONFIG_PATH`

### 16.2 Spacebot

当前标准落点：

- `$INSTANCE_ROOT/spacebot/config.toml`
- `$INSTANCE_ROOT/spacebot/logs/`
- `$INSTANCE_ROOT/spacebot/agents/...`
- `$INSTANCE_ROOT/spacebot/data/secrets.redb`
- `$INSTANCE_ROOT/spacebot/anthropic_oauth.json`

可被这个环境变量覆盖：

- `SPACEBOT_DIR`

### 16.3 本次新增的 workflow binding 持久化

`Spacebot` 会把 workflow run binding 落到各 agent SQLite 中的：

- `workflow_run_bindings`

这张表用于：

- conversation 刷新恢复
- `Spacebot` 重启后的 run 恢复

---

## 17. 升级与重启建议

### 17.1 重启顺序

建议：

1. 停 `Spacebot`
2. 确认 `Antfarm Dashboard` 在不在
3. 如需，重启 `Antfarm Dashboard`
4. 再起 `Spacebot`

原因：

- `Spacebot` 启动时会尝试恢复已持久化的 workflow bindings
- 如果此时 Dashboard 不可用，未结束 run 的恢复与重挂 poller 会不完整

### 17.2 升级 Antfarm

在 `antfarm/` 目录：

```bash
npm install
npm run build
antfarm install
```

### 17.3 升级 Spacebot

在 `spacebot/interface/`：

```bash
bun install
bun run build
```

在 `spacebot/`：

```bash
cargo build --release
```

---

## 18. 已知注意事项

1. 当前 `feature-dev` workflow 仍是单 repo 多角色流水线，不是严格前后端物理隔离版。
2. 当前接口已有独立的 `repo_path` / `branch` / `worktree_path` 参数，但 Antfarm workflow 本身仍依赖这些信息最终以文本上下文形式出现。
3. 当前最终结果提取依赖 Antfarm 最后完成步骤的输出格式，严格结构化产物还需要后续继续规范。
4. 当前 workflow 更新依赖 polling，不是 Dashboard 主动推送。
5. 当前 `Spacebot` 里的 workflow 入口已经足够支撑联调，但不建议继续先做 UI 深挖，优先做真实部署验证。
6. 当前项目 preset 是浏览器本地能力，不是后端共享配置。
7. 如果你在部署机上换浏览器测试，需要重新建立 preset。

---

## 19. 推荐的第一次完整联调脚本

如果你已经按当前标准实例根完成部署，第一次联调按这个顺序即可：

```bash
export INSTANCE_ROOT="$HOME/instances/vibe-team"
export TARGET_PROJECT="$HOME/Desktop/Dev/target-project"

launchctl kickstart -k gui/$(id -u)/ai.openclaw.vibe-team
launchctl kickstart -k gui/$(id -u)/ai.antfarm.vibe-team
launchctl kickstart -k gui/$(id -u)/ai.spacebot.vibe-team

curl http://127.0.0.1:18889/v1/models
curl http://127.0.0.1:3333/api/workflows
curl http://127.0.0.1:19898/api/health
```

然后在另一个终端执行：

```bash
curl http://127.0.0.1:19898/api/health
curl http://127.0.0.1:3333/api/workflows

curl -X POST http://127.0.0.1:19898/api/antfarm/runs \
  -H 'Content-Type: application/json' \
  -d '{
    "request_id": "smoke-feature-001",
    "conversation_id": "portal:chat:pm",
    "workflow_id": "feature-dev",
    "task_title": "做一次 workflow 集成联调",
    "task_body": "需求：只做最小 smoke run，验证 Spacebot -> Antfarm -> OpenClaw 主链路。",
    "repo_path": "'"$TARGET_PROJECT"'",
    "branch": "chore/antfarm-smoke",
    "metadata": {}
  }'
```

浏览器打开：

```text
http://127.0.0.1:19898
```

到这里如果你能看到 workflow 状态变化，这条链就已经基本打通了。

---

## 20. 联调前最终检查表

开始真实联调前，逐项确认：

- `OpenClaw` 在部署机已可用
- `Antfarm Dashboard` 可访问
- `Spacebot` 页面可打开
- 开发机到部署机的 `Tailscale` 链路正常
- `Spacebot` 已配置真实 `SPACEBOT_ANTFARM_DASHBOARD_URL`
- `Spacebot` 没有开启 `SPACEBOT_ENABLE_ANTFARM_MOCK`
- 目标项目路径是绝对路径
- 目标项目有明确可写分支策略
- 至少有一个可用的 `Run Workflow` preset
- 先跑 `smoke run`，不要直接跑大需求

如果这 10 项都满足，就可以开始第一次真实项目联调。

---

## 21. 2026-03-03 实机落地补充

本节记录一次已经完成的本机落地结果，目标是把 `vibe-team` 的运行时形态收敛到和 `vibe-os` 一致的项目实例模型。

### 21.1 最终实例根目录

最终不要再把运行入口直接挂在 `Desktop` 路径下。

本次实机最终采用：

```text
/Users/kris/instances/vibe-team/
  config/
    openclaw.json
  state/
  spacebot/
  antfarm/
  antfarm-home/
  bin/
    spacebot
    antfarm-vibe-team
    run-spacebot-vibe-team.sh
    run-antfarm-dashboard-vibe-team.sh
```

说明：

- 源码 checkout 仍可保留在 `/Users/kris/Desktop/Dev/Vibe-Team`
- 但 `launchd`、常驻服务、wrapper、运行时二进制都应只依赖 `/Users/kris/instances/vibe-team`
- 这样可以避开 macOS 对 `Desktop` 目录的额外访问限制

### 21.2 为什么不能直接把 launchd 指到 Desktop

本次实机验证中，若 `launchd` 直接执行：

- `/Users/kris/Desktop/Dev/Vibe-Team/...`

会出现：

- `Operation not permitted`
- `getcwd: cannot access parent directories`

因此：

- `OpenClaw` 可继续使用 Homebrew / 全局 CLI 路径
- `Spacebot` 二进制应复制到实例根的 `bin/`
- `Antfarm` runtime 也应复制到实例根，而不是让 `launchd` 直接触达 Desktop checkout

### 21.3 本次最终服务形态

本次实机最终确认可用的端口：

- `OpenClaw Gateway`: `127.0.0.1:18889`
- `Antfarm Dashboard`: `127.0.0.1:3333`
- `Spacebot`: `127.0.0.1:19898`

最终服务形态：

- `OpenClaw`: 由 `launchd` 常驻
- `Spacebot`: 由 `launchd` 常驻
- `Antfarm Dashboard`: 由 `launchd` 在登录时触发一次 `dashboard start`，再由 `Antfarm` 自己的 daemon 常驻

这里 `Antfarm` 的进程模型与 `OpenClaw` / `Spacebot` 不同：

- `OpenClaw` / `Spacebot` 适合直接由 `launchd` 挂前台长驻
- `Antfarm dashboard` 更适合沿用它自己的 `dashboard start` -> detached daemon 模型

所以如果你在 `launchctl print` 中看不到一个长期驻留的 `ai.antfarm.vibe-team`，但：

- `3333` 正在监听
- `/api/workflows` 正常
- `/api/runs/*` 正常

那是符合预期的，不代表部署失败。

### 21.4 本次实际使用的 launchd 入口

本次实机使用的入口文件为：

- `OpenClaw launchd`: `/Users/kris/Library/LaunchAgents/ai.openclaw.vibe-team.plist`
- `Spacebot launchd`: `/Users/kris/Library/LaunchAgents/ai.spacebot.vibe-team.plist`
- `Antfarm dashboard bootstrap launchd`: `/Users/kris/Library/LaunchAgents/ai.antfarm.vibe-team.plist`

对应运行脚本 / wrapper：

- `/Users/kris/instances/vibe-team/bin/spacebot`
- `/Users/kris/instances/vibe-team/bin/antfarm-vibe-team`
- `/Users/kris/instances/vibe-team/bin/run-spacebot-vibe-team.sh`
- `/Users/kris/instances/vibe-team/bin/run-antfarm-dashboard-vibe-team.sh`

### 21.5 Spacebot 最终 provider 口径

本次实机没有让 `Spacebot` 直接走本地 `OpenClaw gateway` 的 Responses API 作为自己的主 LLM provider。

原因：

- 本地 `OpenClaw` 的 `/v1/responses` 可用
- 但 `Spacebot` 的某些调用 payload 与当前本地 gateway 的 Responses 兼容性并不完全一致

最终稳定方案是让 `Spacebot` 直接使用标准 OpenAI-compatible relay：

```toml
[llm.provider.openai-relay]
api_type = "openai_completions"
base_url = "https://ai.co.link/openai"
api_key = "env:OPENAI_AUTH_KEY"

[defaults.routing]
channel = "openai-relay/gpt-5.1"
branch = "openai-relay/gpt-5.1"
worker = "openai-relay/gpt-5.1"
compactor = "openai-relay/gpt-5.1"
cortex = "openai-relay/gpt-5.1"
```

注意：

- `Spacebot` 访问外部 relay 时可继续走本机代理
- 但访问本地 `Antfarm dashboard` 必须设置：

```bash
NO_PROXY=127.0.0.1,localhost
no_proxy=127.0.0.1,localhost
```

否则本地 `127.0.0.1:3333` 会被错误地走代理，导致 `502`

### 21.6 Antfarm 迁根时必须一起复制的内容

不要把这一步理解成“只补齐几个缺的目录”。

本次实机验证表明，迁根到新的实例运行目录时，应复制完整的、已经构建好的 `Antfarm` runtime 根目录。至少包含：

- `antfarm/dist/`
- `antfarm/node_modules/`
- `antfarm/workflows/`
- `antfarm/agents/`
- `antfarm/package.json`

否则会出现以下问题：

- dashboard 启动时缺少 `yaml`
- `/api/workflows` 返回空数组
- workflow install 报 `Missing bootstrap file for agent "setup"`

### 21.7 OpenClaw 配置迁根时必须检查的字段

如果实例根从旧路径迁到新路径，`config/openclaw.json` 中以下字段必须同步更新：

- `state/logs` 路径
- workflow subagent 的 `workspace`
- workflow subagent 的 `agentDir`
- 任何硬编码到旧实例根的 `state/workspaces/...`

本次实机迁根时，`openclaw.json` 中这些字段如果不替换，会导致：

- Gateway 虽然能起
- 但 workflow agents 仍落回旧实例路径

### 21.8 本次实机最终 smoke 结果

本次实机最终确认通过：

- `OpenClaw /v1/responses` 正常
- `Spacebot /api/health` 正常
- `Spacebot /api/status` 正常
- `Antfarm /api/workflows` 正常
- `Spacebot -> Antfarm` 查询正常
- 迁根后重新触发的新 run 成功创建：
  - `e08354ab-8172-43f1-8a1a-84471ae58212`

说明：

- `/Users/kris/instances/vibe-team` 这套实例根已经可作为后续联调和常驻运行的标准形态
- 旧的 `~/.openclaw-instances/vibe-team` 已可视为过渡目录，迁移完成后可删除

---

## 22. 下一步推进顺序

结合当前代码状态、总方案和实机落地结果，后续推进不要再平均用力，而应按下面顺序做。

### 22.1 先打通“开发机使用部署机 Spacebot”

当前第一优先级不是继续改 UI，也不是继续扩 workflow，而是验证真实使用路径：

- `部署机` 运行：
  - `Spacebot`
  - `Antfarm`
  - `OpenClaw`
  - `target-project`
- `开发机` 负责：
  - 通过 `Tailscale` 访问部署机
  - 在浏览器里打开 `Spacebot`
  - 从 `Spacebot` 页面触发 workflow

建议动作：

1. 在开发机建立到部署机的 `SSH` 端口转发
2. 在开发机浏览器打开 `http://127.0.0.1:19898`
3. 进入 `WebChat` 或 `ChannelDetail`
4. 使用现有 `Run Workflow` 入口发起一次 `smoke run`

本阶段验收标准：

- 页面能正常打开
- API 请求正常
- SSE 能持续接收状态更新
- workflow 能进入 terminal 状态
- terminal result 能回到 `Spacebot` 页面

### 22.2 再做一次“最小改动 run”

`smoke run` 只验证链路存活，不验证真实交付能力。

第二步应选择一个低风险、易回滚的小任务，例如：

- 改一个文案
- 补一个很小的测试
- 新增一个最小开发脚本

目标：

- 验证 workflow 真的能改目标项目
- 验证 build/test 命令能在真实项目里执行
- 验证 `Spacebot` 返回的结果摘要足够看懂
- 验证失败时能通过 `Spacebot` 定位问题

### 22.3 用第一次真实 run 收敛最终输出契约

当前 `Spacebot` 已能展示：

- `changes`
- `tests`
- `review_decision`
- `branch`
- `pr_url`

但这些字段当前仍然依赖 Antfarm 最终输出的 best-effort 提取。

因此第一次真实 run 完成后，应优先做这件事：

1. 固定 `Antfarm` 最终 step 的输出模板
2. 明确必填字段：
   - `STATUS`
   - `CHANGES`
   - `TESTS`
   - `REVIEW_DECISION`
   - `BRANCH`
   - `PR_URL`
   - `OPEN_QUESTIONS`
3. 让 `Spacebot` 只解析这套固定结构

这一步完成后，系统稳定性会明显高于继续打磨 UI。

### 22.4 联调稳定后，再做 `feature-dev-split`

当前不建议立刻上前后端双流水线。

更稳的顺序是：

1. 先让默认 workflow 稳定跑通
2. 再根据真实 run 的问题点改 workflow
3. 最后再引入分工版 workflow：
   - `planner`
   - `backend-dev`
   - `contract-reviewer`
   - `frontend-dev`
   - `integrator`
   - `tester`
   - `reviewer`

第一版分工 workflow 仍建议采用串行：

```text
plan
-> backend-implement
-> contract-review
-> frontend-implement
-> integrate
-> test
-> review
```

不要一开始就追求并行开发。

### 22.5 当前明确不优先做的事项

在以下事项上继续投入，当前边际收益较低：

1. 继续细化 `Spacebot` UI
2. 做 `Antfarm Dashboard` 嵌入
3. 做自然语言自动触发 workflow
4. 做严格前后端物理隔离
5. 做完全自动化、零人工介入流程

这些能力都应该放在真实联调稳定之后。

### 22.6 当前推荐执行清单

如果只保留最小动作，按这个顺序执行：

1. 开发机连上部署机 `Spacebot`
2. 跑一次真实 `smoke run`
3. 跑一次真实 `最小改动 run`
4. 收集终态输出和失败样例
5. 回到开发机修改：
   - Antfarm 最终输出模板
   - Spacebot 结果解析逻辑
   - `feature-dev-split` workflow 草案

如果这 5 步走完，下一阶段就可以从“集成开发”转到“稳定交付能力建设”。

---

## 23. 2026-03-03 当前阻塞补充

本节记录一次在真实实例上的最新联调结果。

注意：

- 这一节不是对前文“部署完成”的否定
- 而是对当前阻塞点的补充说明
- 截至本节记录时，系统尚未满足 `22.1` 中“workflow 能进入 terminal 状态”的验收标准

### 23.1 当前现象

当前实例：

- `INSTANCE_ROOT=/Users/kris/instances/vibe-team`
- `OpenClaw Gateway=127.0.0.1:18889`
- `Antfarm Dashboard=127.0.0.1:3333`
- `Spacebot=127.0.0.1:19898`

在 `Spacebot` 中触发 workflow 后，run 可以成功创建，但 step 没有继续推进。

实机检查结果：

```bash
"$ANT" workflow status b04b41cf
```

返回要点：

- run 已创建
- `Status: running`
- 但首个 step 仍停在：
  - `[pending] plan (feature-dev_planner)`
- 后续 step 全部仍是：
  - `[waiting]`

这说明当前问题不是“run 创建失败”，而是“run 创建成功后，没有 agent 真正消费第一个 pending step”。

### 23.2 Medic 当前状态

实机检查：

```bash
"$ANT" medic status
```

返回结果：

```text
Antfarm Medic
  Cron: not installed
  Last check: never
  Last 24h: 0 checks, 0 issues found, 0 auto-fixed
```

这意味着：

- 当前实例上的 `Antfarm Medic` 尚未处于正常轮询状态
- 至少从当前观测看，没有一个健康的 cron / medic 机制在推动 workflow step 前进

### 23.3 当前日志观察

`Antfarm dashboard` 的 bootstrap stderr：

```bash
tail -n 100 "$INSTANCE_ROOT/state/antfarm/launchd-dashboard.stderr.log"
```

当前没有有效报错输出。

`OpenClaw gateway` stderr：

```bash
tail -n 100 "$INSTANCE_ROOT/state/logs/gateway.err.log"
```

可见重复报错，关键内容包括：

- `exec host=sandbox is configured, but sandbox runtime is unavailable for this session`
- `exec host not allowed (requested gateway; configure tools.exec.host=sandbox to allow)`
- `exec host not allowed (requested node; configure tools.exec.host=sandbox to allow)`

这说明当前除了 `medic/cron` 未安装外，`OpenClaw` 侧还存在 `tools.exec.host` 与 `sandbox` 配置不一致的问题。

### 23.4 当前判断

截至本节记录时，最合理的当前判断是：

1. `Antfarm run` 创建链路本身是通的
2. 但 `plan` step 没有被实际拉起执行
3. 当前缺少健康的 `medic/cron` 轮询推进机制
4. 同时 `OpenClaw` 的 `tools.exec` 配置存在运行时冲突

因此：

- 当前系统已经达到“能发起 run”
- 但还没有达到“workflow 可持续推进到 terminal 状态”

### 23.5 下一轮排查建议

下一轮排查建议只聚焦两个方向，不要同时扩散：

1. 先确认为什么 `Antfarm Medic` / cron 没有安装成功
   - 是否 `install` 过程中没有真正落地
   - 是否实例迁根后 `HOME` / wrapper / launch 方式导致 cron 安装位置失效
   - 是否 `dashboard start` 的 daemon 模型没有连带恢复 medic 所需机制

2. 再确认 `OpenClaw` 的 `tools.exec.host` / `sandbox` 配置为什么冲突
   - 是否当前 `openclaw.json` 同时保留了互斥配置
   - 是否 workflow agent 运行期请求了 `gateway` / `node`，但当前 profile 只允许 `sandbox`
   - 是否当前会话缺少 sandbox runtime，导致 `exec` 无法真正执行

在这两个问题明确前，不应把当前状态记录为“联调已完全跑通”。

### 23.6 2026-03-03 当天后续修复结果

上述阻塞在同一天的后续排障中已被实机解除。

本次实机最终确认：

- `feature-dev` workflow 不再停在“只能创建 run”
- `plan pending` 问题已解除
- `feature-dev` 的 story loop 已能真实推进：
  - `plan -> setup -> implement -> verify -> test -> pr -> review`
- smoke run `#7 / 8bc6e5c2-3d11-4a5a-afbc-4cff60a60498` 最终状态为：
  - `Status: completed`

本次闭环 run 的最终结果：

- 所有 step 均为 `done`
- 所有 stories `US-001` 到 `US-005` 均为 `done`
- run 日志最终出现：
  - `Run completed`

因此：

- 本实例现在已经满足第 `22.1` 节中“workflow 能进入 terminal 状态”的验收标准
- 第 `23.1` 到 `23.5` 节应视为“当时阻塞快照”，不是当前最终状态

### 23.7 本次最小修复的实际内容

本次没有继续扩散做大改，而是只修了真正阻塞 workflow 推进的几类问题。

#### 23.7.1 agent cron 必须走实例 wrapper，而不是裸 CLI

实机验证表明，cron payload 若直接执行类似：

```bash
node .../antfarm/dist/cli/cli.js ...
```

会读到错误的 `HOME` / `OPENCLAW_STATE_DIR` / `OPENCLAW_CONFIG_PATH`，从而导致：

- 读错实例状态
- medic / cron 看起来“已装”但实际不推进正确 run

本次修复后统一要求：

- cron payload 优先走实例 wrapper：
  - `/Users/kris/instances/vibe-team/bin/antfarm-vibe-team`

#### 23.7.2 cron 必须使用 headless delivery

CLI fallback 创建 cron 时，如果仍落到 `announce/last`，在没有 channel 的实例上会报：

- `Channel is required`

本次修复后统一要求：

- agent cron 使用 `delivery.mode=none`
- CLI fallback 创建 cron 时显式 `--no-deliver`

#### 23.7.3 workflow agent 的 exec 必须与 gateway 实际能力一致

本次实机阻塞的核心并不是 run 创建，而是 agent 执行面冲突：

- 一部分会话默认走了 `sandbox`
- 一部分会话回退到 `gateway` 时又用了错误安全级别

本次实机确认后的可用口径是：

- workflow agents 使用 `tools.exec.host = "gateway"`
- `tools.exec.security = "full"`
- `sandbox.mode = "off"`

同时在 cron/work prompt 中明确要求：

- Antfarm/OpenClaw CLI
- `git`
- `npm`
- 本地 shell 命令

都必须显式使用：

- `host="gateway"`
- `security="full"`

不能再依赖模型默认推断。

#### 23.7.4 planner 输出必须严格满足 reply contract

本次还发现另一个“假推进”问题：

- planner 可能输出了通用 `STATUS/CHANGES/TESTS`
- 但没有真正产出 `REPO/BRANCH/STORIES_JSON`
- 旧逻辑却仍可能把 step 判成 `done`

本次修复后：

- agent prompt 不再强灌统一完成模板
- `step complete` 会校验 step 自身 `Reply with:` 契约
- planner 缺 `REPO/BRANCH/STORIES_JSON` 时会被拒收

这一步修完后，新的 smoke run 已能自动产出：

- `REPO`
- `BRANCH`
- `STORIES_JSON`

并自动推进到 `setup`

#### 23.7.5 verify_each 与 loop completion 需要额外保护

本次 story loop 闭环里还暴露了两个运行态问题：

1. `verify_each` 在 loop step `running` 时无法正常 `peek/claim`
2. 旧 developer 会话可能在 `current_story_id` 已清空后再次 `step complete`，把整条 loop 提前关掉

本次修复后：

- `verify_each` 可在 loop `running` 时正常 claim
- loop step 在 `current_story_id` 为空时，不再接受 stale completion

这两个修复是 story loop 能稳定从 `US-001` 跑到 `US-005` 的关键。

### 23.8 本次 smoke run 的实际验证结论

本次闭环 smoke run 在目标项目 `LobsterBoard` 上最终验证了三件事：

1. workflow 不只是“能创建 run”，而是能真实推进到 terminal
2. planner 产物已能被后续 step 消费
3. 真实 repo 中可以完成一轮最小 smoke 交付并跑通验证

本次在 `LobsterBoard` 分支 `chore/antfarm-smoke-002` 上落下的 smoke 交付物包括：

- `planner-smoke` widget
- `planner-smoke` custom page
- smoke doc：
  - `docs/antfarm-smoke-002.md`
- `node:test` 覆盖

实机验证通过：

```bash
node --test
npm run build
```

### 23.9 当前仍然存在的剩余风险

虽然本次已经完成 terminal 闭环，但仍有几类风险需要记录。

#### 23.9.1 “完全零人工介入”还不能视为已稳定

本次虽然最终跑到了 terminal，但后半段 terminal phase 为了快速收口，仍使用了 Antfarm 正常状态机接口做人工接管：

- `step claim`
- `step complete`

也就是说：

- “workflow 可闭环”已经成立
- 但“每一轮都能完全无人值守闭环”还不应直接宣称为稳定结论

#### 23.9.2 workflow spec 仍带有偏理想化验收模板

`feature-dev` 的 planner / verify / review 模板里仍有一些对通用项目过强的假设，例如：

- 默认要求 `Typecheck passes`
- 默认要求 frontend visual verification
- 默认要求真实 `gh pr create` / `gh pr review`

这些对像 `LobsterBoard` 这种：

- 无 `typecheck`
- 无真实 PR 需求
- smoke-only 目标

的任务并不总是贴合。

因此下一步最值得做的是：

- 收敛 `feature-dev` 各 step 的默认契约
- 把“smoke-only / no-real-PR / no-typecheck-project”这类条件显式参数化

#### 23.9.3 progress 文件路径假设仍不够统一

本次 developer fallback 过程中还暴露出一个旧假设：

- agent 会默认从目标 repo 根读取 `progress-<run>.txt`

但实际 workflow workspace、目标 repo、实例状态目录三者并不总是同一路径模型。

这不会再阻塞当前实例推进，但属于后续应继续收敛的运行面问题。

#### 23.9.4 `agent-browser` 已实机打通，browser review 路径已从“可选 fallback”变为“可真实执行”

在后续 UI smoke 验证中，`feature-dev` 的 `verifier` / `reviewer` 已改为使用官方 `agent-browser` skill：

- `https://github.com/vercel-labs/agent-browser/blob/main/skills/agent-browser/SKILL.md`

本次实机确认了三层状态：

1. workflow 已为 `verifier` / `reviewer` 声明 `agent-browser`
2. live instance 已把官方 skill provision 到对应 workspace
3. Playwright Chromium 运行时也已通过代理下载完成，`agent-browser` 不再停留在“只有 SKILL.md，没有 browser runtime”

本次 browser smoke 的 live 证据是：

- smoke run：
  - `#10 / cd272a28-09ee-48f1-8252-acf96a50a6e1`
- verifier screenshot：
  - `/tmp/cd272a28-verify.png`
- reviewer screenshot：
  - `/tmp/cd272a28-review.png`

该 run 已实机确认：

- `verify` 输入中 `Has frontend changes: true`
- `review` 输入中 `Has frontend changes: true`
- `agent-browser` 能真实打开：
  - `file:///Users/kris/Desktop/Dev/LobsterBoard/pages/planner-smoke/index.html`
- 页面中新增的可视化目标文案已被浏览器渲染并读到：
  - `Browser verification target ready`

因此：

- 当前实例已经具备真实 browser verification / visual review 能力
- “browser tool 不可用，所以只能 fallback 到代码检查”不再是当前 live 实例的主要限制

#### 23.9.5 当前自动调度面的更准确剩余风险：cron job 已创建，但 `feature-dev` agent 仍可能因缺失 `tools.exec` 配置而在 `step peek` 自阻塞

在 `#10` 的后续排查中，实机确认：

- live instance 的 cron job 注册文件已经存在：
  - `/Users/kris/instances/vibe-team/state/cron/jobs.json`
- `feature-dev` 的 planner / setup / developer / verifier / tester / reviewer jobs 都已经写入 jobs.json

但同一时间，live `openclaw.json` 里 `feature-dev_*` agent 条目仍存在一个更隐蔽的问题：

- agent 条目里虽然保留了：
  - `deny: ["gateway", ...]`
- 却没有像 `bug-fix_*` / `security-audit_*` 那样显式写入：
  - `tools.exec.host = "gateway"`
  - `tools.exec.security = "full"`
  - `tools.exec.ask = "off"`

这会导致 cron session 实际运行时在最开始的 `step peek` 就报：

- `exec host not allowed (requested gateway; configure tools.exec.host=sandbox to allow)`

因此当前更准确的结论是：

- “cron 没创建”不是当前根因
- “cron job 已创建，但 `feature-dev` agent 缺少显式 `tools.exec` 配置，导致自动 poller 在 peek 阶段自阻塞”才是当前调度面的主要剩余风险

这一点解释了为什么：

- jobs.json 里已经有 `feature-dev` poller
- 但新 run 仍可能长时间停在 `plan pending`
- 人工 `step claim/complete` 仍然能推进，因为坏的不是状态机本身，而是 cron agent 的本地 CLI 可执行能力

后续修复原则应明确为：

- 安装 workflow 时，所有依赖本地 Antfarm/OpenClaw CLI、`git`、`npm` 的 workflow agents，都必须显式写入 `tools.exec`
- 不能只依赖 role/profile，而不把 `exec.host/security` 实际落进 agent config

#### 23.9.6 上述 `tools.exec` 缺口修复后的 live 回归结果

在继续排障后，已对 installer 做进一步修复：

- workflow install 现在会为 workflow agents 显式写入：
  - `tools.exec.host = "gateway"`
  - `tools.exec.security = "full"`
  - `tools.exec.ask = "off"`

随后在 live instance 上重新安装 `feature-dev`，并再次核对：

- `feature-dev_planner`
- `feature-dev_setup`
- `feature-dev_developer`
- `feature-dev_verifier`
- `feature-dev_tester`
- `feature-dev_reviewer`

这些 agent 在 live `openclaw.json` 中都已带上上述 `tools.exec` 配置。

修复后的自动调度回归验证：

- 重新创建 `feature-dev` cron jobs
- 启动新的 smoke run：
  - `#11 / 88820962-978e-4eec-bb19-82282c454c61`
- 本轮**没有人工 `step claim`**

live 结果显示：

- `07:08 PM [88820962] planner Claimed step`
- `07:10 PM [88820962] Step completed`
- `07:10 PM [88820962] setup Claimed step`
- `07:10 PM [88820962] Step completed`

对应 planner cron 新 job 文件中也已经出现成功执行记录，说明新 cron session 已能跨过原来的 `step peek` / `gateway exec` 阻塞并真正消费 work：

- `/Users/kris/instances/vibe-team/state/cron/runs/2ab9003c-ca1b-4189-937a-686692e64860.jsonl`

该 run 在无人手动 claim 的情况下，已自动推进到：

- `plan done`
- `setup done`
- `implement pending`

因此：

- “feature-dev 新 run 会长期停在 `plan pending`”这一问题，在修复 live agent `tools.exec` 配置后已被实机解除
- 当前剩余的不确定性不再是 `plan/setup` 自动推进，而是更后续阶段是否还会暴露新的 repo/task 特定问题

#### 23.9.7 权限分离回归后的 live 结论：`__cron` helper 已能无人值守推进 `plan -> setup`，但真实 worker 仍暴露出新的执行面约束

在开发机 code review 指出“不能把所有真实 workflow 角色都直接升级成 `gateway/full/ask=off` exec”之后，对 `feature-dev` 做了进一步收敛：

- 真实 workflow agents 继续保留按角色收紧的工具策略
- 新增独立的 `feature-dev_*__cron` helper agents
- 只有这些 `__cron` helper agents 持有：
  - `tools.exec.host = "gateway"`
  - `tools.exec.security = "full"`
  - `tools.exec.ask = "off"`
- cron job 的 `agentId` 也切换为：
  - `feature-dev_planner__cron`
  - `feature-dev_setup__cron`
  - `feature-dev_developer__cron`
  - `feature-dev_verifier__cron`
  - `feature-dev_tester__cron`
  - `feature-dev_reviewer__cron`

本轮 live 验证过程中，实机又暴露出三层此前被旧 fallback 掩盖的执行面问题：

1. 新建的 `__cron` helper agent 若未显式写入 `sandbox.mode = "off"`，`sessions_spawn` 会先被 sandbox 继承策略拦住
2. polling prompt 若不显式指定 `runtime: "subagent"`，模型会把 worker handoff 误走到 ACP 语义，报：
   - `ACP runtime backend is not configured. Install and enable the acpx runtime plugin.`
3. worker work prompt 若硬性要求 `exec host = "gateway"`，真实 workflow agent 会因为未授予 gateway host 而在执行 repo 命令时自阻塞；对 worker 更合理的约束是：
   - 使用当前 agent 的默认 exec policy
   - 不在 prompt 里强推 `gateway/full`

修完上述三点后，重新同步 live instance、重建 `feature-dev` crons，并启动新的无人值守回归：

- `#17 / 6dfeca7d-ad44-4873-9e27-cc5c2ae19093`

本轮**没有人工 `step claim`**，初始 live 结果为：

- `08:24 AM [6dfeca7d] planner Claimed step`
- `08:26 AM [6dfeca7d] Step completed`
- `08:26 AM [6dfeca7d] Pipeline advanced`
- `08:30 AM [6dfeca7d] setup Claimed step`
- `08:30 AM [6dfeca7d] Step completed`
- `08:30 AM [6dfeca7d] Pipeline advanced`

对应状态先自动推进到：

- `plan done`
- `setup done`
- `implement pending`

随后在 `US-001` 完成后，live 又暴露出一个新的运行面现象：

- `feature-dev_verifier__cron` 在 `jobs.json` 里残留了 `runningAtMs`
- 导致 `verify pending` 没被继续捡起
- 通过一次：
  - `launchctl kickstart -k gui/$(id -u)/ai.openclaw.vibe-team`
  - 清理掉卡住的 cron runtime 后，`#17` 又继续自动向前推进

重启后，`#17` 继续出现：

- `08:35 AM [6dfeca7d] developer Claimed step`
- `08:35 AM [6dfeca7d] developer Story started — US-001`
- `08:37 AM [6dfeca7d] Story done — US-001`
- `08:40 AM [6dfeca7d] verifier Claimed step`
- `08:41 AM [6dfeca7d] Story verified`

在同一轮（2026-03-04）继续无人值守运行后，`#17` 最终到达 terminal：

- `09:11 AM [6dfeca7d] tester Claimed step`
- `09:12 AM [6dfeca7d] Step completed`
- `09:15 AM [6dfeca7d] developer Claimed step`
- `09:15 AM [6dfeca7d] Step completed`
- `09:17 AM [6dfeca7d] reviewer Claimed step`
- `09:17 AM [6dfeca7d] Step completed`
- `09:17 AM [6dfeca7d] Run completed`

因此可以把当前 live 结论更新为：

- `feature-dev` 在“cron helper 与真实 workflow role 分离”的前提下，已经再次实机证明可在**无人手工 claim**下自动推进
- 当前至少已连续自动跨过：
  - `plan -> setup`
  - `implement(US-001) -> verify(US-001)`
- 同一 run 已继续自动完成：
  - `test -> pr -> review -> terminal(completed)`
- 这说明权限分离后的核心推进链仍然成立，而不是只在旧的“所有角色都拿 gateway/full exec”模型下才成立

但也要明确保留当前剩余风险：

- live gateway 在重建 cron job 时出现过一次瞬时超时：
  - `gateway timeout after 120ms`
  - 通过 `launchctl kickstart -k gui/$(id -u)/ai.openclaw.vibe-team` 后恢复
- live cron runtime 也出现过一次 `runningAtMs` 残留，需要同样通过 `launchctl kickstart -k ...` 清理
- 因此当前最准确的口径是：
  - `feature-dev` 已在“权限分离 + __cron helper”模型下出现至少 1 次无人手工 claim 的完整 terminal 闭环
  - 但稳定性仍需继续 soak（目标从“能否到 terminal”切换为“到 terminal 的可重复性与无需重启恢复”）

#### 23.9.8 第二轮 unattended soak（`#18 / f0f5dbed`）结论：已再次到 terminal，但暴露出“polling-only 场景下 abandoned cleanup 触发点不足”的运行面缺口

在 `2026-03-04` 发起第二轮 unattended soak：

- run: `#18 / f0f5dbed-a7bf-4bb4-9a82-9c2a5dcd8a11`
- 任务目标：在不人工 claim 的前提下验证权限分离模型可重复到 terminal

实机过程里出现过一段长时间停滞：

- `11:56 AM` verifier claimed 后，subagent 输出了 `STATUS: retry ...` 文本，但没有实际执行 `step complete` / `step fail`
- run 一度停在：
  - `implement = running`
  - `verify = running`

随后系统在 `12:40 PM` 自动记录：

- `Step timed out (Reset to pending (abandon 1/5))`

并继续自动推进，最终在 `2026-03-04 01:43 PM` 到达：

- `Run completed`

也就是第二轮 unattended soak 仍然实现了无人手工 claim 的 terminal 闭环。

但本轮也明确暴露了一个实现缺口：

- 现有 `cleanupAbandonedSteps()` 主要由 `claimStep()` 路径触发
- 当 cron 长时间只做 `peek -> NO_WORK` 轮询时，abandoned 清理触发不够及时/不够稳定

针对这个缺口，已补最小修复（代码已落地并同步 live）：

- `peekStep()` 也纳入同一节流 cleanup 路径（与 `claimStep()` 共用）
- 增加回归测试覆盖：
  - polling-only 情况下也能回收超时 `running` step

因此，`#18` 的结论可归纳为：

- ✅ 在权限分离模型下再次完成无人手工 claim 的 terminal run
- ⚠️ 过程中出现过一次 `step timeout`，说明 worker 不回报时仍会拉长总时长
- ✅ 针对“peek-only 场景 cleanup 触发不足”的运行面缺口已经补上最小修复

#### 23.9.9 第三轮 unattended soak（`#19 / 1956b73e`）结论：稳定性门槛已达成（连续 3 次 terminal）

在 `2026-03-04` 发起第三轮 unattended soak：

- run: `#19 / 1956b73e-641c-4c60-a351-7128f6022836`
- 任务目标：验证在 `peek` 路径 cleanup 修复后，链路能否再次无人手工 claim 到 terminal

本轮实机结果：

- `plan -> setup -> implement -> verify -> test -> pr -> review` 全部自动推进并完成
- 最终状态：`Run completed`
- 全程未执行人工 `step claim`

结合前两轮：

- `#17`（completed）
- `#18`（completed，含一次 `step timeout` 后自动恢复）
- `#19`（completed）

当前已满足本阶段“连续 3 次 unattended terminal”的稳定性门槛，可以从“继续验证能否到 terminal”切换到“输出契约收敛”阶段。

---

## 24. 当前部署机推进清单（可直接给部署机 LLM）

本节只回答“现在下一步怎么做”，不再展开历史背景。

### 24.1 当前阶段目标

当前目标不是继续改 UI，也不是继续扩架构，而是进入“输出契约收敛”：

- `feature-dev` 在“权限分离 + __cron helper”模型下
- 已经验证可连续 unattended 到 terminal（`#17/#18/#19`）

下一步建议：

1. 固定 terminal 输出字段，减少 best-effort 解析
2. 在收敛期间继续保留低频 unattended soak 监控（防回归）
3. 若再次出现 `runningAtMs` 残留或频繁 `kickstart` 依赖，回退到稳定性专项排查

### 24.2 部署机最小执行步骤

在部署机执行：

```bash
export INSTANCE_ROOT="/Users/kris/instances/vibe-team"
export OPENCLAW_STATE_DIR="$INSTANCE_ROOT/state"
export OPENCLAW_CONFIG_PATH="$INSTANCE_ROOT/config/openclaw.json"
export ANT="$INSTANCE_ROOT/bin/antfarm-vibe-team"
```

先做健康检查：

```bash
curl -sf http://127.0.0.1:18889/v1/models >/dev/null && echo "openclaw ok"
curl -sf http://127.0.0.1:3333/api/workflows >/dev/null && echo "antfarm dashboard ok"
curl -sf http://127.0.0.1:19898/api/health >/dev/null && echo "spacebot ok"
```

确认 `feature-dev` 的 `__cron` jobs 已存在：

```bash
jq -r '.[].agentId // empty' "$OPENCLAW_STATE_DIR/cron/jobs.json" | rg '^feature-dev_.*__cron$' | sort -u
```

如需重建（例如刚更新过 installer/workflow）：

```bash
"$ANT" workflow crons-recreate feature-dev
```

然后发起新的 unattended run（推荐仍从 Spacebot UI 发起）。  
如需 API 触发，可参考：

```bash
curl -X POST http://127.0.0.1:19898/api/antfarm/runs \
  -H 'Content-Type: application/json' \
  -d '{
    "request_id": "soak-feature-dev-001",
    "conversation_id": "portal:chat:pm",
    "workflow_id": "feature-dev",
    "task_title": "做一次更长的 unattended soak",
    "task_body": "验证 feature-dev 在权限分离 + __cron helper 模型下能否无人手工 claim 跑到 terminal。REPO_PATH: /ABS/PATH/TO/TARGET REPO: /ABS/PATH/TO/TARGET BRANCH: chore/feature-dev-soak-001",
    "repo_path": "/ABS/PATH/TO/TARGET",
    "branch": "chore/feature-dev-soak-001",
    "metadata": {}
  }'
```

### 24.3 运行中只关注这几项

每轮 soak 只记录这 6 个字段：

1. `run_id`
2. `last_progress_event`
3. `current_step`
4. `current_agent`
5. `terminal_status` (`completed` / `failed` / `stuck`)
6. `manual_recovery_action`（如有）

如果卡住，再补这两类日志：

```bash
tail -n 120 "$INSTANCE_ROOT/state/antfarm/launchd-dashboard.stderr.log"
tail -n 120 "$INSTANCE_ROOT/state/logs/gateway.err.log"
```

### 24.4 可直接给部署机 LLM 的任务模板

把下面这段直接给部署机 LLM：

```text
目标：在当前实例上执行一次更长的 feature-dev unattended soak，并给出结构化结果。

要求：
1. 不做新的架构改造，不改 Spacebot UI。
2. 先做服务健康检查（openclaw/antfarm/spacebot）。
3. 确认 feature-dev 的 __cron jobs 存在；缺失则重建。
4. 发起一轮新的 feature-dev run（可用 Spacebot UI 或 API）。
5. 全程不做手工 step claim。
6. 若卡住，仅允许最小恢复动作（例如 launchctl kickstart），并记录动作与时间。
7. 输出必须是这 6 个字段：
   run_id, last_progress_event, current_step, current_agent, terminal_status, manual_recovery_action
8. 附上关键错误日志片段（若失败或卡住）。
```

### 24.5 当前阶段完成标准（已达成）

原稳定性门槛：

1. 至少连续 3 次 unattended run 在新权限分离模型下跑到 terminal
2. terminal 结果可被 Spacebot 面板稳定展示
3. 无需人工 `step claim`
4. 不依赖 `launchctl kickstart` 才能持续推进（允许偶发恢复，但不应成为每轮必需动作）
5. 不再出现长期 `runningAtMs` 残留导致的步骤停滞

当前判定（截至 `#19`）：

1. ✅ 条件 1 已满足（`#17/#18/#19` 连续 terminal）
2. ✅ 条件 3 已满足（上述 run 均无人工 `step claim`）
3. ⚠️ 条件 4/5 仍建议继续观察，但已不阻塞进入“输出契约收敛”

结论：本节门槛视为达成，后续进入输出契约阶段。

---

## 25. 当前目标与下一步工作

### 25.1 当前目标（2026-03）

当前目标已经从“能不能跑通”切换为：

1. 提升 unattended 运行稳定性和可重复性
2. 降低人工恢复依赖（尤其是 `kickstart`）
3. 为下一阶段“结果输出契约收敛”建立稳定运行基线

换句话说，当前不是继续扩功能，而是先把现有链路做稳。

### 25.2 下一步工作顺序

1. 进入输出契约收敛，先固定 `tester/reviewer` 的 terminal 字段
2. 保留低频 unattended soak（按 24.3 的 6 字段）做回归监控
3. 如再次卡住，优先归类为：
   - 运行时稳定性（cron/gateway/recovery）
   - workflow 输出契约问题（step 输出与 `Reply with` 不一致）
   - 目标 repo 任务适配问题（命令、分支、环境差异）
4. 契约收敛稳定后，再推进“4 库 + 测试”多仓闭环

### 25.3 当前阶段明确不优先做的事项

1. 继续改 Spacebot UI
2. 做 Dashboard 嵌入
3. 做自然语言自动触发 workflow
4. 立即切换到 `feature-dev-split` 作为主流程

以上事项可在稳定性基线达标后再推进。
