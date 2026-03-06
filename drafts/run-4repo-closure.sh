#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUNS_ROOT="$WORKSPACE_ROOT/docs/agent_team/runs"

STAGE_KEYS=("COLLECTOR" "SHARED" "API" "FE" "INTEGRATION")
STAGE_NAMES=("collector" "shared-data" "api" "frontend" "integration-test")

SKIP_STAGES_RAW="${SKIP_STAGES:-}"
SKIP_STAGES_NORMALIZED=","
if [[ -n "$SKIP_STAGES_RAW" ]]; then
  IFS=',' read -r -a _skip_items <<< "$SKIP_STAGES_RAW"
  for _item in "${_skip_items[@]}"; do
    _clean="$(echo "$_item" | tr '[:upper:]' '[:lower:]' | xargs)"
    if [[ -n "$_clean" ]]; then
      SKIP_STAGES_NORMALIZED="${SKIP_STAGES_NORMALIZED}${_clean},"
    fi
  done
fi

mkdir -p "$RUNS_ROOT"
RUN_ID="fourrepo-$(date +%Y%m%d-%H%M%S)"
RUN_DIR="$RUNS_ROOT/$RUN_ID"
mkdir -p "$RUN_DIR"

now_iso() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

run_cmd() {
  local repo="$1"
  local cmd="$2"
  local log_file="$3"

  if [[ -z "$cmd" ]]; then
    echo "[fourrepo] SKIP (empty command)" | tee "$log_file"
    return 0
  fi

  echo "[fourrepo] RUN: $cmd" | tee "$log_file"
  (
    cd "$repo"
    bash -lc "$cmd"
  ) >>"$log_file" 2>&1
}

should_skip_stage() {
  local key_lc="$1"
  local name_lc="$2"
  if [[ "$SKIP_STAGES_NORMALIZED" == *",$key_lc,"* ]] || [[ "$SKIP_STAGES_NORMALIZED" == *",$name_lc,"* ]]; then
    return 0
  fi
  return 1
}

final_status="PASS"
failed_stage="none"
failed_step="none"

echo "run_id=$RUN_ID" > "$RUN_DIR/env.txt"
echo "generated_at_utc=$(now_iso)" >> "$RUN_DIR/env.txt"
echo "skip_stages=${SKIP_STAGES_RAW:-<none>}" >> "$RUN_DIR/env.txt"

declare_stage_result_file="$RUN_DIR/stage_results.txt"
: > "$declare_stage_result_file"

for i in "${!STAGE_KEYS[@]}"; do
  key="${STAGE_KEYS[$i]}"
  stage="${STAGE_NAMES[$i]}"
  key_lc="$(echo "$key" | tr '[:upper:]' '[:lower:]')"

  repo_var="${key}_REPO"
  branch_var="${key}_BRANCH"
  build_var="${key}_BUILD_CMD"
  test_var="${key}_TEST_CMD"
  smoke_var="${key}_SMOKE_CMD"

  repo="${!repo_var:-}"
  branch="${!branch_var:-}"
  build_cmd="${!build_var:-}"
  test_cmd="${!test_var:-}"
  smoke_cmd="${!smoke_var:-}"

  if should_skip_stage "$key_lc" "$stage"; then
    echo "[fourrepo] SKIP stage=$stage (matched SKIP_STAGES='$SKIP_STAGES_RAW')"
    {
      echo
      echo "[$stage]"
      echo "repo=<skipped>"
      echo "branch=<skipped>"
      echo "commit=<skipped>"
      echo "build_cmd=<skipped>"
      echo "test_cmd=<skipped>"
      echo "smoke_cmd=<skipped>"
    } >> "$RUN_DIR/env.txt"
    echo "$stage=SKIP" >> "$declare_stage_result_file"
    continue
  fi

  if [[ -z "$repo" || ! -d "$repo/.git" ]]; then
    echo "[fourrepo] missing or invalid repo for $stage: $repo" >&2
    final_status="FAIL"
    failed_stage="$stage"
    failed_step="input"
    echo "$stage=FAIL" >> "$declare_stage_result_file"
    break
  fi

  current_branch="$(git -C "$repo" rev-parse --abbrev-ref HEAD)"
  current_commit="$(git -C "$repo" rev-parse HEAD)"

  if [[ -n "$branch" && "$current_branch" != "$branch" ]]; then
    echo "[fourrepo] branch mismatch for $stage: expected '$branch', got '$current_branch'" >&2
    final_status="FAIL"
    failed_stage="$stage"
    failed_step="branch"
    echo "$stage=FAIL" >> "$declare_stage_result_file"
    break
  fi

  {
    echo
    echo "[$stage]"
    echo "repo=$repo"
    echo "branch=${branch:-$current_branch}"
    echo "commit=$current_commit"
    echo "build_cmd=${build_cmd:-<empty>}"
    echo "test_cmd=${test_cmd:-<empty>}"
    echo "smoke_cmd=${smoke_cmd:-<empty>}"
  } >> "$RUN_DIR/env.txt"

  stage_state="PASS"

  build_log="$RUN_DIR/${i}_${stage}_build.log"
  test_log="$RUN_DIR/${i}_${stage}_test.log"
  smoke_log="$RUN_DIR/${i}_${stage}_smoke.log"

  if run_cmd "$repo" "$build_cmd" "$build_log"; then
    :
  else
    stage_state="FAIL"
    final_status="FAIL"
    failed_stage="$stage"
    failed_step="build"
  fi

  if [[ "$stage_state" == "PASS" ]]; then
    if run_cmd "$repo" "$test_cmd" "$test_log"; then
      :
    else
      stage_state="FAIL"
      final_status="FAIL"
      failed_stage="$stage"
      failed_step="test"
    fi
  fi

  if [[ "$stage_state" == "PASS" ]]; then
    if run_cmd "$repo" "$smoke_cmd" "$smoke_log"; then
      :
    else
      stage_state="FAIL"
      final_status="FAIL"
      failed_stage="$stage"
      failed_step="smoke"
    fi
  fi

  echo "$stage=$stage_state" >> "$declare_stage_result_file"

  if [[ "$stage_state" == "FAIL" ]]; then
    break
  fi

done

{
  echo "# 4 Repo Closure Summary"
  echo
  echo "- run_id: $RUN_ID"
  echo "- generated_at_utc: $(now_iso)"
  echo "- final_status: $final_status"
  echo "- failed_stage: $failed_stage"
  echo "- failed_step: $failed_step"
  echo
  echo "## Stage Results"
  echo
  while IFS= read -r line; do
    stage_name="${line%%=*}"
    stage_state="${line#*=}"
    echo "- $stage_name: $stage_state"
  done < "$declare_stage_result_file"
  echo
  echo "## Evidence"
  echo
  echo "- env.txt"
  echo "- *_build.log / *_test.log / *_smoke.log"
} > "$RUN_DIR/SUMMARY.md"

echo "[fourrepo] run_id=$RUN_ID"
echo "[fourrepo] summary=$RUN_DIR/SUMMARY.md"

if [[ "$final_status" != "PASS" ]]; then
  exit 1
fi
