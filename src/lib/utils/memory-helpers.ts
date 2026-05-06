/**
 * Filter memory file candidates for sidebar display.
 *
 * - Always shows files that exist on disk.
 * - Shows non-existing (creatable) files only when `showCreate` is true,
 *   OR when the file is currently selected (to keep highlight visible).
 */
export function filterVisibleCandidates<T extends { path: string; exists: boolean }>(
  files: T[],
  showCreate: boolean,
  selectedPath: string,
): T[] {
  return files.filter((f) => f.exists || showCreate || f.path === selectedPath);
}
