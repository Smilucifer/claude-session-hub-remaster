# Phase 1 Memo Implementation Plan

**Feature:** Phase 1.a / 1.b - Global and Project Memo
**Status:** Done
**Merged:** `216d500 feat: add phase 1 memo foundation`
**Goal:** Ship a small persistent memo surface that gives immediate value without touching Claw GO session orchestration.
**Acceptance Criteria:**
- Users can add, edit, copy, delete, and clear memo items.
- Global memo survives app restart.
- Project memo is stored separately per project cwd and switches when the active cwd changes.
- Missing memo files return empty state; corrupt memo files do not crash the app.
- Existing `/chat` behavior is unchanged.

**Architecture:** Store memo data in native Rust storage, expose narrow Tauri commands, and keep frontend state in a small Svelte store. Global and project memo share the same schema; scope only changes the storage path.
**Tech Stack:** Tauri v2 commands, Rust `serde`, local JSON files, Svelte 5 stores, Vitest, Rust unit tests.
**Frontend Verification:** Yes - reviewer must open the app and exercise add/edit/copy/delete/clear for global and project scopes.

---

## Finish Line

Phase 1 Memo is complete when Claw GO has a command-accessible memo model with a visible frontend panel. The implementation should be small enough to land before Room work starts, but permanent enough to extend later with room memo.

Completion note, 2026-04-30: Phase 1.a / 1.b landed before Room work and is treated as complete for roadmap purposes. Room memo was delivered separately with Phase 2's room storage/UI.

Out of scope:

- Room memo.
- Arena Memory.
- Prompt syncing into Claude/Codex/Gemini.
- Hub memo import.
- Sidebar redesign.
- Multi-user sharing.

## Terminal Schema

