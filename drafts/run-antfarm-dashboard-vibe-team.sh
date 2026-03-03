#!/usr/bin/env bash
set -euo pipefail

export INSTANCE_ROOT="/Users/kris/instances/vibe-team"
export HOME="$INSTANCE_ROOT/antfarm-home"
export OPENCLAW_PROFILE="vibe-team"
export OPENCLAW_STATE_DIR="$INSTANCE_ROOT/state"
export OPENCLAW_CONFIG_PATH="$INSTANCE_ROOT/config/openclaw.json"
export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH:-}"

exec /opt/homebrew/bin/node "$INSTANCE_ROOT/antfarm/dist/server/daemon.js" 3333
