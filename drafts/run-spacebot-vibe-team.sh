#!/usr/bin/env bash
set -euo pipefail

export INSTANCE_ROOT="/Users/kris/instances/vibe-team"
export SPACEBOT_DIR="$INSTANCE_ROOT/spacebot"
export OPENCLAW_STATE_DIR="$INSTANCE_ROOT/state"
export OPENAI_AUTH_KEY="${OPENAI_AUTH_KEY:?OPENAI_AUTH_KEY is required}"
export SPACEBOT_ANTFARM_DASHBOARD_URL="http://127.0.0.1:3333"
export SPACEBOT_ANTFARM_CLI_PATH="$INSTANCE_ROOT/bin/antfarm-vibe-team"
export SPACEBOT_ANTFARM_WORKDIR="/Users/kris/Desktop/Dev"
export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH:-}"
export https_proxy="http://127.0.0.1:7890"
export http_proxy="http://127.0.0.1:7890"
export all_proxy="socks5://127.0.0.1:7890"
export NO_PROXY="127.0.0.1,localhost"
export no_proxy="127.0.0.1,localhost"

exec "$INSTANCE_ROOT/bin/spacebot" \
  --config "$SPACEBOT_DIR/config.toml" \
  start \
  --foreground
