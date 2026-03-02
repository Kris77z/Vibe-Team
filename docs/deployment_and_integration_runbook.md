# Vibe-Team 部署与联调手册

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
  - 在刷新后恢复会话绑定的 workflow runs
  - 在 `Spacebot` 重启后恢复已持久化的 run binding，并对未结束 run 重挂 poller

当前限制：

- 真实联调还没有在目标 Mac 上做过一次完整端到端验证
- `Spacebot` 里“通过自然语言自动触发 workflow”的聊天动作还没正式接入，当前稳定入口是 API
- `Final result` 的 `changes/tests/review_decision/branch/pr_url` 仍是 best-effort 提取，依赖 Antfarm 最后一步输出格式
- 当前状态同步是 `polling`，不是 webhook push 或原生 step-level SSE 桥接

---

## 3. 推荐拓扑

推荐把三者部署在同一台 Mac，同机本地互通：

```text
浏览器
  -> Spacebot UI / API        http://127.0.0.1:19898
  -> Spacebot SSE             http://127.0.0.1:19898/api/events

Spacebot
  -> Antfarm CLI             antfarm workflow run ...
  -> Antfarm Dashboard API   http://127.0.0.1:3333/api/...

Antfarm
  -> OpenClaw state/config   ~/.openclaw 或 OPENCLAW_STATE_DIR
  -> OpenClaw runtime        已完成 onboard / gateway 可用
```

推荐端口：

- `Spacebot API/UI`: `19898`
- `Antfarm Dashboard`: `3333`
- `OpenClaw Gateway`: `18789`

建议第一版全部只监听本机回环地址，不要直接暴露公网。

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

推荐在目标 Mac 上保持类似结构：

```text
$HOME/dev/vibe-team/
├── openclaw/
├── antfarm/
├── spacebot/
└── target-project/
```

推荐环境变量：

```bash
export VIBE_TEAM_HOME="$HOME/dev/vibe-team"
export TARGET_PROJECT="$HOME/dev/target-project"
export OPENCLAW_STATE_DIR="$HOME/.openclaw"
export SPACEBOT_DIR="$HOME/.spacebot"
```

如果你希望把 OpenClaw/Antfarm 状态放到项目私有目录，也可以：

```bash
export OPENCLAW_STATE_DIR="$HOME/dev/vibe-runtime/openclaw"
```

但必须保证：

- `OpenClaw`
- `Antfarm`
- `Spacebot`

在运行时看到的是同一个 `OPENCLAW_STATE_DIR`。

---

## 6. 部署顺序

部署顺序不要打乱，推荐固定为：

1. `OpenClaw`
2. `Antfarm`
3. `Spacebot`
4. 联调与 smoke test

原因：

- `Antfarm` 默认会依赖 `~/.openclaw/openclaw.json`
- 先做 `OpenClaw onboard`，能保证 OpenClaw 的状态目录和配置先存在
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

当前项目更推荐方案 B，因为你本地是按源码集成推进的。

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
openclaw gateway --port 18789 --verbose
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
export SPACEBOT_ANTFARM_CLI_PATH="$VIBE_TEAM_HOME/antfarm/dist/cli/cli.js"
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

- `Antfarm` 默认状态目录不是项目目录，而是：
  - `~/.openclaw/antfarm`
  - `~/.openclaw/workspaces/workflows`
- 如果你设置了 `OPENCLAW_STATE_DIR`，这些目录会跟着改
- `Antfarm` 当前工作流大量依赖 prompt 中的 `REPO` 信息
- `Spacebot -> Antfarm` 触发接口当前没有独立 `repo_path` 字段，所以你要把 repo 路径明确写进任务正文

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

默认实例目录：

- `~/.spacebot`
- 或 `SPACEBOT_DIR` 指向的目录

创建配置文件：

```bash
mkdir -p "$SPACEBOT_DIR"
cat > "$SPACEBOT_DIR/config.toml" <<'EOF'
[llm]
openrouter_key = "env:OPENROUTER_API_KEY"

[defaults.routing]
channel = "anthropic/claude-sonnet-4"
worker = "anthropic/claude-sonnet-4"

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
cat > "$HOME/run-spacebot-antfarm.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

export VIBE_TEAM_HOME="$HOME/dev/vibe-team"
export SPACEBOT_DIR="$HOME/.spacebot"
export OPENCLAW_STATE_DIR="$HOME/.openclaw"

export OPENROUTER_API_KEY="your-key"

export SPACEBOT_ANTFARM_DASHBOARD_URL="http://127.0.0.1:3333"
export SPACEBOT_ANTFARM_CLI_PATH="antfarm"
export SPACEBOT_ANTFARM_WORKDIR="$HOME/dev"

exec "$VIBE_TEAM_HOME/spacebot/target/release/spacebot" --config "$SPACEBOT_DIR/config.toml" start --foreground
EOF

chmod +x "$HOME/run-spacebot-antfarm.sh"
```

可选环境变量说明：

- `SPACEBOT_ANTFARM_DASHBOARD_URL`
  - 必填
  - 例如：`http://127.0.0.1:3333`
