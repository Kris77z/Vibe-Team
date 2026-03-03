import { afterEach, describe, it } from "node:test";
import assert from "node:assert/strict";
import crypto from "node:crypto";
import { getDb } from "../db.js";
import { claimStep, completeStep, peekStep } from "./step-ops.js";
import { assertOutputMatchesReplyContract, extractReplyContracts } from "./step-ops.js";

describe("extractReplyContracts", () => {
  it("extracts planner reply keys including STORIES_JSON", () => {
    const template = `Plan the work.

Reply with:
STATUS: done
REPO: /path/to/repo
BRANCH: feature-branch-name
STORIES_JSON: [ ... array of story objects ... ]`;

    const contracts = extractReplyContracts(template);
    assert.deepEqual(contracts.keysByStatus.done, ["STATUS", "REPO", "BRANCH", "STORIES_JSON"]);
  });

  it("tracks alternative retry contracts by status", () => {
    const template = `Verify the work.

Reply with:
STATUS: done
VERIFIED: What you confirmed

Or if incomplete:
STATUS: retry
ISSUES:
- Missing coverage`;

    const contracts = extractReplyContracts(template);
    assert.deepEqual(contracts.keysByStatus.done, ["STATUS", "VERIFIED"]);
    assert.deepEqual(contracts.keysByStatus.retry, ["STATUS", "ISSUES"]);
  });
});

describe("assertOutputMatchesReplyContract", () => {
  it("rejects planner output that omits STORIES_JSON", () => {
    const template = `Reply with:
STATUS: done
REPO: /repo
BRANCH: branch
STORIES_JSON: [ ... ]`;

    assert.throws(
      () => assertOutputMatchesReplyContract(template, "STATUS: done\nREPO: /repo\nBRANCH: branch"),
      /missing required key\(s\): STORIES_JSON/
    );
  });

  it("accepts retry output without done-only keys", () => {
    const template = `Reply with:
STATUS: done
RESULTS: What you tested

Or if issues found:
STATUS: retry
FAILURES:
- Broken test`;

    assert.doesNotThrow(() => assertOutputMatchesReplyContract(template, "STATUS: retry\nFAILURES:\n- Broken test"));
  });

  it("rejects output that omits STATUS when the reply contract requires it", () => {
    const template = `Reply with:
STATUS: done
REPO: /repo
BRANCH: branch
STORIES_JSON: [ ... ]`;

    assert.throws(
      () => assertOutputMatchesReplyContract(template, "REPO: /repo\nBRANCH: branch\nSTORIES_JSON: []"),
      /missing required key\(s\): STATUS/
    );
  });
});

function createRunWithVerifyEach(runId: string) {
  const db = getDb();
  const now = new Date().toISOString();
  db.prepare(
    "INSERT INTO runs (id, workflow_id, task, status, context, created_at, updated_at) VALUES (?, ?, ?, 'running', '{}', ?, ?)"
  ).run(runId, "feature-dev", "test task", now, now);
  db.prepare(
    "INSERT INTO steps (id, run_id, step_id, agent_id, step_index, input_template, expects, status, type, loop_config, created_at, updated_at) VALUES (?, ?, 'implement', 'feature-dev_developer', 2, '', 'STATUS: done', 'running', 'loop', ?, ?, ?)"
  ).run(
    crypto.randomUUID(),
    runId,
    JSON.stringify({ over: "stories", completion: "all_done", verifyEach: true, verifyStep: "verify" }),
    now,
    now
  );
  db.prepare(
    "INSERT INTO steps (id, run_id, step_id, agent_id, step_index, input_template, expects, status, created_at, updated_at) VALUES (?, ?, 'verify', 'feature-dev_verifier', 3, '', 'STATUS: done', 'pending', ?, ?)"
  ).run(
    crypto.randomUUID(),
    runId,
    now,
    now
  );
}

function cleanupRun(runId: string) {
  const db = getDb();
  db.prepare("DELETE FROM stories WHERE run_id = ?").run(runId);
  db.prepare("DELETE FROM steps WHERE run_id = ?").run(runId);
  db.prepare("DELETE FROM runs WHERE id = ?").run(runId);
}

