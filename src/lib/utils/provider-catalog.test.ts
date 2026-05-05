import { describe, expect, it } from "vitest";
import { getPhase7Provider, PHASE7_PROVIDERS, providerIdForRun } from "./provider-catalog";

describe("Phase 7 provider catalog", () => {
  it("exposes the five visible providers in product order", () => {
    expect(PHASE7_PROVIDERS.map((provider) => provider.label)).toEqual([
      "Claude",
      "Codex",
      "Gemini",
      "DeepSeek",
      "GLM",
    ]);
  });

  it("keeps API providers on Claude execution with provider identity metadata", () => {
    expect(getPhase7Provider("deepseek")).toMatchObject({
      mode: "claude_compatible_api",
      executionAgent: "claude",
      platformId: "deepseek",
      requiredConfig: ["api_key"],
      defaultPermissionMode: "bypass",
    });
    expect(getPhase7Provider("glm")).toMatchObject({
      mode: "claude_compatible_api",
      executionAgent: "claude",
      platformId: "zhipu",
      requiredConfig: ["api_key", "base_url", "model"],
      defaultPermissionMode: "bypass",
    });
  });

  it("maps run execution identity back to visible provider identity", () => {
    expect(providerIdForRun("claude", "deepseek")).toBe("deepseek");
    expect(providerIdForRun("claude", "zhipu")).toBe("glm");
    expect(providerIdForRun("codex")).toBe("codex");
    expect(providerIdForRun("gemini")).toBe("gemini");
    expect(providerIdForRun("claude")).toBe("claude");
  });
});
