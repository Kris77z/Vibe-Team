#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SPACEBOT_REPO="${SPACEBOT_REPO:-$WORKSPACE_ROOT/spacebot}"

if [[ ! -d "$SPACEBOT_REPO/.git" ]]; then
  echo "[run-gates] invalid repo path: $SPACEBOT_REPO" >&2
  exit 1
fi

if [[ -z "${BASH_FOR_GATE:-}" ]]; then
  if [[ -x /opt/homebrew/bin/bash ]]; then
    BASH_FOR_GATE="/opt/homebrew/bin/bash"
  else
    BASH_FOR_GATE="bash"
  fi
fi

if ! command -v "$BASH_FOR_GATE" >/dev/null 2>&1; then
  echo "[run-gates] bash executable not found: $BASH_FOR_GATE" >&2
  exit 1
fi

# Prefer rustup-managed toolchain when available.
if [[ -x "$HOME/.cargo/bin/cargo" && -x "$HOME/.cargo/bin/rustc" ]]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

cd "$SPACEBOT_REPO"

"$BASH_FOR_GATE" ./scripts/preflight.sh --ci
"$BASH_FOR_GATE" ./scripts/gate-pr.sh --ci --fast
