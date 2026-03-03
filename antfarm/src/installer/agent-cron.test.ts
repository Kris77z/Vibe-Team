import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { buildWorkPrompt } from "./agent-cron.js";

describe("buildWorkPrompt", () => {
  it("does not hardcode a generic completion payload", () => {
    const prompt = buildWorkPrompt("feature-dev", "planner");
    assert.ok(prompt.includes("Reply with:"));
    assert.ok(prompt.includes("NEVER default to STATUS/CHANGES/TESTS"));
    assert.ok(prompt.includes('explicitly set exec host to "gateway" and exec security to "full"'));
    assert.ok(!prompt.includes("CHANGES: what you did"));
    assert.ok(!prompt.includes("TESTS: what tests you ran"));
    assert.ok(prompt.includes("<paste the exact KEY: VALUE output required by the claimed step here>"));
  });
});

describe("buildPollingPrompt", () => {
  it("includes a direct-execution fallback when sessions_spawn is unavailable", async () => {
    const { buildPollingPrompt } = await import("./agent-cron.js");
    const prompt = buildPollingPrompt("feature-dev", "developer", "openai-relay/gpt-5.1");
    assert.ok(prompt.includes("If sessions_spawn is unavailable, rejected, or fails for any reason, DO NOT stop."));
    assert.ok(prompt.includes("continue in the current cron session and execute the claimed step yourself"));
  });
});