describe("verify_each claiming", () => {
  const runIds: string[] = [];

  afterEach(() => {
    for (const runId of runIds) cleanupRun(runId);
    runIds.length = 0;
  });

  it("allows peek and claim for the verify step while the loop step is running", () => {
    const runId = crypto.randomUUID();
    runIds.push(runId);
    createRunWithVerifyEach(runId);

    assert.equal(peekStep("feature-dev_verifier"), "HAS_WORK");
    const claimed = claimStep("feature-dev_verifier");
    assert.equal(claimed.found, true);
    assert.equal(claimed.runId, runId);
  });
});

describe("completeStep idempotency", () => {
  const runIds: string[] = [];

  afterEach(() => {
    for (const runId of runIds) cleanupRun(runId);
    runIds.length = 0;
  });

  it("ignores duplicate completion calls for steps that are no longer running", () => {
    const runId = crypto.randomUUID();
    runIds.push(runId);
    const db = getDb();
    const now = new Date().toISOString();
    db.prepare(
      "INSERT INTO runs (id, workflow_id, task, status, context, created_at, updated_at) VALUES (?, ?, ?, 'running', '{}', ?, ?)"
    ).run(runId, "feature-dev", "test task", now, now);
    db.prepare(
      "INSERT INTO steps (id, run_id, step_id, agent_id, step_index, input_template, expects, status, created_at, updated_at) VALUES (?, ?, 'verify', 'feature-dev_verifier', 0, 'Reply with:\\nSTATUS: done\\nVERIFIED: ok', 'STATUS: done', 'waiting', ?, ?)"
    ).run("step-1", runId, now, now);
    db.prepare(
      "INSERT INTO stories (id, run_id, story_index, story_id, title, description, acceptance_criteria, status, retry_count, max_retries, created_at, updated_at) VALUES (?, ?, 0, 'US-001', 'Story', 'Desc', '[]', 'done', 0, 2, ?, ?)"
    ).run("story-1", runId, now, now);

    const result = completeStep("step-1", "STATUS: retry\nISSUES:\n- should be ignored");
    assert.deepEqual(result, { advanced: false, runCompleted: false });

    const story = db.prepare("SELECT status, retry_count FROM stories WHERE id = ?").get("story-1") as { status: string; retry_count: number };
    assert.equal(story.status, "done");
    assert.equal(story.retry_count, 0);
  });

  it("ignores stale loop completions when the current story has already been cleared", () => {
    const runId = crypto.randomUUID();
    runIds.push(runId);
    const db = getDb();
    const now = new Date().toISOString();
    db.prepare(
      "INSERT INTO runs (id, workflow_id, task, status, context, created_at, updated_at) VALUES (?, ?, ?, 'running', '{}', ?, ?)"
    ).run(runId, "feature-dev", "test task", now, now);
    db.prepare(
      "INSERT INTO steps (id, run_id, step_id, agent_id, step_index, input_template, expects, status, type, current_story_id, created_at, updated_at) VALUES (?, ?, 'implement', 'feature-dev_developer', 0, 'Reply with:\\nSTATUS: done\\nCHANGES: x\\nTESTS: y', 'STATUS: done', 'running', 'loop', NULL, ?, ?)"
    ).run("loop-step", runId, now, now);
    db.prepare(
      "INSERT INTO steps (id, run_id, step_id, agent_id, step_index, input_template, expects, status, created_at, updated_at) VALUES (?, ?, 'verify', 'feature-dev_verifier', 1, 'Reply with:\\nSTATUS: done\\nVERIFIED: ok', 'STATUS: done', 'waiting', ?, ?)"
    ).run("verify-step", runId, now, now);
    db.prepare(
      "INSERT INTO stories (id, run_id, story_index, story_id, title, description, acceptance_criteria, status, retry_count, max_retries, created_at, updated_at) VALUES (?, ?, 0, 'US-001', 'Story', 'Desc', '[]', 'done', 0, 2, ?, ?)"
    ).run("story-1", runId, now, now);

    const result = completeStep("loop-step", "STATUS: done\nCHANGES: stale\nTESTS: stale");
    assert.deepEqual(result, { advanced: false, runCompleted: false });

    const step = db.prepare("SELECT status FROM steps WHERE id = ?").get("loop-step") as { status: string };
    assert.equal(step.status, "running");
  });
});
