# Vibe-Team 部署手册（部署机启动 + 本机发命令）

## 1. 目标与边界

当前仅按 `Spacebot-only` 执行。

本手册目标：

1. 在部署机启动 Spacebot
2. 在本机通过 SSH 隧道访问部署机 API
3. 在不暴露公网端口的情况下完成联调

不包含：

1. 清空部署机历史目录
2. OpenClaw / Antfarm 相关流程

## 2. 标准目录与变量

部署机建议目录：

```text
$HOME/vibe-team/
└── spacebot/

$HOME/instances/vibe-team/
├── bin/
├── spacebot/
│   ├── config.toml
│   ├── data/
│   └── logs/
```

部署机环境变量：

```bash
export VIBE_TEAM_HOME="$HOME/vibe-team"
export INSTANCE_ROOT="$HOME/instances/vibe-team"
export SPACEBOT_HOME="$INSTANCE_ROOT/spacebot"
export SPACEBOT_DIR="$SPACEBOT_HOME/data"
export SPACEBOT_BIN="$INSTANCE_ROOT/bin/spacebot"
export SPACEBOT_CONFIG="$SPACEBOT_HOME/config.toml"
```

## 3. 部署机首次落地

### 3.1 依赖检查

```bash
rustc -V
cargo -V
protoc --version
bun -v
git --version
```

最低建议：

1. Rust `>= 1.88.0`
2. 其余命令可用

### 3.2 构建并安装二进制

```bash
mkdir -p "$VIBE_TEAM_HOME"
cd "$VIBE_TEAM_HOME"
# 已有仓库可跳过 clone
git clone https://github.com/spacedriveapp/spacebot.git
cd "$VIBE_TEAM_HOME/spacebot"

cargo build --release
mkdir -p "$INSTANCE_ROOT/bin"
cp target/release/spacebot "$SPACEBOT_BIN"
chmod +x "$SPACEBOT_BIN"
```

### 3.3 写入配置

```bash
mkdir -p "$SPACEBOT_HOME" "$SPACEBOT_DIR" "$SPACEBOT_HOME/logs"
cat > "$SPACEBOT_CONFIG" <<'EOF'
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

[defaults]
timezone = "Asia/Shanghai"

[[agents]]
id = "pm"
name = "Vibe Team PM"
EOF
```

关键配置要求：

1. `api.bind = "127.0.0.1"`
2. `api.port = 19898`
3. 至少一个 `[[agents]]`
4. `api_key` 使用环境变量引用（如 `env:OPENAI_AUTH_KEY`）

## 4. 部署机启动

### 4.1 前台启动（首轮推荐）

```bash
export OPENAI_AUTH_KEY="你的key"
SPACEBOT_DIR="$SPACEBOT_DIR" \
"$SPACEBOT_BIN" \
  --config "$SPACEBOT_CONFIG" \
  start \
  --foreground
```

### 4.2 后台启动（可选）

```bash
export OPENAI_AUTH_KEY="你的key"
nohup "$SPACEBOT_BIN" --config "$SPACEBOT_CONFIG" start --foreground \
  > "$SPACEBOT_HOME/logs/spacebot.log" 2>&1 &
```

## 5. 本机访问部署机

在本机执行 SSH 隧道（保持该终端不退出）：

```bash
ssh -L 19898:127.0.0.1:19898 <user>@<deploy-host>
```

随后在本机访问：

```bash
curl -sS http://127.0.0.1:19898/api/health
```

返回可解析 JSON 即表示链路通。

## 6. 本机发送命令方式

本机可通过两类方式发命令：

1. API 方式：通过 `127.0.0.1:19898` 调 Spacebot HTTP 接口
2. 平台消息方式：若配置了 Slack/Discord/Telegram，直接在对应平台对话

建议先用 API 健康检查，再进行业务命令联调。

## 7. 最小验收标准

满足以下 4 项即视为“可开始联调”：

1. 部署机进程持续运行
2. 本机 `curl /api/health` 成功
3. 能从本机触发至少 1 次有效请求
4. 日志中无持续性启动错误

## 8. 常见问题

1. `No space left on device`：释放部署机磁盘后重编译
2. `connection refused`：确认 Spacebot 正在部署机运行，且 SSH 隧道仍在线
3. `401/模型报错`：确认 `OPENAI_AUTH_KEY` 已导出且配置引用正确
4. 无响应：先看部署机日志，再查 `api.bind/api.port` 是否一致
