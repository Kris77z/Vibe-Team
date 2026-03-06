#!/usr/bin/env bash
set -euo pipefail

# Spacebot-only 启动脚本（前台）
# 用法：
#   OPENAI_AUTH_KEY=... ./drafts/run-spacebot-vibe-team.sh

export INSTANCE_ROOT="${INSTANCE_ROOT:-$HOME/instances/vibe-team}"
export SPACEBOT_HOME="${SPACEBOT_HOME:-$INSTANCE_ROOT/spacebot}"
export SPACEBOT_DIR="${SPACEBOT_DIR:-$SPACEBOT_HOME/data}"
export SPACEBOT_BIN="${SPACEBOT_BIN:-$INSTANCE_ROOT/bin/spacebot}"
export SPACEBOT_CONFIG="${SPACEBOT_CONFIG:-$SPACEBOT_HOME/config.toml}"

if [[ ! -x "$SPACEBOT_BIN" ]]; then
  echo "[run-spacebot] missing executable: $SPACEBOT_BIN" >&2
  exit 1
fi

if [[ ! -f "$SPACEBOT_CONFIG" ]]; then
  echo "[run-spacebot] missing config: $SPACEBOT_CONFIG" >&2
  exit 1
fi

mkdir -p "$SPACEBOT_DIR"

export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH:-}"

exec "$SPACEBOT_BIN" \
  --config "$SPACEBOT_CONFIG" \
  start \
  --foreground
