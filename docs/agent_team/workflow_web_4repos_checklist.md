# 4 仓闭环执行清单（填写版）

## 1. 运行信息

- run_id:
- owner:
- date:
- summary_target:

执行脚本：

`/Users/applychart/Desktop/vibe-team/drafts/run-4repo-closure.sh`

## 2. 仓库输入（逐项必填）

### 2.1 collector-repo

- repo_path:
- branch:
- build_cmd:
- test_cmd:
- pass_criteria:

### 2.2 shared-data-repo

- repo_path:
- branch:
- build_cmd:
- test_cmd:
- pass_criteria:

### 2.3 api-repo

- repo_path:
- branch:
- build_cmd:
- test_cmd:
- pass_criteria:

### 2.4 frontend-repo

- repo_path:
- branch:
- build_cmd:
- test_cmd:
- pass_criteria:

### 2.5 integration-test

- repo_path:
- branch:
- build_cmd:
- test_cmd:
- smoke_cmd:
- pass_criteria:

## 3. Gate 结果记录

- Collector Gate:
- Shared Gate:
- API Gate:
- FE Gate:
- Integration Gate:

## 4. 失败处置

- failed_stage:
- failed_step:
- key_error:
- retry_command:
- owner_action:

## 5. 最终结论

- final_status:
- artifacts:
- next_step:

## 6. 执行命令模板（流程优先）

### 6.1 标准执行（全阶段）

```bash
COLLECTOR_REPO="..." \
SHARED_REPO="..." \
API_REPO="..." \
FE_REPO="..." \
INTEGRATION_REPO="..." \
COLLECTOR_BRANCH='...' \
SHARED_BRANCH='...' \
API_BRANCH='...' \
FE_BRANCH='...' \
INTEGRATION_BRANCH='...' \
COLLECTOR_BUILD_CMD='...' \
COLLECTOR_TEST_CMD='...' \
SHARED_BUILD_CMD='...' \
SHARED_TEST_CMD='...' \
API_BUILD_CMD='...' \
API_TEST_CMD='...' \
FE_BUILD_CMD='...' \
FE_TEST_CMD='...' \
INTEGRATION_BUILD_CMD='...' \
INTEGRATION_TEST_CMD='...' \
INTEGRATION_SMOKE_CMD='true' \
/Users/applychart/Desktop/vibe-team/drafts/run-4repo-closure.sh
```

### 6.2 跳过阻塞阶段（不改仓先推进）

```bash
SKIP_STAGES='collector' \
COLLECTOR_REPO="..." \
SHARED_REPO="..." \
API_REPO="..." \
FE_REPO="..." \
INTEGRATION_REPO="..." \
COLLECTOR_BRANCH='...' \
SHARED_BRANCH='...' \
API_BRANCH='...' \
FE_BRANCH='...' \
INTEGRATION_BRANCH='...' \
COLLECTOR_BUILD_CMD='...' \
COLLECTOR_TEST_CMD='...' \
SHARED_BUILD_CMD='...' \
SHARED_TEST_CMD='...' \
API_BUILD_CMD='...' \
API_TEST_CMD='...' \
FE_BUILD_CMD='...' \
FE_TEST_CMD='...' \
INTEGRATION_BUILD_CMD='...' \
INTEGRATION_TEST_CMD='...' \
INTEGRATION_SMOKE_CMD='true' \
/Users/applychart/Desktop/vibe-team/drafts/run-4repo-closure.sh
```