Rust:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoItem {
    pub id: String,
    pub text: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MemoScope {
    Global,
    Project { cwd: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoFile {
    schema_version: u32,
    items: Vec<MemoItem>,
}
```

TypeScript:

```ts
export type MemoItem = {
  id: string;
  text: string;
  createdAt: string;
  updatedAt: string;
};

export type MemoScope =
  | { kind: 'global' }
  | { kind: 'project'; cwd: string };
```

Storage paths:

```text
data_dir()/memos/global.json
data_dir()/projects/{project-key}/memo.json
```

Use a stable project key derived from canonical cwd. Prefer a short SHA-256 hex key over `cli_sessions::encode_cwd`, because memo paths should not expose full project paths and should be stable across slash styles.

## Task 0: Data Directory Override

**Files:**

- Modify: `src-tauri/src/storage/mod.rs`
- Test: `src-tauri/src/storage/mod.rs`

**Steps:**

1. Add `CLAWGO_DATA_DIR` support to `storage::data_dir()`.
2. Keep existing home-dir fallback when the env var is absent or empty.
3. Add tests for env override and fallback behavior. Guard env mutation with a test mutex or equivalent serial helper so parallel Rust tests do not race.

Expected behavior:

```rust
// Pseudocode
std::env::set_var("CLAWGO_DATA_DIR", temp.path());
assert_eq!(storage::data_dir(), temp.path());
```

Run:

```bash
npm run rust:check
```

## Task 1: Backend Memo Storage

**Files:**

- Create: `src-tauri/src/storage/memos.rs`
- Modify: `src-tauri/src/storage/mod.rs`
- Test: `src-tauri/src/storage/memos.rs`

**Steps:**

1. Write failing tests for missing file, corrupt file, add, update, delete, clear, and separate global/project scopes.
2. Implement `MemoScope`, `MemoItem`, and internal `MemoFile`.
3. Implement path resolution:
   - global: `data_dir()/memos/global.json`
   - project: `data_dir()/projects/{project-key}/memo.json`
4. Implement atomic save through temp-file plus rename, following `storage/favorites.rs` style.
5. Keep corrupt file fallback conservative: return empty list and log a warning, do not delete the corrupt file in Phase 1.

Public storage API:

```rust
pub fn list_memos(scope: MemoScope) -> Vec<MemoItem>;
pub fn add_memo(scope: MemoScope, text: String) -> Result<MemoItem, String>;
pub fn update_memo(scope: MemoScope, id: String, text: String) -> Result<MemoItem, String>;
pub fn delete_memo(scope: MemoScope, id: String) -> Result<(), String>;
pub fn clear_memos(scope: MemoScope) -> Result<(), String>;
```

Run:

```bash
npm run rust:check
```

## Task 2: Tauri Commands

**Files:**

- Create: `src-tauri/src/commands/memos.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify if shared models are preferred: `src-tauri/src/models.rs`

**Steps:**

1. Write command smoke tests where practical, or storage-backed command tests if the existing Tauri test harness supports it.
2. Expose commands:

```rust
#[tauri::command]
pub fn list_memos(scope: MemoScope) -> Result<Vec<MemoItem>, String>;

#[tauri::command]
pub fn add_memo(scope: MemoScope, text: String) -> Result<MemoItem, String>;

#[tauri::command]
pub fn update_memo(scope: MemoScope, id: String, text: String) -> Result<MemoItem, String>;

#[tauri::command]
pub fn delete_memo(scope: MemoScope, id: String) -> Result<(), String>;

#[tauri::command]
pub fn clear_memos(scope: MemoScope) -> Result<(), String>;
```

3. Register commands in `lib.rs`.
4. Validate text: trim empty input at command boundary and return a clear error.

Run:

```bash
npm run rust:check
```

## Task 3: Frontend API and Store

**Files:**

- Modify: `src/lib/api.ts`
- Modify: `src/lib/types.ts`
- Create: `src/lib/stores/memo-store.svelte.ts`
- Test: `src/lib/stores/memo-store.test.ts`

**Steps:**

1. Add `MemoItem` and `MemoScope` types to `types.ts`.
2. Add typed API wrappers for all memo commands in `api.ts`.
3. Implement a Svelte store with:
   - `scope`
   - `items`
   - `loading`
   - `error`
   - `load(scope)`
   - `add(text)`
   - `update(id, text)`
   - `delete(id)`
   - `clear()`
4. Write Vitest coverage for optimistic/non-optimistic state transitions. Prefer simple await-and-refresh behavior for Phase 1; optimistic updates are optional.

Run:

```bash
npm run lint
npm run check
npm test
```

## Task 4: Memo UI

**Files:**

- Create: `src/lib/components/memo/MemoPanel.svelte`
- Modify: command palette or app shell entry point used by existing lightweight tools
- Modify if needed: localized message catalogs for new visible copy

**Steps:**

1. Add a floating or command-palette opened panel, not a permanent sidebar.
2. Provide global/project scope switch if active cwd is known; otherwise show global only.
3. Provide item actions:
   - copy
   - edit
   - delete
4. Provide clear-all with confirmation.
5. Gate the visible entry point behind the agreed runtime setting if the feature flag is already available when this lands.
6. Keep the UI compact and workbench-like. Do not introduce a landing page, decorative cards, or a three-pane redesign.

Manual verification:

```text
1. Open the app.
2. Open Memo from the chosen entry point.
3. Add a global memo.
4. Restart the app and confirm it persists.
5. Edit, copy, delete, and clear.
6. Switch project cwd and confirm project memo isolation.
7. Confirm /chat remains reachable and normal.
```

Run:

```bash
npm run lint
npm run check
npm test
```

## Task 5: Project Scope Integration

**Files:**

- Modify: `src/lib/stores/session-store.svelte.ts` or the current source of active cwd
- Modify: `src/lib/stores/memo-store.svelte.ts`
- Test: `src/lib/stores/memo-store.test.ts`

**Steps:**

1. Identify the existing frontend source of active cwd.
2. Load `{ kind: 'project', cwd }` when project memo is selected.
3. Refresh memo items when active cwd changes.
4. Treat missing or invalid cwd as global-only UI state.

Run:

```bash
npm run lint
npm run check
npm test
npm run rust:check
```

## Quality Gates

Required before claiming Phase 1 Memo complete:

```bash
npm run lint
npm run check
npm test
npm run rust:check
```

Run the broader gate when frontend and backend changes are both substantial:

```bash
npm run verify
```

Manual evidence:

- Global memo add/edit/copy/delete/clear works.
- Global memo survives restart.
- Project memo is isolated by cwd.
- Corrupt memo file does not crash the app.
- `/chat` still works.

## Handoff Notes

- Keep memo storage independent of Room. Room memo should reuse the schema later, but Phase 1 should not introduce `room_id`.
- Keep text-only memo items for now. Tags, markdown previews, and prompt insertion are future extensions.
- Do not import Hub memo data in Phase 1.
- Do not duplicate memo text into run `events.jsonl`.
