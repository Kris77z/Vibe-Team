import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { buildWorkPrompt } from "./agent-cron.js";

describe("buildWorkPrompt", () => {
  it("does not hardcode a generic completion payload", () => {
    const prompt = buildWorkPrompt("feature-dev", "planner");
    assert.ok(prompt.includes("Reply with:"));
    assert.ok(prompt.includes("NEVER default to STATUS/CHANGES/TESTS"));
    assert.ok(prompt.includes("use the current agent's default exec policy"));
    assert.ok(prompt.includes('Do NOT force exec host to "gateway"'));
    assert.ok(!prompt.includes("CHANGES: what you did"));
    assert.ok(!prompt.includes("TESTS: what tests you ran"));
    assert.ok(prompt.includes("<paste the exact KEY: VALUE output required by the claimed step here>"));
  });
});

describe("buildPollingPrompt", () => {
  it("fails the claimed step instead of executing it in the cron session when sessions_spawn fails", async () => {
    const { buildPollingPrompt } = await import("./agent-cron.js");
    const prompt = buildPollingPrompt("feature-dev", "developer", "openai-relay/gpt-5.1");
    assert.ok(prompt.includes('step fail "<stepId>" "Failed to spawn worker session for feature-dev_developer"'));
    assert.ok(!prompt.includes("execute the claimed step yourself"));
  });
});
