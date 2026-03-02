/**
 * Development draft only.
 *
 * This file is intentionally NOT wired into Spacebot, Antfarm, or OpenClaw yet.
 * It exists to lock the integration boundary before deployment work starts on the
 * target Mac.
 *
 * Reuse map:
 * - Antfarm read APIs already exist in:
 *   - antfarm/src/server/dashboard.ts
 *     - GET /api/runs
 *     - GET /api/runs/:id
 *     - GET /api/runs/:id/events
 *     - GET /api/runs/:id/stories
 * - Antfarm event storage already exists in:
 *   - antfarm/src/installer/events.ts
 * - Spacebot already has SSE/event streaming examples in:
 *   - spacebot/src/agent/cortex_chat.rs
 *   - spacebot/src/api/server.rs
 *
 * Design choice:
 * - Do NOT invent a new raw step-log protocol.
 * - Reuse Antfarm's existing dashboard JSON endpoints for run reads.
 * - Keep workflow launch as a separate concern; first version should still use
 *   CLI/shell for `antfarm workflow run ...` because Antfarm does not currently
 *   expose a dedicated HTTP run-start endpoint.
 */

export type WorkflowId = "feature-dev" | "feature-dev-split" | string;

export type TriggerWorkflowRequest = {
  requestId: string;
  source: "spacebot";
  conversationId: string;
  workflowId: WorkflowId;
  taskTitle: string;
  taskBody: string;
  repoPath?: string;
  branch?: string;
  worktreePath?: string;
  metadata?: Record<string, string>;
};

export type TriggerWorkflowResult = {
  ok: boolean;
  runId: string;
  workflowId: string;
  status: "running" | "failed";
  acceptedAt: string;
  runNumber?: number | null;
};

export type RunBlockingState = {
  type: "human_input_required" | "retrying" | "infra_error";
  reason: string;
} | null;

export type RunEventSummary = {
  type: string;
  label: string;
  detail?: string;
};

export type RunSummary = {
  runId: string;
  workflowId: string;
  status: string;
  currentStep: string | null;
  currentAgent: string | null;
  storyProgress: {
    done: number;
    total: number;
  };
  lastUpdatedAt: string;
  recentEvents: RunEventSummary[];
  blocking: RunBlockingState;
};

export type FinalRunResult = {
  runId: string;
  workflowId: string;
  status: "completed" | "failed" | "cancelled";
  summary: {
    task: string;
    changes: string;
    tests: string;
    reviewDecision: string;
  };
  artifacts: {
    branch?: string;
    prUrl?: string;
    commitRange?: string;
  };
  handoff: {
    needsHumanAcceptance: boolean;
    openQuestions: string[];
  };
};

export interface WorkflowLauncher {
  triggerWorkflow(request: TriggerWorkflowRequest): Promise<TriggerWorkflowResult>;
}

export interface RunReader {
  getRunSummary(runId: string): Promise<RunSummary>;
  getFinalRunResult(runId: string): Promise<FinalRunResult | null>;
}

export interface SpacebotAntfarmAdapter extends WorkflowLauncher, RunReader {}

type AntfarmRunStep = {
  id: string;
  step_id: string;
  agent_id: string;
  status: string;
  output?: string | null;
  updated_at: string;
};

type AntfarmRunDetail = {
  id: string;
  run_number?: number | null;
  workflow_id: string;
  task: string;
  status: string;
  updated_at: string;
  steps: AntfarmRunStep[];
};

type AntfarmStory = {
  status: "pending" | "running" | "done" | "failed";
};

type AntfarmEvent = {
  event: string;
  detail?: string;
  stepId?: string;
  agentId?: string;
  storyId?: string;
  storyTitle?: string;
  ts: string;
};

function mapEventLabel(event: AntfarmEvent): string {
  switch (event.event) {
    case "run.started":
      return "Task started";
    case "run.completed":
      return "Task completed";
    case "run.failed":
      return "Task failed";
    case "step.running":
      return "Stage started";
    case "step.done":
      return "Stage completed";
    case "step.failed":
      return "Stage failed";
    case "story.done":
      return "Story completed";
    case "story.retry":
      return "Story retry";
    default:
      return event.event;
  }
}

