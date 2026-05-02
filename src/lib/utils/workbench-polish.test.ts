import { describe, expect, it } from "vitest";
import type { TaskRun } from "$lib/types";
import type { ConversationGroup } from "./sidebar-groups";
import {
  buildResumeLastActiveCommand,
  buildLastActiveResumePath,
  conversationUnreadCount,
  markConversationSeen,
  previewForConversation,
} from "./workbench-polish";

function makeRun(overrides: Partial<TaskRun> = {}): TaskRun {
  return {
    id: "r1",
    prompt: "Initial prompt text",
    cwd: "/project",
    agent: "claude",
    auth_mode: "cli",
    status: "completed",
    started_at: "2024-01-01T00:00:00Z",
    execution_path: "session_actor",
    message_count: 1,
    ...overrides,
  };
}

function makeConversation(runs: TaskRun[]): ConversationGroup {
  return {
    groupKey: "s:session",
    runs,
    title: "Session",
    latestRun: runs[0],
    isFavorite: false,
    totalMessages: runs.reduce((sum, run) => sum + (run.message_count ?? 0), 0),
  };
}

describe("workbench polish helpers", () => {
  it("uses persisted last message preview before falling back to prompt", () => {
    const conversation = makeConversation([
      makeRun({ prompt: "Start topic", last_message_preview: "Latest assistant answer" }),
    ]);

    expect(previewForConversation(conversation)).toBe("Latest assistant answer");
  });

  it("computes unread messages from persisted seen counts", () => {
    const conversation = makeConversation([
      makeRun({ id: "r1", message_count: 4 }),
      makeRun({ id: "r2", message_count: 2 }),
    ]);

    expect(conversationUnreadCount(conversation, { r1: 2, r2: 2 })).toBe(2);
  });

  it("marks every run in an opened conversation as seen", () => {
    const conversation = makeConversation([
      makeRun({ id: "r1", message_count: 4 }),
      makeRun({ id: "r2", message_count: 2 }),
    ]);

    expect(markConversationSeen({ r1: 1 }, conversation)).toEqual({ r1: 4, r2: 2 });
  });

  it("builds a resume URL for the latest active run", () => {
    expect(buildLastActiveResumePath(makeRun({ id: "run-active" }))).toBe(
      "/chat?run=run-active&resume=resume",
    );
  });

  it("builds a command palette entry for resuming the latest active run", () => {
    expect(buildResumeLastActiveCommand(makeRun({ id: "run-active" }))).toMatchObject({
      id: "resume-last-active",
      category: "chat",
      action: "navigate",
      payload: "/chat?run=run-active&resume=resume",
    });
  });
});
