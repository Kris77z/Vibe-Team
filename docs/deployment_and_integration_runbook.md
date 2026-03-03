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