- `SPACEBOT_ANTFARM_CLI_PATH`
  - 可选
  - 默认是 `antfarm`
  - 如果没做 `npm link`，可指向 `dist/cli/cli.js`
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
"$HOME/run-spacebot-antfarm.sh"
```

或者不写脚本，直接当前 shell 启动：

```bash
cd "$VIBE_TEAM_HOME/spacebot"
export SPACEBOT_DIR="$HOME/.spacebot"
export OPENCLAW_STATE_DIR="$HOME/.openclaw"
export SPACEBOT_ANTFARM_DASHBOARD_URL="http://127.0.0.1:3333"
export SPACEBOT_ANTFARM_CLI_PATH="antfarm"
./target/release/spacebot --config "$SPACEBOT_DIR/config.toml" start --foreground
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
    "task_body": "REPO: '"$TARGET_PROJECT"'\nBRANCH: feature/checkin\n需求：实现用户连续签到、断签重置、积分累加、最小可验证测试。",
    "metadata": {}
  }'
```

说明：

- `conversation_id` 这里用 `portal:chat:<agent_id>`，其中 `pm` 要替换成你真实的 agent id
- 当前接口没有单独的 `repo_path` 字段，所以 `task_body` 里必须显式写 `REPO: /absolute/path`
- 这是当前实现的现实约束，不要省略

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

---

## 15. 数据落点

### 15.1 OpenClaw / Antfarm

默认：

- `~/.openclaw/openclaw.json`
- `~/.openclaw/antfarm/`
- `~/.openclaw/workspaces/workflows/`

可被这些环境变量覆盖：

- `OPENCLAW_STATE_DIR`
- `OPENCLAW_CONFIG_PATH`

### 15.2 Spacebot

默认：

- `~/.spacebot/config.toml`
- `~/.spacebot/logs/`
- `~/.spacebot/agents/...`
- `~/.spacebot/data/secrets.redb`
- `~/.spacebot/anthropic_oauth.json`

可被这个环境变量覆盖：

- `SPACEBOT_DIR`

### 15.3 本次新增的 workflow binding 持久化

`Spacebot` 会把 workflow run binding 落到各 agent SQLite 中的：

- `workflow_run_bindings`

这张表用于：

- conversation 刷新恢复
- `Spacebot` 重启后的 run 恢复

---

## 16. 升级与重启建议

### 16.1 重启顺序

建议：

1. 停 `Spacebot`
2. 确认 `Antfarm Dashboard` 在不在
3. 如需，重启 `Antfarm Dashboard`
4. 再起 `Spacebot`

原因：

- `Spacebot` 启动时会尝试恢复已持久化的 workflow bindings
- 如果此时 Dashboard 不可用，未结束 run 的恢复与重挂 poller 会不完整

### 16.2 升级 Antfarm

在 `antfarm/` 目录：

```bash
npm install
npm run build
antfarm install
```

### 16.3 升级 Spacebot

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

## 17. 已知注意事项

1. 当前 `feature-dev` workflow 仍是单 repo 多角色流水线，不是严格前后端物理隔离版。
2. 当前接口还没有独立的 `repo_path` 参数，repo 路径必须明确写进任务正文。
3. 当前最终结果提取依赖 Antfarm 最后完成步骤的输出格式，严格结构化产物还需要后续继续规范。
4. 当前 workflow 更新依赖 polling，不是 Dashboard 主动推送。
5. 当前 `Spacebot` 里的 workflow 入口已经足够支撑联调，但不建议继续先做 UI 深挖，优先做真实部署验证。

---

## 18. 推荐的第一次完整联调脚本

如果你只想跑最短路径，按这个顺序即可：

```bash
export VIBE_TEAM_HOME="$HOME/dev/vibe-team"
export TARGET_PROJECT="$HOME/dev/target-project"
export OPENCLAW_STATE_DIR="$HOME/.openclaw"
export SPACEBOT_DIR="$HOME/.spacebot"

cd "$VIBE_TEAM_HOME/openclaw"
pnpm install
pnpm ui:build
pnpm build
pnpm openclaw onboard --install-daemon

cd "$VIBE_TEAM_HOME/antfarm"
npm install
npm run build
npm link
antfarm install

cd "$VIBE_TEAM_HOME/spacebot/interface"
bun install
bun run build

cd "$VIBE_TEAM_HOME/spacebot"
cargo build --release

cat > "$SPACEBOT_DIR/config.toml" <<'EOF'
[llm]
openrouter_key = "env:OPENROUTER_API_KEY"

[defaults.routing]
channel = "anthropic/claude-sonnet-4"
worker = "anthropic/claude-sonnet-4"

[api]
enabled = true
bind = "127.0.0.1"
port = 19898

[[agents]]
id = "pm"
EOF

export SPACEBOT_ANTFARM_DASHBOARD_URL="http://127.0.0.1:3333"
export SPACEBOT_ANTFARM_CLI_PATH="antfarm"

./target/release/spacebot --config "$SPACEBOT_DIR/config.toml" start --foreground
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
    "task_body": "REPO: '"$TARGET_PROJECT"'\nBRANCH: chore/antfarm-smoke\n需求：只做最小 smoke run，验证 Spacebot -> Antfarm -> OpenClaw 主链路。",
    "metadata": {}
  }'
```

浏览器打开：

```text
http://127.0.0.1:19898
```

到这里如果你能看到 workflow 状态变化，这条链就已经基本打通了。