function summarizeEvents(events: AntfarmEvent[], limit = 8): RunEventSummary[] {
  return events.slice(-limit).map((event) => ({
    type: event.event,
    label: mapEventLabel(event),
    detail: event.storyTitle ?? event.storyId ?? event.stepId ?? event.detail,
  }));
}

function deriveCurrentStep(steps: AntfarmRunStep[]): AntfarmRunStep | null {
  return (
    steps.find((step) => step.status === "running") ??
    steps.find((step) => step.status === "pending") ??
    steps.findLast((step) => step.status === "done") ??
    null
  );
}

function deriveBlockingState(run: AntfarmRunDetail, events: AntfarmEvent[]): RunBlockingState {
  const latest = events.at(-1);
  if (!latest) return null;

  if (run.status === "failed") {
    return {
      type: "human_input_required",
      reason: latest.detail ?? "Workflow failed and requires review",
    };
  }

  if (latest.event === "step.failed" || latest.event === "story.retry") {
    return {
      type: "retrying",
      reason: latest.detail ?? "A workflow step failed and is being retried",
    };
  }

  return null;
}

function extractStepOutputValue(output: string | null | undefined, key: string): string {
  if (!output) return "";
  const match = output.match(new RegExp(`^${key}:\\s*(.*)$`, "im"));
  return match?.[1]?.trim() ?? "";
}

export function mapAntfarmRunToSummary(params: {
  run: AntfarmRunDetail;
  stories: AntfarmStory[];
  events: AntfarmEvent[];
}): RunSummary {
  const currentStep = deriveCurrentStep(params.run.steps);
  const doneStories = params.stories.filter((story) => story.status === "done").length;

  return {
    runId: params.run.id,
    workflowId: params.run.workflow_id,
    status: params.run.status,
    currentStep: currentStep?.step_id ?? null,
    currentAgent: currentStep?.agent_id ?? null,
    storyProgress: {
      done: doneStories,
      total: params.stories.length,
    },
    lastUpdatedAt: params.run.updated_at,
    recentEvents: summarizeEvents(params.events),
    blocking: deriveBlockingState(params.run, params.events),
  };
}

export function mapAntfarmRunToFinalResult(params: {
  run: AntfarmRunDetail;
  events: AntfarmEvent[];
}): FinalRunResult | null {
  if (!["completed", "failed", "cancelled"].includes(params.run.status)) {
    return null;
  }

  const lastCompletedStep = params.run.steps.findLast((step) => step.status === "done");
  const output = lastCompletedStep?.output ?? "";

  return {
    runId: params.run.id,
    workflowId: params.run.workflow_id,
    status: params.run.status as "completed" | "failed" | "cancelled",
    summary: {
      task: params.run.task,
      // These fields are best-effort extraction from step output.
      // Real production integration should tighten this by defining explicit
      // final-step output keys instead of relying on generic parsing.
      changes: extractStepOutputValue(output, "CHANGES") || extractStepOutputValue(output, "RESULTS"),
      tests: extractStepOutputValue(output, "TESTS") || extractStepOutputValue(output, "RESULTS"),
      reviewDecision:
        extractStepOutputValue(output, "DECISION") ||
        (params.run.status === "completed" ? "approved" : "not_approved"),
    },
    artifacts: {
      branch: extractStepOutputValue(output, "BRANCH") || undefined,
      prUrl: extractStepOutputValue(output, "PR") || undefined,
      commitRange: undefined,
    },
    handoff: {
      needsHumanAcceptance: params.run.status === "completed",
      openQuestions: [],
    },
  };
}

/**
 * HTTP read adapter that reuses Antfarm Dashboard endpoints.
 *
 * This is a real integration direction, not a mock. It is still kept as a draft
 * because workflow triggering is intentionally left out; first version should
 * continue to launch workflows via CLI/shell, then use this reader for polling.
 */
export class AntfarmDashboardReader implements RunReader {
  constructor(private readonly baseUrl: string) {}

  async getRunSummary(runId: string): Promise<RunSummary> {
    const [run, stories, events] = await Promise.all([
      this.getJSON<AntfarmRunDetail>(`/api/runs/${encodeURIComponent(runId)}`),
      this.getJSON<AntfarmStory[]>(`/api/runs/${encodeURIComponent(runId)}/stories`),
      this.getJSON<AntfarmEvent[]>(`/api/runs/${encodeURIComponent(runId)}/events`),
    ]);

    return mapAntfarmRunToSummary({ run, stories, events });
  }

