import { describe, it, expect } from "vitest";
import { getAgentFeatures, isKnownAgent } from "../agent-features";

describe("getAgentFeatures", () => {
  it("returns full features for claude", () => {
    const f = getAgentFeatures("claude");
    expect(f.effortSelector).toBe(true);
    expect(f.planModeToggle).toBe(true);
    expect(f.permissionModeSwitch).toBe(true);
    expect(f.slashCommandMenu).toBe(true);
    expect(f.addDirAction).toBe(true);
  });

  it("returns native shortcut features for codex", () => {
    const f = getAgentFeatures("codex");
    expect(f.effortSelector).toBe(false);
    expect(f.planModeToggle).toBe(false);
    expect(f.permissionModeSwitch).toBe(true);
    expect(f.slashCommandMenu).toBe(true);
    expect(f.addDirAction).toBe(true);
  });

  it("returns native shortcut features for gemini", () => {
    const f = getAgentFeatures("gemini");
    expect(f.effortSelector).toBe(false);
    expect(f.planModeToggle).toBe(false);
    expect(f.permissionModeSwitch).toBe(true);
    expect(f.slashCommandMenu).toBe(true);
    expect(f.addDirAction).toBe(true);
  });

  it("returns minimal features for unknown agent", () => {
    const f = getAgentFeatures("unknown-agent");
    expect(f.effortSelector).toBe(false);
    expect(f.addDirAction).toBe(false);
  });
});

describe("isKnownAgent", () => {
  it("recognizes claude, codex, and gemini", () => {
    expect(isKnownAgent("claude")).toBe(true);
    expect(isKnownAgent("codex")).toBe(true);
    expect(isKnownAgent("gemini")).toBe(true);
  });

  it("returns false for unknown agents", () => {
    expect(isKnownAgent("")).toBe(false);
  });
});
