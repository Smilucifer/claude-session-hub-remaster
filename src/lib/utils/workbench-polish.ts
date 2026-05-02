import type { CommandDef } from "$lib/commands";
import type { TaskRun } from "$lib/types";
import type { ConversationGroup } from "./sidebar-groups";
import { truncate } from "./format";

export type SeenMessageCounts = Record<string, number>;

export function previewForRun(run: TaskRun): string {
  return truncate(run.last_message_preview?.trim() || run.prompt?.trim() || "", 72);
}

export function previewForConversation(conversation: ConversationGroup): string {
  return previewForRun(conversation.latestRun);
}

export function conversationUnreadCount(
  conversation: ConversationGroup,
  seen: SeenMessageCounts,
): number {
  return conversation.runs.reduce((sum, run) => {
    const current = run.message_count ?? 0;
    const previous = seen[run.id] ?? 0;
    return sum + Math.max(0, current - previous);
  }, 0);
}

export function markConversationSeen(
  seen: SeenMessageCounts,
  conversation: ConversationGroup,
): SeenMessageCounts {
  const next = { ...seen };
  for (const run of conversation.runs) {
    next[run.id] = run.message_count ?? 0;
  }
  return next;
}

export function buildLastActiveResumePath(run: TaskRun): string {
  return `/chat?run=${encodeURIComponent(run.id)}&resume=resume`;
}

export function buildResumeLastActiveCommand(run: TaskRun | undefined): CommandDef | undefined {
  if (!run) return undefined;
  return {
    id: "resume-last-active",
    name: "Resume Last Active Session",
    description: "Open the most recently active resumable session",
    category: "chat",
    agent: "both",
    action: "navigate",
    payload: buildLastActiveResumePath(run),
  };
}
