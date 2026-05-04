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
  it("only returns Claude runs with a session id for Claude startup", () => {
    const runs = [
      run("codex-latest", "codex", {
        conversation_ref: { kind: "codex_thread", id: "thread-1" },
      }),
      run("claude-latest-without-session", "claude"),
      run("claude-continuable", "claude", { session_id: "session-1" }),
    ];

    expect(findLastContinuableRun(runs, "claude")?.id).toBe("claude-continuable");
  });

  it("does not show another agent's continue entry on native CLI startup pages", () => {
    const runs = [
      run("claude-continuable", "claude", { session_id: "session-1" }),
      run("codex-completed", "codex", {
        conversation_ref: { kind: "codex_thread", id: "thread-1" },
      }),
    ];

    expect(findLastContinuableRun(runs, "codex")).toBeNull();
    expect(findLastContinuableRun(runs, "gemini")).toBeNull();
  });

  it("ignores active runs", () => {
    const runs = [
      run("active", "claude", { session_id: "session-2", status: "running" }),
      run("stopped", "claude", { session_id: "session-1", status: "stopped" }),
    ];

    expect(findLastContinuableRun(runs, "claude")?.id).toBe("stopped");
  });
});
