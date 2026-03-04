# Vibe-Team 敏捷智能体团队架构方案 (Agile Agent Team)

> 文档拆分说明：主入口见 [docs/agent_team/README.md](/Users/applychart/Desktop/vibe-team/docs/agent_team/README.md)。  
> 本文主要保留“架构原则与设计基线”，推进记录请写入 [docs/agent_team/progress.md](/Users/applychart/Desktop/vibe-team/docs/agent_team/progress.md)。

## 1. 架构愿景与设计理念

本方案目标是搭建一个由 AI Agent 协作完成需求分析、开发、验证、测试与交付的「虚拟敏捷团队」。

人类在系统中扮演两个角色：

1. 产品发起人
2. 最终验收者

整体设计不再采用单会话阻塞式大循环，而是采用“前台常驻 + 后台异步施工”的模式：

- `Spacebot` 负责前台对话、澄清需求、汇总进度、回传结果
- `Antfarm` 负责多 Agent 工作流编排、状态机推进、失败重试
- `OpenClaw` 负责底层执行能力与本地 agent runtime

### 核心设计原则

1. **前台常驻，后台施工**：PM Agent 不能被长时间编译、测试、改代码卡住。
2. **工作流可恢复**：任务、步骤、故事拆分、重试次数都要落盘持久化。
3. **职责隔离要落到工作区和工具权限**：不能只靠 prompt 约束。
4. **统一操作入口**：默认在 `Spacebot` 中看任务状态和结果，减少多面板切换。
5. **Dashboard 可选，不是必选**：`Antfarm Dashboard` 作为调试和运维观测面板保留，但不作为主入口。

---

## 2. 核心组件栈

本方案采用“三层协作”，但不是三个完全平级的独立系统，而是一个主入口加一套工作流内核：