  async getFinalRunResult(runId: string): Promise<FinalRunResult | null> {
    const [run, events] = await Promise.all([
      this.getJSON<AntfarmRunDetail>(`/api/runs/${encodeURIComponent(runId)}`),
      this.getJSON<AntfarmEvent[]>(`/api/runs/${encodeURIComponent(runId)}/events`),
    ]);

    return mapAntfarmRunToFinalResult({ run, events });
  }

  private async getJSON<T>(path: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`);
    if (!response.ok) {
      throw new Error(`Antfarm dashboard request failed: ${response.status} ${path}`);
    }
    return (await response.json()) as T;
  }
}

/**
 * MOCK ONLY.
 *
 * This adapter exists so Spacebot-side UI and state handling can be developed
 * before the deployment Mac is ready.
 *
 * Important:
 * - Do not mistake this for a production implementation.
 * - Do not wire this as the default runtime adapter without an explicit feature
 *   flag or test-only guard.
 * - Every status transition below is synthetic.
 */
export class MockSpacebotAntfarmAdapter implements SpacebotAntfarmAdapter {
  private readonly runs = new Map<
    string,
    {
      request: TriggerWorkflowRequest;
      trigger: TriggerWorkflowResult;
      summaryPollCount: number;
    }
  >();

  async triggerWorkflow(request: TriggerWorkflowRequest): Promise<TriggerWorkflowResult> {
    const runId = `mock-run-${request.requestId}`;
    const trigger: TriggerWorkflowResult = {
      ok: true,
      runId,
      workflowId: request.workflowId,
      status: "running",
      acceptedAt: new Date().toISOString(),
      runNumber: this.runs.size + 1,
    };

    this.runs.set(runId, {
      request,
      trigger,
      summaryPollCount: 0,
    });

    return trigger;
  }

  async getRunSummary(runId: string): Promise<RunSummary> {
    const record = this.runs.get(runId);
    if (!record) {
      throw new Error(`Unknown mock run: ${runId}`);
    }

    record.summaryPollCount += 1;
    const poll = record.summaryPollCount;

    if (poll <= 1) {
      return this.makeSummary(record, "running", "plan", "planner", 0, 3, "Task started");
    }

    if (poll === 2) {
      return this.makeSummary(record, "running", "setup", "setup", 0, 3, "Stage started");
    }

    if (poll === 3) {
      return this.makeSummary(record, "running", "implement", "developer", 1, 3, "Story completed");
    }

    return this.makeSummary(record, "completed", "review", "reviewer", 3, 3, "Task completed");
  }

  async getFinalRunResult(runId: string): Promise<FinalRunResult | null> {
    const record = this.runs.get(runId);
    if (!record) {
      throw new Error(`Unknown mock run: ${runId}`);
    }

    if (record.summaryPollCount < 4) {
      return null;
    }

    return {
      runId,
      workflowId: record.trigger.workflowId,
      status: "completed",
      summary: {
        task: record.request.taskTitle,
        changes: "Mock result: backend API, frontend entry, and validation flow completed.",
        tests: "Mock result: 15 tests passed.",
        reviewDecision: "approved",
      },
      artifacts: {
        branch: "feature/mock-checkin",
        prUrl: "https://example.invalid/pr/123",
        commitRange: "abc123..def456",
      },
      handoff: {
        needsHumanAcceptance: true,
        openQuestions: [],
      },
    };
  }

  private makeSummary(
    record: {
      trigger: TriggerWorkflowResult;
      summaryPollCount: number;
    },
    status: string,
    currentStep: string,
    currentAgent: string,
    done: number,
    total: number,
    label: string,
  ): RunSummary {
    return {
      runId: record.trigger.runId,
      workflowId: record.trigger.workflowId,
      status,
      currentStep,
      currentAgent,
      storyProgress: { done, total },
      lastUpdatedAt: new Date().toISOString(),
      recentEvents: [{ type: "mock.event", label, detail: currentStep }],
      blocking: null,
    };
  }
}
