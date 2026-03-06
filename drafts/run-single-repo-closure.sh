#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUNS_ROOT="$WORKSPACE_ROOT/docs/agent_team/runs"

TARGET_REPO="${TARGET_REPO:-}"
TARGET_BRANCH="${TARGET_BRANCH:-}"
BUILD_CMD="${BUILD_CMD:-}"
TEST_CMD="${TEST_CMD:-}"
SMOKE_CMD="${SMOKE_CMD:-}"
CONTRACT_NOTE="${CONTRACT_NOTE:-}"

if [[ -z "$TARGET_REPO" || ! -d "$TARGET_REPO/.git" ]]; then
  echo "[single-closure] TARGET_REPO must be an absolute git repo path" >&2
  exit 1
fi

CURRENT_BRANCH="$(git -C "$TARGET_REPO" rev-parse --abbrev-ref HEAD)"
CURRENT_COMMIT="$(git -C "$TARGET_REPO" rev-parse HEAD)"

if [[ -n "$TARGET_BRANCH" && "$CURRENT_BRANCH" != "$TARGET_BRANCH" ]]; then
  echo "[single-closure] branch mismatch: expected '$TARGET_BRANCH', got '$CURRENT_BRANCH'" >&2
  exit 1
fi

mkdir -p "$RUNS_ROOT"

RUN_ID="single-$(date +%Y%m%d-%H%M%S)"
RUN_DIR="$RUNS_ROOT/$RUN_ID"
mkdir -p "$RUN_DIR"

now_iso() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

run_step() {
  local name="$1"
  local cmd="$2"
  local log_file="$3"

  if [[ -z "$cmd" ]]; then
    echo "[single-closure] SKIP $name (empty command)" | tee "$log_file"
    return 0
  fi

  echo "[single-closure] RUN $name: $cmd" | tee "$log_file"
  (
    cd "$TARGET_REPO"
    bash -lc "$cmd"
  ) >>"$log_file" 2>&1
}

write_summary() {
  local build_rc="$1"
  local test_rc="$2"
  local smoke_rc="$3"

  local gate_a="PASS"
  local gate_b="PASS"
  local gate_c="PASS"
  local final_status="PASS"
  local failed_stage=""

  if [[ -z "$CONTRACT_NOTE" ]]; then
    gate_a="FAIL"
    final_status="FAIL"
    failed_stage="A"
  fi

  if [[ "$build_rc" -ne 0 || "$test_rc" -ne 0 ]]; then
    gate_b="FAIL"
    final_status="FAIL"
    if [[ -z "$failed_stage" ]]; then
      failed_stage="B"
    fi
  fi

  if [[ -n "$SMOKE_CMD" && "$smoke_rc" -ne 0 ]]; then
    gate_c="FAIL"
    final_status="FAIL"
    if [[ -z "$failed_stage" ]]; then
      failed_stage="C"
    fi
  fi

  cat > "$RUN_DIR/SUMMARY.md" <<SUMMARY
# Single Repo Closure Summary

- run_id: $RUN_ID
- generated_at_utc: $(now_iso)
- target_repo: $TARGET_REPO
- target_branch: ${TARGET_BRANCH:-$CURRENT_BRANCH}
- target_commit: $CURRENT_COMMIT
- final_status: $final_status
- failed_stage: ${failed_stage:-none}

## Gate Result

- Gate A (contract freeze): $gate_a
- Gate B (build + test): $gate_b
- Gate C (smoke): $gate_c

## Commands

- BUILD_CMD: ${BUILD_CMD:-<empty>}
- TEST_CMD: ${TEST_CMD:-<empty>}
- SMOKE_CMD: ${SMOKE_CMD:-<empty>}

## Evidence Files

- env.txt
- contract.txt
- 01_git_status.log
- 02_build.log
- 03_test.log
- 04_smoke.log
SUMMARY
}

{
  echo "run_id=$RUN_ID"
  echo "generated_at_utc=$(now_iso)"
  echo "target_repo=$TARGET_REPO"
  echo "target_branch=${TARGET_BRANCH:-$CURRENT_BRANCH}"
  echo "target_commit=$CURRENT_COMMIT"
  echo "build_cmd=${BUILD_CMD:-<empty>}"
  echo "test_cmd=${TEST_CMD:-<empty>}"
  echo "smoke_cmd=${SMOKE_CMD:-<empty>}"
} > "$RUN_DIR/env.txt"

printf '%s\n' "$CONTRACT_NOTE" > "$RUN_DIR/contract.txt"

# Stage A evidence: repo status snapshot
(
  cd "$TARGET_REPO"
  echo "# git status --short --branch"
  git status --short --branch
  echo
  echo "# git log --oneline -n 3"
  git log --oneline -n 3
) > "$RUN_DIR/01_git_status.log" 2>&1

build_rc=0
test_rc=0
smoke_rc=0

if run_step "build" "$BUILD_CMD" "$RUN_DIR/02_build.log"; then
  build_rc=0
else
  build_rc=$?
fi

if run_step "test" "$TEST_CMD" "$RUN_DIR/03_test.log"; then
  test_rc=0
else
  test_rc=$?
fi

if run_step "smoke" "$SMOKE_CMD" "$RUN_DIR/04_smoke.log"; then
  smoke_rc=0
else
  smoke_rc=$?
fi

write_summary "$build_rc" "$test_rc" "$smoke_rc"

echo "[single-closure] run_id=$RUN_ID"
echo "[single-closure] summary=$RUN_DIR/SUMMARY.md"

if [[ "$build_rc" -ne 0 || "$test_rc" -ne 0 || ( -n "$SMOKE_CMD" && "$smoke_rc" -ne 0 ) || -z "$CONTRACT_NOTE" ]]; then
  exit 1
fi
