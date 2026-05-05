import { describe, expect, it } from "vitest";
import { findLastContinuableRun } from "./continuable-run";
import type { TaskRun } from "$lib/types";

function run(id: string, agent: string, extras: Partial<TaskRun> = {}): TaskRun {
  return {
    id,
    prompt: id,
    cwd: "D:/work",
    agent,
    auth_mode: "cli",
    status: "completed",
    started_at: "2026-05-04T00:00:00.000Z",
    execution_path: agent === "claude" ? "session_actor" : "pipe_exec",
    ...extras,
  };
}

describe("findLastContinuableRun", () => {
  it("returns Claude runs with a session id for Claude startup", () => {
    const runs = [
      run("codex-latest", "codex", {
        conversation_ref: { kind: "codex_thread", id: "thread-1" },
      }),
      run("claude-latest-without-session", "claude"),
      run("claude-continuable", "claude", { session_id: "session-1" }),
    ];

    expect(findLastContinuableRun(runs, "claude")?.id).toBe("claude-continuable");
  });

  it("returns native CLI runs for their own startup pages", () => {
    const runs = [
      run("claude-continuable", "claude", { session_id: "session-1" }),
      run("codex-completed", "codex", {
        conversation_ref: { kind: "codex_thread", id: "thread-1" },
      }),
      run("gemini-completed", "gemini"),
    ];

    expect(findLastContinuableRun(runs, "codex")?.id).toBe("codex-completed");
    expect(findLastContinuableRun(runs, "gemini")?.id).toBe("gemini-completed");
  });

  it("returns Claude-compatible API runs by provider identity", () => {
    const runs = [
      run("claude-continuable", "claude", { session_id: "session-1" }),
      run("deepseek-continuable", "claude", {
        session_id: "session-2",
        platform_id: "deepseek",
      }),
      run("glm-continuable", "claude", {
        session_id: "session-3",
        platform_id: "zhipu",
      }),
      run("qwen-continuable", "claude", {
        session_id: "session-4",
        platform_id: "bailian",
      }),
      run("kimi-continuable", "claude", {
        session_id: "session-5",
        platform_id: "kimi",
      }),
    ];

    expect(findLastContinuableRun(runs, "deepseek")?.id).toBe("deepseek-continuable");
    expect(findLastContinuableRun(runs, "glm")?.id).toBe("glm-continuable");
    expect(findLastContinuableRun(runs, "qwen")?.id).toBe("qwen-continuable");
    expect(findLastContinuableRun(runs, "kimi")?.id).toBe("kimi-continuable");
  });

  it("ignores active runs", () => {
    const runs = [
      run("active", "claude", { session_id: "session-2", status: "running" }),
      run("stopped", "claude", { session_id: "session-1", status: "stopped" }),
    ];

    expect(findLastContinuableRun(runs, "claude")?.id).toBe("stopped");
  });
});
