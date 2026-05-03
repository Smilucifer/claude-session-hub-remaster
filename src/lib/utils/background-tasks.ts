import type { TaskNotificationItem } from "$lib/stores/session-store.svelte";

const INACTIVE_TASK_STATUSES = new Set(["completed", "failed", "error", "cancelled", "canceled"]);

export function isActiveBackgroundTask(item: TaskNotificationItem): boolean {
  return !INACTIVE_TASK_STATUSES.has(item.status);
}

export function sortBackgroundTasks(items: TaskNotificationItem[]): TaskNotificationItem[] {
  return [...items].sort((a, b) => {
    const aActive = isActiveBackgroundTask(a) ? 0 : 1;
    const bActive = isActiveBackgroundTask(b) ? 0 : 1;
    if (aActive !== bActive) return aActive - bActive;
    return b.startedAt - a.startedAt;
  });
}
