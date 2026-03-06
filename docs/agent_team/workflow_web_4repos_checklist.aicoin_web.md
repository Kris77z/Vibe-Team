# 4 仓闭环执行清单（aicoin-web 预填版）

## 1. 运行信息

- run_id: fourrepo-20260306-180052
- owner: applychart
- date: 2026-03-06
- summary_target: docs/agent_team/runs/fourrepo-20260306-180052/SUMMARY.md

## 2. 仓库输入（已预填）

### 2.1 collector-repo

- repo_path: /Users/applychart/Desktop/开发/aicoin-web/data-spider
- branch: feature/polymarket-heatmap
- build_cmd: go build ./...
- test_cmd: go test ./...
- pass_criteria: build=0 且 test=0

### 2.2 shared-data-repo

- repo_path: /Users/applychart/Desktop/开发/aicoin-web/go-core-lib
- branch: develop
- build_cmd: go build ./...
- test_cmd: go test ./...
- pass_criteria: build=0 且 test=0

### 2.3 api-repo

- repo_path: /Users/applychart/Desktop/开发/aicoin-web/go-web-api
- branch: feat/polymarket-heatmap
- build_cmd: go build ./...
- test_cmd: go test ./...
- pass_criteria: build=0 且 test=0

### 2.4 frontend-repo

- repo_path: /Users/applychart/Desktop/开发/aicoin-web/web
- branch: fix/restore-vip-translations
- build_cmd: yarn build
- test_cmd: yarn test --watch=false
- pass_criteria: build=0 且 test=0

### 2.5 integration-test

- repo_path: /Users/applychart/Desktop/开发/aicoin-web/aicoin-universal-web
- branch: feat/polymarket-heatmap-integration
- build_cmd: yarn build
- test_cmd: yarn lint
- smoke_cmd: true
- pass_criteria: build=0 且 test=0 且 smoke=0

## 3. 当前 Gate 结果（实跑）

- Collector Gate: FAIL
- Shared Gate: NOT RUN
- API Gate: NOT RUN
- FE Gate: NOT RUN
- Integration Gate: NOT RUN

## 4. 当前失败处置

- failed_stage: collector
- failed_step: test
- key_error:
  - tasks/stock/*.go 出现 `fmt.Errorf("... %w", err)` 参数类型不满足 error
  - internal/lock 测试依赖本地 Redis (`[::1]:6379`) 未启动
  - 多个业务测试断言失败（dex/config、dex/monitor、hyper）
- retry_command: 见第 5 节
- owner_action: 先修 collector 测试/依赖，再重跑 4 仓脚本

## 5. 可直接复跑命令（含代理）

```bash
export https_proxy=http://127.0.0.1:7890
export http_proxy=http://127.0.0.1:7890
export all_proxy=socks5://127.0.0.1:7890

TMPDIR=/tmp \
GOENV_VERSION=1.24.1 \
GOTOOLCHAIN=local \
GOPATH=/tmp/go \
GOMODCACHE=/tmp/go/pkg/mod \
GOCACHE=/tmp/go/build-cache \
COLLECTOR_REPO="/Users/applychart/Desktop/开发/aicoin-web/data-spider" \
SHARED_REPO="/Users/applychart/Desktop/开发/aicoin-web/go-core-lib" \
API_REPO="/Users/applychart/Desktop/开发/aicoin-web/go-web-api" \
FE_REPO="/Users/applychart/Desktop/开发/aicoin-web/web" \
INTEGRATION_REPO="/Users/applychart/Desktop/开发/aicoin-web/aicoin-universal-web" \
COLLECTOR_BRANCH='feature/polymarket-heatmap' \
SHARED_BRANCH='develop' \
API_BRANCH='feat/polymarket-heatmap' \
FE_BRANCH='fix/restore-vip-translations' \
INTEGRATION_BRANCH='feat/polymarket-heatmap-integration' \
COLLECTOR_BUILD_CMD='go build ./...' \
COLLECTOR_TEST_CMD='go test ./...' \
SHARED_BUILD_CMD='go build ./...' \
SHARED_TEST_CMD='go test ./...' \
API_BUILD_CMD='go build ./...' \
API_TEST_CMD='go test ./...' \
FE_BUILD_CMD='yarn build' \
FE_TEST_CMD='yarn test --watch=false' \
INTEGRATION_BUILD_CMD='yarn build' \
INTEGRATION_TEST_CMD='yarn lint' \
INTEGRATION_SMOKE_CMD='true' \
/Users/applychart/Desktop/vibe-team/drafts/run-4repo-closure.sh
```

## 6. 跳过 Collector 继续推进（不改 data-spider）

```bash
export https_proxy=http://127.0.0.1:7890
export http_proxy=http://127.0.0.1:7890
export all_proxy=socks5://127.0.0.1:7890

TMPDIR=/tmp \
GOENV_VERSION=1.24.1 \
GOTOOLCHAIN=local \
GOPATH=/tmp/go \
GOMODCACHE=/tmp/go/pkg/mod \
GOCACHE=/tmp/go/build-cache \
SKIP_STAGES='collector' \
COLLECTOR_REPO="/Users/applychart/Desktop/开发/aicoin-web/data-spider" \
SHARED_REPO="/Users/applychart/Desktop/开发/aicoin-web/go-core-lib" \
API_REPO="/Users/applychart/Desktop/开发/aicoin-web/go-web-api" \
FE_REPO="/Users/applychart/Desktop/开发/aicoin-web/web" \
INTEGRATION_REPO="/Users/applychart/Desktop/开发/aicoin-web/aicoin-universal-web" \
COLLECTOR_BRANCH='feature/polymarket-heatmap' \
SHARED_BRANCH='develop' \
API_BRANCH='feat/polymarket-heatmap' \
FE_BRANCH='fix/restore-vip-translations' \
INTEGRATION_BRANCH='feat/polymarket-heatmap-integration' \
COLLECTOR_BUILD_CMD='go build ./...' \
COLLECTOR_TEST_CMD='go test ./...' \
SHARED_BUILD_CMD='go build ./...' \
SHARED_TEST_CMD='go test ./...' \
API_BUILD_CMD='go build ./...' \
API_TEST_CMD='go test ./...' \
FE_BUILD_CMD='yarn build' \
FE_TEST_CMD='yarn test --watch=false' \
INTEGRATION_BUILD_CMD='yarn build' \
INTEGRATION_TEST_CMD='yarn lint' \
INTEGRATION_SMOKE_CMD='true' \
/Users/applychart/Desktop/vibe-team/drafts/run-4repo-closure.sh
```
