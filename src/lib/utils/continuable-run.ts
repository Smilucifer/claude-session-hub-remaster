import type { TaskRun } from "$lib/types";

const FINISHED_STATUSES = new Set(["completed", "stopped", "failed"]);

export function findLastContinuableRun(runs: TaskRun[], agent: string): TaskRun | null {
  const normalizedAgent = agent.trim().toLowerCase();
  return (
    runs.find((run) => {
      if (run.agent !== normalizedAgent) return false;
      if (!FINISHED_STATUSES.has(run.status)) return false;

      // The current continue path is backed by Claude session_id resume.
      // Codex thread refs are captured for history, but are not wired to
      // resumeSession yet, so showing a continue button would fail.
      return normalizedAgent === "claude" && !!run.session_id;
    }) ?? null
  );
}