* **入口与指挥中枢 (Router/PM): [Spacebot](https://github.com/spacedriveapp/spacebot)**
  * **定位**：面向用户的统一交互入口。
  * **作用**：提供 WebChat/Channel 交互界面，负责需求澄清、触发任务、展示进度摘要、接收最终结果。
  * **要求**：即使后台已有长任务在运行，前台对话仍保持可响应。

* **敏捷工作流内核 (Workflow Engine): [Antfarm](https://github.com/snarktank/antfarm)**
  * **定位**：多 Agent 流水线编排器。
  * **作用**：基于 YAML 定义 Planner、Setup、Developer、Verifier、Tester、Reviewer 等角色，使用 SQLite + Cron 推进步骤、重试失败节点、记录运行历史。
  * **注意**：`Antfarm` 不是完全脱离 `OpenClaw` 的独立层，而是运行在 `OpenClaw` agent/runtime 之上。

* **执行底座 (Execution Runtime): [OpenClaw](https://github.com/anomalyco/openclaw)**
  * **定位**：本地 agent 执行环境。
  * **作用**：提供 workspace、工具权限、文件读写、命令执行、browser、cron、agent 配置等基础能力。
  * **结论**：在实现层面，`OpenClaw` 是 `Antfarm` 的运行底座，不建议把两者拆成互不感知的平级系统。

---

## 2.1 关于“敏捷团队”的可配置性

这套方案里，“敏捷团队”不是固定编制，也不是规定必须有哪几个角色。

真正稳定的只有三层架构职责：

1. `Spacebot` 作为统一入口
2. `Antfarm` 作为工作流编排器
3. `OpenClaw` 作为执行底座

在这个架构之上，下面这些内容都可以按项目需要自定义：

1. 角色数量
2. 角色名称
3. 步骤顺序
4. 串行还是有限并行
5. 失败重试规则
6. 人工介入点
7. 每个角色的 workspace
8. 每个角色的工具权限
9. 每个角色的输入输出契约

因此：

- `feature-dev` 不是唯一标准答案
- `planner/setup/developer/tester/reviewer` 只是当前可用模板
- 后续完全可以改造成更贴合团队实际的版本，例如：
  - 极简三角色版
  - 前后端分工版
  - 带安全审查或发布管理的重流程版

真正需要控制的，不是“角色一定长什么样”，而是：

1. 每个角色职责是否清晰
2. 上下游输入输出是否稳定
3. 权限和 workspace 是否与职责匹配
4. 当前 workflow 复杂度是否超过引擎承载范围

换句话说，本方案定义的是“可配置的虚拟团队协作框架”，不是“固定人数的组织图”。

---

## 3. 目录与状态规划

业务仓库建议继续保持在当前工作区下，但运行时状态不要再假设落在 `antfarm-workers/.antfarm`，而应明确区分：

```text
vibe-team/
├── spacebot/                 # Web 交互入口 / PM 中枢
├── antfarm/                  # Antfarm 源码或安装副本
├── openclaw/                 # OpenClaw 源码或安装副本
└── target-project/           # 真实业务代码仓库
    ├── frontend/
    └── backend/
```

运行时状态分两类：

1. **业务代码工作区**
   - `target-project/`
   - 由开发、验证、测试角色围绕同一仓库协作

2. **Agent 运行时状态**
   - 默认位于 `~/.openclaw/`
   - 包含 `Antfarm` 的 SQLite、workflow 安装目录、agent workspace、OpenClaw agent 配置
   - 如需项目级隔离，可通过 `OPENCLAW_STATE_DIR` 显式改到项目私有目录

### 关于前后端隔离

“frontend 只给前端 agent 看，backend 只给后端 agent 看”这个目标是合理的，但当前不能只靠目录命名达成，必须落到两层：

1. workflow 中拆分独立角色，例如 `frontend-dev`、`backend-dev`、`qa`
2. 给不同角色配置不同 workspace 和工具权限

第一版 MVP 不建议一开始就做最强隔离。更现实的路径是：

1. 先打通统一入口和工作流链路
2. 再把 `feature-dev` workflow 改造成前后端分工版本

---

## 4. 一次完整的敏捷迭代 (Workflow) 演示

**目标**：开发一个包含积分奖励的“用户连续签到功能”。

1. **需求澄清阶段**
   * 人类在 `Spacebot` 中输入：“我要在 `target-project` 里加个签到功能。”
   * `Spacebot (PM Agent)` 继续追问关键边界，例如断签规则、积分策略、是否需要登录态、是否有 UI 入口。
   * 人类补齐约束后，PM 生成结构化需求说明。

2. **任务下发阶段**
   * `Spacebot` 在后台触发 `Antfarm workflow run ...`。
   * 前台立即回复：“任务已启动，我会持续回传进度和关键节点。”
   * 用户后续默认直接在 `Spacebot` 里查看：
     - 当前阶段
     - 已完成 stories
     - 阻塞点
     - 测试状态
     - 最终交付结果

3. **开发流水线**
   * `Planner` 拆 Story。
   * `Setup` 建立分支、探测 build/test 命令、确认基线。
   * `Developer` 分 story 实现。
   * `Verifier` 核对实现是否满足 story 和验收标准。
   * `Tester` 运行集成或端到端测试。
   * `Reviewer` 进行最终审查。

4. **进度回传阶段**
   * `Antfarm` 在关键事件上向 `Spacebot` 回传运行状态。
   * `Spacebot` 将原始事件整理成适合人类阅读的状态摘要，而不是直接暴露底层步骤日志。

5. **最终交付阶段**
   * 流水线全部通过后，`Spacebot` 在对话中给出最终结果：
     - 功能完成情况
     - 测试通过情况
     - 变更摘要
     - PR / commit / 分支信息
     - 待人工验收项

### 关于 Antfarm Dashboard

`Antfarm Dashboard` 不是必须步骤。

推荐定位如下：

- **主视图**：`Spacebot`
- **辅助视图**：`Antfarm Dashboard`

也就是说：

1. 普通使用者默认只需要打开 `Spacebot`
2. `Antfarm Dashboard` 仅在以下场景使用：
   - 调试 workflow
   - 观察底层 step 流转
   - 排查卡死、重试、cron、agent 健康问题
   - 做运维级别的诊断

如果后续把 `Antfarm` 的运行摘要、事件流和 run 详情聚合进 `Spacebot`，则可以把 `Antfarm Dashboard` 彻底降级为“工程维护工具”，而不是产品主界面。

---

## 5. 实施落地步骤 (Action Plan)

为了实现该架构 MVP，建议按“本机先开发，部署后置”的方式推进。

部署与联调执行细节请直接参考：

- `docs/deployment_and_integration_runbook.md`

当前约束如下：

1. 当前项目仓库只负责方案设计、workflow 设计、集成开发与本地验证
2. `OpenClaw` / `Spacebot` / `Antfarm` 的真实部署将在另一台 Mac 上完成
3. 因此本阶段不把“安装成功”或“部署跑通”作为阻塞条件，而是优先完成可迁移的配置、文档、workflow 与集成代码

建议分三阶段实施。

### Phase 1: 建立统一入口设计 (Spacebot)

1. 明确 `Spacebot` 是唯一主入口，后续所有进度、结果、验收回执都优先回到这里。
2. 设计 `Spacebot` 侧需要暴露的能力：
   - 需求澄清
   - 任务触发
   - run 状态摘要
   - 最终结果回传
3. 定义 `Spacebot` 与 `Antfarm` 之间的最小集成接口，而不是在当前机器上强行跑完整部署。

### Phase 2: 完成工作流与集成开发 (OpenClaw + Antfarm)

1. 设计并改造 `feature-dev` workflow，使其至少能完成：
   - 需求拆分
   - 基线检查
   - story 实现
   - 验证
   - 测试
   - 最终审查
2. 补齐前后端分工版本 workflow 的 YAML 草案、agent 文件清单、结构化输出契约。
3. 开发 `Spacebot -> Antfarm` 的触发与状态回传逻辑。
4. 将部署依赖项单独沉淀为 checklist，留待目标 Mac 上执行。

### Phase 3: 部署准备与迁移验证 (Deferred)

这一阶段不在当前机器完成，只为后续目标 Mac 做准备：

1. 整理部署清单：
   - Node 版本要求
   - OpenClaw 配置项
   - Spacebot 配置项
   - 模型 Provider 凭据
   - workflow 安装步骤
2. 在目标 Mac 上完成真实安装和运行。
3. 验证 `Spacebot -> Antfarm -> OpenClaw` 全链路。
4. 可选保留 `Antfarm Dashboard` 作为调试入口，但不再把它定义为主流程必经环节。

### 当前阶段目标（2026-03）

结合最新实机联调结果，当前阶段目标已从“主链是否可跑”切换为“主链是否可稳定重复运行”。

当前优先级：

1. 保持 `feature-dev` 在权限分离模型下的 unattended 可重复推进
2. 降低对人工恢复动作（如 `kickstart`）的依赖
3. 在稳定性达标后再推进输出契约收敛和分工版 workflow

当前不优先事项：

1. 继续扩 UI 功能
2. Dashboard 嵌入
3. 过早切换到复杂并行分工流程

执行细节以 `docs/deployment_and_integration_runbook.md` 的最新章节为准。

---

## 6. MVP 边界建议

为了尽快落地，当前仓库阶段的 MVP 先做到以下能力即可：

1. 用户在 `Spacebot` 中提需求
2. `Spacebot` 能触发 `Antfarm` 跑标准 workflow
3. `Spacebot` 能展示 run 状态和关键事件摘要
4. 任务完成后，`Spacebot` 能回传测试结果和变更摘要

这里的“能触发”“能展示”在当前阶段可以先通过本地开发代码、配置约定、模拟数据或最小验证链路完成，不要求本机承担最终部署职责。

以下能力放到第二阶段：

1. 前后端 agent 严格物理隔离
2. 契约驱动的 frontend/backend 双流水线
3. 完整的 dashboard 嵌入式视图
4. 更细粒度的审批、回滚和人工介入节点

---

## 7. 前后端分工 Workflow 草案

当 MVP 跑通后，可以把默认的 `feature-dev` workflow 进一步拆成更符合“敏捷团队分工”的版本。

建议角色如下：

1. `planner`
   - 负责需求拆解
   - 输出 stories、验收标准、前后端边界、接口改动点

2. `backend-dev`
   - 只负责后端代码、数据结构、接口实现、后端测试
   - 默认 workspace 聚焦 `target-project/backend`

3. `frontend-dev`
   - 只负责前端页面、交互、状态管理、前端测试
   - 默认 workspace 聚焦 `target-project/frontend`

4. `contract-reviewer`
   - 负责检查前后端契约是否一致
   - 重点检查接口字段、状态码、空值语义、分页结构、错误码

5. `integrator`
   - 负责把前后端改动放回同一分支验证集成结果
   - 只在后段介入，避免前期多人同时改一套文件

6. `tester`
   - 跑集成测试和端到端测试
   - 对用户路径做最终验收

7. `reviewer`
   - 做最终审查，确认无明显遗漏、无越权改动、无未验证风险

### 推荐流水线

```text
plan
-> backend-implement
-> contract-review
-> frontend-implement
-> integrate
-> test
-> review
```

如果需求更复杂，也可以改成：

```text
plan
-> backend-implement
-> frontend-implement
-> contract-review
-> integrate
-> test
-> review
```

但第二种更依赖契约先写清楚，否则前后端会互相等待。

### 推荐输入输出契约

`planner` 应至少产出以下结构化内容：

1. `REPO`
2. `BRANCH`
3. `STORIES_JSON`
4. `API_CONTRACT`
5. `FRONTEND_SCOPE`
6. `BACKEND_SCOPE`
7. `TEST_PLAN`

其中 `API_CONTRACT` 建议使用稳定格式，例如 Markdown 或 JSON，至少包含：

1. 接口路径
2. 请求方法
3. 请求参数
4. 响应字段
5. 错误响应
6. 字段是否可空
7. 分页或游标规则

### 实施方式建议

第一步不要追求真正“双仓并行开发”，而是采用下面这个更稳的版本：

1. `planner` 在同一分支上完成拆解
2. `backend-dev` 先完成接口与测试
3. `contract-reviewer` 确认接口契约冻结
4. `frontend-dev` 再基于冻结契约完成功能
5. `integrator` 做最终联调

这样虽然牺牲了一点并行度，但大幅降低了前后端互相返工的概率。

---

## 8. 实施注意事项

下面这些点在落地时最需要提前控制。

### 1. 不要把隔离只写在文档里

如果只是写“frontend agent 不看 backend”，但实际上所有 agent 都指向同一个 workspace，并且都有完整读写权限，那隔离就是假的。

要真正做到隔离，至少要同时满足：

1. workspace 路径不同
2. 工具权限不同
3. workflow 输入不同
4. 验证节点独立存在

### 2. 先冻结契约，再放大并行度

前后端同时开工只有在契约足够稳定时才划算。

否则会出现：

1. backend 改字段名
2. frontend 按旧字段开发
3. tester 才发现联调失败
4. 整条流水线返工

因此在第二阶段，宁可先做“后端先行 + 契约冻结 + 前端跟进”。

### 3. 不要让 PM 直接暴露底层事件流

`Antfarm` 的 step 事件适合工程排障，不适合直接给最终用户看。

`Spacebot` 最好做一层状态摘要，把底层事件转换成下面这种信息：

1. 当前处于哪个阶段
2. 是否阻塞
3. 阻塞原因
4. 最近一次失败发生在哪里
5. 需要人类决策还是系统会自动重试

### 4. 明确 Node 运行时要求

`Antfarm` 依赖真实 `Node.js >= 22`。

如果环境里混用了 Bun 的 `node` wrapper，工作流可能在最开始就失败。这个问题应在目标部署机安装阶段先做探测，不要等到 run 失败后再排查。

### 5. Dashboard 嵌入不应阻塞 MVP

“在 Spacebot 看 dashboard”可以做，但不应该成为第一阶段前置条件。

更合理的优先级是：

1. 先在 `Spacebot` 里看摘要
2. 再提供跳转到 `Antfarm Dashboard`
3. 最后才做嵌入式 run 详情页或 iframe 级整合

原因很简单：摘要是工作流必需能力，嵌入式 dashboard 只是体验增强。

### 6. 角色越多，越要控制上下文格式

多 agent 协作最常见的问题不是“模型不够强”，而是上下游输出不稳定。

因此每个关键节点都应该尽量要求结构化输出，例如：

1. `STATUS: done|retry|failed`
2. `CHANGES:`
3. `ISSUES:`
4. `TESTS:`
5. `NEXT_ACTION:`

只靠自由文本，工作流越长越容易断。

### 7. 人工介入点要明确

不是所有失败都该自动重试。

建议至少预留以下人工介入点：

1. 需求边界不清
2. 基线测试本身就是红的
3. 需要数据库迁移但环境不完整
4. 需要真实第三方密钥
5. 测试结果存在高不确定性

### 8. 第一个版本不要强求“完全自治”

第一版最容易失败的目标就是“全自动、零人工、全链路一次成功”。

现实一点的目标应该是：

1. 系统能稳定发起 workflow
2. 能稳定推进主要步骤
3. 失败时能清楚暴露原因
4. 人能在 `Spacebot` 中接管判断

只要这四点成立，方案就已经有工程价值。

### 9. 开发机与部署机职责要分开

既然当前仓库所在机器只负责开发，那文档、代码和配置就要避免混入“必须在本机完成部署”的假设。

建议明确分层：

1. **开发机负责**
   - 文档
   - workflow 设计
   - 集成代码
   - 配置模板
   - 部署 checklist

2. **部署机负责**
   - OpenClaw 安装
   - Spacebot 运行
   - Antfarm 安装
   - provider 凭据注入
   - 真正的 run 验证

如果这两层不分开，后面很容易把“开发完成”和“部署完成”混成一件事，导致节奏失控。

---

## 9. 可执行 YAML 草案

下面给出一版接近可执行的 workflow 设计骨架，用于指导后续真正落地到 `Antfarm`。

这不是最终可直接安装版本，但它已经尽量贴近当前 `workflow.yml` 的写法。

```yaml
id: feature-dev-split
name: Frontend Backend Split Workflow
version: 1

polling:
  model: default
  timeoutSeconds: 120

agents:
  - id: planner
    name: Planner
    role: analysis
    workspace:
      baseDir: agents/planner
      files:
        AGENTS.md: agents/planner/AGENTS.md
        SOUL.md: agents/planner/SOUL.md
        IDENTITY.md: agents/planner/IDENTITY.md

  - id: backend
    name: Backend Developer
    role: coding
    workspace:
      baseDir: agents/backend
      files:
        AGENTS.md: agents/backend/AGENTS.md
        SOUL.md: agents/backend/SOUL.md
        IDENTITY.md: agents/backend/IDENTITY.md

  - id: frontend
    name: Frontend Developer
    role: coding
    workspace:
      baseDir: agents/frontend
      files:
        AGENTS.md: agents/frontend/AGENTS.md
        SOUL.md: agents/frontend/SOUL.md
        IDENTITY.md: agents/frontend/IDENTITY.md

  - id: contract
    name: Contract Reviewer
    role: verification
    workspace:
      baseDir: agents/contract
      files:
        AGENTS.md: agents/contract/AGENTS.md
        SOUL.md: agents/contract/SOUL.md
        IDENTITY.md: agents/contract/IDENTITY.md

  - id: integrator
    name: Integrator
    role: coding
    workspace:
      baseDir: agents/integrator
      files:
        AGENTS.md: agents/integrator/AGENTS.md
        SOUL.md: agents/integrator/SOUL.md
        IDENTITY.md: agents/integrator/IDENTITY.md

  - id: tester
    name: Tester
    role: testing
    workspace:
      baseDir: agents/tester
      files:
        AGENTS.md: agents/tester/AGENTS.md
        SOUL.md: agents/tester/SOUL.md
        IDENTITY.md: agents/tester/IDENTITY.md

  - id: reviewer
    name: Reviewer
    role: analysis
    workspace:
      baseDir: agents/reviewer
      files:
        AGENTS.md: agents/reviewer/AGENTS.md
        SOUL.md: agents/reviewer/SOUL.md
        IDENTITY.md: agents/reviewer/IDENTITY.md

steps:
  - id: plan
    agent: planner
    input: |
      Decompose the task into backend stories, frontend stories, API contract, and test plan.

      TASK:
      {{task}}

      Reply with:
      STATUS: done
      REPO: /path/to/repo
      BRANCH: feature-branch-name
      API_CONTRACT: ...
      BACKEND_STORIES_JSON: [ ... ]
      FRONTEND_STORIES_JSON: [ ... ]
      TEST_PLAN: ...
    expects: "STATUS: done"

  - id: backend-implement
    agent: backend
    input: |
      Implement backend changes only.

      REPO: {{repo}}
      BRANCH: {{branch}}
      API_CONTRACT: {{api_contract}}
      BACKEND_STORIES_JSON: {{backend_stories_json}}

      Rules:
      1. Only modify backend files
      2. Add or update backend tests
      3. Do not modify frontend code except generated contract artifacts if explicitly required

      Reply with:
      STATUS: done
      BACKEND_CHANGES: ...
      BACKEND_TESTS: ...
    expects: "STATUS: done"

  - id: contract-review
    agent: contract
    input: |
      Review whether backend implementation matches the declared API contract.

      API_CONTRACT: {{api_contract}}
      BACKEND_CHANGES: {{backend_changes}}

      Reply with:
      STATUS: done
      CONTRACT_STATUS: frozen
      CONTRACT_NOTES: ...

      Or if contract mismatch:
      STATUS: retry
      ISSUES:
      - ...
    expects: "STATUS: done"
    on_fail:
      retry_step: backend-implement
      max_retries: 2
      on_exhausted:
        escalate_to: human

  - id: frontend-implement
    agent: frontend
    input: |
      Implement frontend changes only, based on the frozen API contract.

      REPO: {{repo}}
      BRANCH: {{branch}}
      API_CONTRACT: {{api_contract}}
      CONTRACT_STATUS: {{contract_status}}
      FRONTEND_STORIES_JSON: {{frontend_stories_json}}

      Rules:
      1. Only modify frontend files
      2. Do not change backend contract
      3. Add or update frontend tests where applicable

      Reply with:
      STATUS: done
      FRONTEND_CHANGES: ...
      FRONTEND_TESTS: ...
    expects: "STATUS: done"

  - id: integrate
    agent: integrator
    input: |
      Run integration checks across frontend and backend.

      REPO: {{repo}}
      BRANCH: {{branch}}
      API_CONTRACT: {{api_contract}}
      BACKEND_CHANGES: {{backend_changes}}
      FRONTEND_CHANGES: {{frontend_changes}}
      TEST_PLAN: {{test_plan}}

      Reply with:
      STATUS: done
      INTEGRATION_RESULT: ...

      Or if issues found:
      STATUS: retry
      ISSUES:
      - ...
    expects: "STATUS: done"
    on_fail:
      retry_step: frontend-implement
      max_retries: 2
      on_exhausted:
        escalate_to: human

  - id: test
    agent: tester
    input: |
      Run end-to-end and regression testing for the full feature.

      REPO: {{repo}}
      BRANCH: {{branch}}
      TEST_PLAN: {{test_plan}}
      INTEGRATION_RESULT: {{integration_result}}

      Reply with:
      STATUS: done
      RESULTS: ...
    expects: "STATUS: done"

  - id: review
    agent: reviewer
    input: |
      Review the final implementation for correctness, completeness, and risk.

      REPO: {{repo}}
      BRANCH: {{branch}}
      RESULTS: {{results}}

      Reply with:
      STATUS: done
      DECISION: approved
    expects: "STATUS: done"
```

### 这版 YAML 草案的设计意图

1. `backend` 和 `frontend` 分开承担实现责任
2. `contract-review` 充当“冻结契约”节点
3. `integrator` 吸收联调复杂度，不让前后端 agent 互相扯皮
4. `tester` 只做面向用户路径的最终确认

---

## 10. 当前实现约束与注意点

这部分非常重要。它决定了上面的草案能否直接落地，还是必须先调整 workflow 策略。

### 1. 当前 `Antfarm` 更像线性工作流，不是通用 DAG

按现有类型定义，step 本质上是线性数组推进，失败时通过 `retry_step` 回跳；没有原生“前后端两个分支并行再汇合”的 DAG 结构。

因此：

1. 不要假设可以原生并行跑 `backend-implement` 和 `frontend-implement`
2. 第一版建议仍然走串行流程
3. 真要并行，需要额外扩展引擎，而不是只改 YAML

### 2. 原生 loop 只支持 `stories`

当前 loop 配置只支持：

1. `over: stories`
2. `completion: all_done`

这意味着如果你想同时做“后端 stories”与“前端 stories”两套循环，当前实现未必能直接优雅承载。

更稳的做法有两个：

1. 第一版不用双 loop，先用单步串行实现前后端
2. 或者让 `planner` 产出统一 `STORIES_JSON`，但给每条 story 标记 `area: backend|frontend`，再在实现 prompt 中要求 agent 只处理对应 area

### 3. agent role 是现成的，agent capability 不是无限自由的

当前角色类型主要是：

1. `analysis`
2. `coding`
3. `verification`
4. `testing`
5. `pr`
6. `scanning`

如果要增加 `contract-reviewer` 或 `integrator`，建议先映射到现有 role：

1. `contract-reviewer` -> `verification`
2. `integrator` -> `coding`

不要一开始就扩展 role 体系，除非现有权限模型明显不够。

### 4. 真正的“目录隔离”需要 agent workspace 配置配合

即使 prompt 写了“只改 frontend”，如果 `backend` 和 `frontend` agent 最终都能直接操作整仓，隔离仍然有限。

所以第二阶段应考虑：

1. 是否给不同 agent 提供不同 repo 挂载方式
2. 是否用只读镜像目录供 verifier / reviewer 使用
3. 是否把契约文件放在共享、但受控的中间目录

### 5. `Spacebot` 集成时更适合摘要轮询，不适合直接转发所有底层日志

第一版 `Spacebot -> Antfarm` 集成，推荐只拉这些字段：

1. `run id`
2. `workflow id`
3. `current step`
4. `run status`
5. 最近 N 条关键事件

不要一开始就尝试完整内嵌所有 step output，否则前台信息噪声会过高。

### 6. 文档里的 workflow 草案仍缺少 agent bootstrap 文件

如果后面真的要做成可安装 workflow，还必须补齐：

1. 各 agent 的 `AGENTS.md`
2. 各 agent 的 `SOUL.md`
3. 各 agent 的 `IDENTITY.md`
4. 可能需要的 skills

也就是说，真正落地时最少是“一套 YAML + 一组 agent 身份文件”，不是只有 YAML。

---

## 11. `Spacebot -> Antfarm` 最小接口设计

在当前阶段，不需要先做完整嵌入式 dashboard，也不需要先做复杂的双向实时同步。

更合理的第一版是定义一套足够小、足够稳定的接口，让 `Spacebot` 能完成四件事：

1. 触发 workflow
2. 记录 run
3. 获取摘要状态
4. 接收最终结果

### 1. 触发接口

`Spacebot` 在需求澄清完成后，应生成一个结构化请求对象，而不是直接把整段自然语言原样丢给 shell。

推荐内部请求格式：

```json
{
  "requestId": "req_20260302_001",
  "source": "spacebot",
  "conversationId": "spacebot:web:abc123",
  "workflowId": "feature-dev",
  "taskTitle": "Implement user check-in feature with points reward",
  "taskBody": "完整需求说明文本",
  "notifyTarget": {
    "type": "spacebot",
    "conversationId": "spacebot:web:abc123"
  },
  "metadata": {
    "productArea": "growth",
    "priority": "medium",
    "requestedBy": "human"
  }
}
```

### 2. `trigger_antfarm` 的职责

`Spacebot` 侧建议实现一个单一职责工具，例如 `trigger_antfarm`。

它只负责：

1. 接收结构化请求
2. 生成可审计的任务文本
3. 调用 `antfarm workflow run <workflowId> "<task>"`
4. 保存 `runId`
5. 把 `runId` 绑定回 `conversationId`

它不应负责：

1. 解释底层 step 日志
2. 承担完整 dashboard 渲染
3. 决定业务验收是否通过

### 3. 启动返回格式

触发成功后，`Spacebot` 内部至少应拿到以下结构：

```json
{
  "ok": true,
  "runId": "a1fdf573-xxxx-xxxx-xxxx",
  "workflowId": "feature-dev",
  "status": "running",
  "acceptedAt": "2026-03-02T10:00:00Z"
}
```

对用户展示时不需要原样输出 JSON，可以整理成：

- 任务已启动
- 工作流：`feature-dev`
- 运行编号：`#12`
- 当前状态：`running`

### 4. 状态摘要接口

第一版不要直接传输全部 step output，先收敛成摘要模型。

推荐状态摘要格式：

```json
{
  "runId": "a1fdf573-xxxx-xxxx-xxxx",
  "workflowId": "feature-dev",
  "status": "running",
  "currentStep": "implement",
  "currentAgent": "developer",
  "storyProgress": {
    "done": 2,
    "total": 5
  },
  "lastUpdatedAt": "2026-03-02T10:12:00Z",
  "recentEvents": [
    {
      "type": "story.done",
      "label": "Story completed",
      "detail": "checkin-api"
    },
    {
      "type": "step.running",
      "label": "Current step running",
      "detail": "implement"
    }
  ],
  "blocking": null
}
```

如果任务被卡住，则 `blocking` 字段应显式给出：

```json
{
  "type": "human_input_required",
  "reason": "Baseline tests are already failing on main branch"
}
```

### 5. 关键事件映射规则

`Antfarm` 原始事件粒度偏底层，`Spacebot` 不应直接裸透传。

建议做如下映射：

1. `run.started`
   - 用户文案：任务已启动

2. `step.running`
   - 用户文案：进入新阶段

3. `story.done`
   - 用户文案：完成一个子任务

4. `step.failed`
   - 用户文案：当前阶段失败，系统准备重试或等待处理

5. `run.failed`
   - 用户文案：任务失败，需要人工介入

6. `run.completed`
   - 用户文案：任务完成，等待验收

### 6. 最终结果回传格式

`Spacebot` 需要的不是“全部日志”，而是一份可用于交付的结果对象。

推荐最终结果格式：

```json
{
  "runId": "a1fdf573-xxxx-xxxx-xxxx",
  "workflowId": "feature-dev",
  "status": "completed",
  "summary": {
    "task": "Implement user check-in feature with points reward",
    "changes": "新增签到接口、积分规则、前端签到入口",
    "tests": "15 tests passed",
    "reviewDecision": "approved"
  },
  "artifacts": {
    "branch": "feature/checkin",
    "prUrl": "https://github.com/org/repo/pull/123",
    "commitRange": "abc123..def456"
  },
  "handoff": {
    "needsHumanAcceptance": true,
    "openQuestions": []
  }
}
```

### 7. 失败结果回传格式

失败时不要只返回一句 `run failed`，而应包含最低限度的定位信息：

```json
{
  "runId": "a1fdf573-xxxx-xxxx-xxxx",
  "workflowId": "feature-dev",
  "status": "failed",
  "failedStep": "setup",
  "failedAgent": "setup",
  "reason": "Baseline test suite failed before feature work started",
  "retryExhausted": true,
  "needsHumanIntervention": true,
  "suggestedNextAction": "Confirm whether failing baseline can be accepted or should be fixed first"
}
```

### 8. 建议的数据绑定关系

为了让 `Spacebot` 能在多会话场景下正确回显，至少要有以下绑定：

1. `conversationId -> runId`
2. `requestId -> runId`
3. `runId -> workflowId`
4. `runId -> latestStatus`
5. `runId -> finalResult`

如果这些关系不先定义好，后面很容易出现：

1. 任务已经完成，但消息回不到原对话
2. 多个 run 混到同一个会话里
3. 用户追问时查不到当前状态

### 9. 建议的第一版通信方式

第一版推荐优先级如下：

1. `Spacebot` 调用 shell 触发 `antfarm workflow run`
2. `Spacebot` 通过轮询读取 run 状态与最近事件
3. 可选再补 webhook 用于完成态通知

不建议第一版就做：

1. 全实时 SSE 桥接
2. 全量日志流同步
3. dashboard 级别嵌入

原因是这些功能对体验有帮助，但不是协议稳定性的核心。

### 10. 开发阶段的 mock 策略

既然当前机器不承担最终部署职责，接口开发时建议先提供一个 mock adapter：

1. mock `trigger_antfarm` 返回固定 `runId`
2. mock 状态接口返回一组可预期状态迁移
3. mock 最终结果对象

这样可以先把 `Spacebot` 侧的：

1. 触发逻辑
2. run 状态展示
3. 结果回传 UI
4. 错误处理流程

全部开发出来，后面再替换成真实 `Antfarm` 适配层。

---

## 12. 最终建议

这套方案应当坚持一个原则：

**用户只面对 `Spacebot`，`Antfarm` 负责推进流程，`OpenClaw` 负责真正干活。**

因此：

- `Antfarm Dashboard` 可以有，但不是必须
- 主产品入口应统一收敛在 `Spacebot`
- 方案文档后续如继续扩展，应默认围绕“单入口、多后台”的产品形态展开
