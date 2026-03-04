import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { buildCronAgentId, buildCronAgentToolsConfig, buildWorkflowAgentToolsConfig } from "./install.js";

describe("install role tool policies", () => {
  it("keeps workflow verifier agents on role-based tools without forcing gateway exec", () => {
    const tools = buildWorkflowAgentToolsConfig("verification");
    assert.equal("exec" in tools, false);
    assert.deepEqual((tools.deny as string[]).slice(0, 3), ["gateway", "cron", "message"]);
    assert.ok((tools.deny as string[]).includes("write"));
    assert.ok((tools.deny as string[]).includes("apply_patch"));
  });

  it("creates dedicated cron agents with gateway exec and no write tools", () => {
    const tools = buildCronAgentToolsConfig();
    assert.deepEqual(tools.exec, {
      host: "gateway",
      security: "full",
      ask: "off",
    });
    assert.ok((tools.deny as string[]).includes("write"));
    assert.ok((tools.deny as string[]).includes("edit"));
    assert.ok((tools.deny as string[]).includes("apply_patch"));
    assert.ok((tools.deny as string[]).includes("browser"));
  });

  it("namespaces cron agents separately from worker agents", () => {
    assert.equal(buildCronAgentId("feature-dev", "verifier"), "feature-dev_verifier__cron");
  });

  it("keeps managed agents out of sandbox mode", async () => {
    const mod = await import("./install.js");
    const workflowAgent = (mod as any).buildWorkflowAgentConfig({
      id: "feature-dev_planner",
      workspaceDir: "/tmp/workflow/planner",
      agentDir: "/tmp/agents/feature-dev_planner/agent",
      role: "analysis",
    });
    const cronAgent = (mod as any).buildCronAgentConfig({
      workflowId: "feature-dev",
      localId: "planner",
      workspaceDir: "/tmp/workflow/__cron/planner",
      agentDir: "/tmp/agents/feature-dev_planner__cron/agent",
    });

    assert.deepEqual(workflowAgent.sandbox, { mode: "off" });
    assert.deepEqual(cronAgent.sandbox, { mode: "off" });
  });
});
