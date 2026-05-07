# Phase 8 Design: Roundtable Enhancements + Gemini Cleanup + Context Events

**Date:** 2026-05-07
**Status:** Draft v2 (incorporating cx2cc + deepseek review feedback)

---

## Overview

Six coordinated changes to the Claude Session Hub Tauri app:

1. **Gemini Removal** — Complete removal of all Gemini provider support (~54 files)
2. **Stepper Mini-Map** — Replace History strip with a turn-by-turn stepper for room replay
3. **@Name SingleTarget** — `@DisplayName msg` sends a public turn to only the named participant
4. **Room Session Grouping** — Virtual "Rooms" folder in sidebar for room participant runs
5. **Roundtable Prompt Constraint** — Bilingual "discussable judgment, not actionable plan" constraint
6. **Context Events Verification** — Ensure all CC session types correctly display context events in the UI

---

## 1. Gemini Removal

### Scope

Remove all Gemini references from frontend, backend, tests, and configuration. The README already marks Gemini as deprecated.

### Files to modify (~54 files)

**Type definitions (frontend):**
- `src/lib/types.ts` — Remove `"gemini"` from `AgentKind`, `ConnectionProfile.agent`, `CcAgentProfile.agent`, `MemoryFileCandidate.provider`
- `src/lib/utils/provider-catalog.ts` — Remove `"gemini"` from `ExecutionAgent`, `Phase7ProviderId`, delete Gemini provider entry from `PHASE7_PROVIDERS`, remove from `providerIdForRun()`
- `src/lib/utils/agent-capabilities.ts` — Remove `"gemini"` branch in `normalizeAgentKind()` and capabilities fallback
- `src/lib/utils/agent-features.ts` — Remove `"gemini"` from `FEATURES_MAP`
- `src/lib/utils/native-permission.ts` — Remove `"gemini"` from `NATIVE_AGENTS`
- `src/lib/utils/continuable-run.ts` — Remove Gemini-specific continuation logic
- `src/lib/commands.ts` — Remove `"gemini"` from `CommandAgent`, delete `new-gemini` command palette entry
- `src/lib/api.ts` — Remove `"gemini"` from `ManagedCliApp` and API function param types

**UI routes/components:**
- `src/routes/chat/+page.svelte` — Remove `"gemini"` from `CHAT_AGENTS`, `startupCliChecks`, CLI check logic, URL query param handling
- `src/routes/settings/+page.svelte` — Remove Gemini tab from `ConnectionAgentTab`, CLI check state
- `src/routes/rooms/+page.svelte` — Remove default Gemini participant slot (line 262-268), agent detection
- `src/routes/plugins/+page.svelte` — Remove Gemini managed app entry
- `src/lib/components/McpConfiguredPanel.svelte` — Remove `"gemini"` from `app` prop type
- `src/lib/components/McpDiscoverPanel.svelte` — Remove `"gemini"` from `app` prop type

**Stores:**
- `src/lib/stores/agent-settings-cache.svelte.ts` — Remove Gemini settings loading
- `src/lib/stores/session-store.svelte.ts` — Remove Gemini-specific resume logic and comments

**Rust backend:**
- `src-tauri/src/agent/spawn.rs` — Remove `build_gemini_base_args()`, `"gemini"` match arms in `build_agent_command()` and `build_agent_resume_command()`, `resolve_windows_npm_shim()` gemini branch, all Gemini tests
- `src-tauri/src/agent/stream.rs` — Remove `"gemini"` from `native_transcript_mode` check
- `src-tauri/src/agent/native_transcript.rs` — Remove all `find_gemini_session()`, `find_latest_gemini_session()`, `parse_gemini_turn()`, `wait_for_gemini_turn()` functions and tests
- `src-tauri/src/agent/native_pty.rs` — Remove Gemini comments and test references
- `src-tauri/src/agent/adapter.rs` — Remove `AgentKind::Gemini` variant, Gemini capabilities, `default_api_key_env("gemini")`, `default_base_url_env("gemini")`
- `src-tauri/src/models.rs` — Remove `"gemini"` from `AllSettings::default()`
- `src-tauri/src/storage/managed_apps.rs` — Remove `ManagedCliApp::Gemini` variant
- `src-tauri/src/storage/mcp_registry.rs` — Remove all `list_configured_gemini()`, `read_gemini_settings()`, `write_gemini_settings()`, `write_gemini_mcp_server()`, `remove_gemini_mcp_server()`, `toggle_gemini_server()`, `gemini_settings_path()`
- `src-tauri/src/storage/settings.rs` — Verify and remove any Gemini-specific read/write functions (not in original list; flagged by review)
- `src-tauri/src/commands/files.rs` — Remove `~/.gemini/` protected dir, Gemini memory file detection
- `src-tauri/src/commands/rooms.rs` — Remove `"gemini"` from agent validation, error message, default label
- `src-tauri/src/commands/runs.rs` — Remove Gemini test cases
- `src-tauri/src/commands/diagnostics.rs` — Remove Gemini CLI binary mapping

**Tests (frontend):**
- `src/lib/utils/provider-catalog.test.ts`
- `src/lib/utils/agent-capabilities.test.ts`
- `src/lib/utils/continuable-run.test.ts`
- `src/lib/utils/room-ui.test.ts`
- `src/lib/utils/__tests__/agent-features.test.ts`
- `src/lib/utils/__tests__/native-permission.test.ts`
- `src/lib/utils/__tests__/add-dir-action.test.ts`
- `src/lib/stores/room-store.test.ts`
- `src/lib/stores/session-store.test.ts`

**i18n:** Verify with `grep -i gemini messages/` — no Gemini-specific messages expected, but confirm.

### Historical Gemini Runs

Existing runs with `agent === "gemini"` are preserved as read-only history. UI fallback:
- In `provider-catalog.ts` `providerIdForRun()`: return a fallback label `"Gemini (deprecated)"` for runs with `agent === "gemini"` instead of panicking
- In `agent-capabilities.ts` `normalizeAgentKind()`: map `"gemini"` to `"unknown"` (not a hard error)
- Sidebar and history pages render these runs normally with the deprecated label

### Approach

Systematic file-by-file removal. For type unions, remove the `"gemini"` variant. For match arms / if-else branches, remove the Gemini case. For functions that are Gemini-only, delete the entire function. For tests, remove Gemini test cases or adjust test data.

### Risk

Medium. The scope (~54 files, cross-cutting frontend/backend/types/tests) means high chance of missed string branches or test fixture drift. Mitigation: grep-based post-removal verification.

---

## 2. Stepper Mini-Map (Replace History Strip)

### Current State

The History strip (`rooms/+page.svelte` lines 608-683) shows collapsible turn chips with `#turn.idx`, mode, user_input, and per-participant status dots. It's a flat list — no snapshot/replay capability.

### Data Model Verification

`RoomResponseRef` in `src-tauri/src/room/models.rs` (lines 49-58) **does** include `event_seq_start: u64` and `event_seq_end: u64`. These are valid seq numbers into the run's `events.jsonl`. The `list_events(run_id, since_seq)` function in `events.rs` can read events by seq range. Snapshot loading is feasible via:

```
for each response in turn.responses:
    events = list_events(response.run_id, 0)
    snapshot_events = events.filter(e => e.seq >= response.event_seq_start && e.seq <= response.event_seq_end)
    content = extract_assistant_text(snapshot_events)
```

### Design

Replace the History strip with a **vertical stepper** component:

**Layout:**
```
┌─────────────────────────────────────────────┐
│  ○ Turn 1 · fanout · "讨论架构选型"          │
│  │                                          │
│  ○ Turn 2 · debate · "@debate"              │
│  │                                          │
│  ◉ Turn 3 · fanout · "性能优化方案"  ← active│
│  │                                          │
│  ○ Turn 4 · summary · "@summary @Claude"    │
│  └───────────────────────────────────────────│
│                                              │
│  ┌─ 🟣 只读历史 · 第 3 轮快照 ──────────────┐│
│  │  [退出快照]                               ││
│  ├──────────────────────────────────────────┤│
│  │  [Participant A pane] [Participant B] [C] ││
│  │  shows turn-3 snapshot data               ││
│  └──────────────────────────────────────────┘│
└─────────────────────────────────────────────┘
```

**Behavior:**
- Each turn is a clickable element in a vertical stepper
- Dot color: green (all complete), amber (in progress), red (has failure), gray (pending)
- Turn text summary is always visible as inline label next to the dot (not hidden in tooltip)
- Tooltip provides additional detail (full user_input, per-participant status) as enhancement only
- Clicking a dot enters **snapshot mode**:
  - Participant panes switch to show that turn's response data
  - Purple banner appears: 「只读历史 · 第 N 轮快照」with an「退出快照」button
  - Composer is disabled
- Clicking the latest turn or「退出快照」returns to **live mode**
- The stepper is scrollable when there are many turns

### Snapshot Pane Rendering

When snapshot mode is active, participant panes need to render static content instead of live streams.

**Approach:** Add `readonly` mode to existing pane rendering:
- Each pane receives `snapshotContent: string | null` prop
- When `snapshotContent` is non-null, render it as formatted markdown (reuse existing `ChatMessage` component in readonly mode)
- When null, render live session data as before

**Two-phase loading:**
1. **Immediate (from timeline):** Use `RoomResponseRef.preview` (already stored, truncated) for instant display
2. **Full (on demand):** When user clicks a snapshot dot, fire `get_room_turn_snapshot` Tauri command to load full event data between `event_seq_start..event_seq_end`

### New Files

- `src/lib/components/RoomStepper.svelte` — The stepper component
- New Tauri command: `get_room_turn_snapshot` in `src-tauri/src/commands/rooms.rs`

### `get_room_turn_snapshot` Command

```rust
#[tauri::command]
pub fn get_room_turn_snapshot(room_id: String, turn_id: String) -> Result<RoomTurnSnapshot, String>
```

Returns:
```rust
pub struct RoomTurnSnapshot {
    pub turn: RoomTurn,
    pub participant_contents: Vec<ParticipantSnapshot>,
}

pub struct ParticipantSnapshot {
    pub participant_id: String,
    pub label: String,
    pub content: String,       // Full assistant text extracted from events
    pub status: String,
    pub error: Option<String>,
}
```

Implementation: read `timeline.jsonl` to find the turn, then for each `RoomResponseRef`, read events from `events.jsonl` by seq range and extract assistant message content.

### Modified Files

- `src/routes/rooms/+page.svelte` — Replace History strip section with `RoomStepper`, add snapshot mode state
- `src/lib/stores/room-store.svelte.ts` — Add `snapshotTurn`, `exitSnapshot` methods, `activeSnapshot` state
- `src/lib/types.ts` — Add `RoomTurnSnapshot`, `ParticipantSnapshot` types
- `messages/en.json`, `messages/zh-CN.json` — Add i18n keys for snapshot banner

### Edge Cases

- Soft-deleted runs: `get_room_turn_snapshot` should handle missing `events.jsonl` gracefully — return `preview` fallback with status `"deleted"`
- Large turn counts (20+): stepper uses CSS `overflow-y: auto` with max-height, no virtualization needed for typical room sizes
- Rapid consecutive turns: snapshot data is read-only from persisted JSONL, no consistency issues

### Private Turns in Stepper

Existing private turns (stored in `private.json`) do NOT appear in the stepper — only public `timeline.jsonl` turns are shown.

---

## 3. @Name SingleTarget (Public Turn to One Participant)

### Current State

`parse_roundtable_command()` in `orchestrator.rs` (lines 677-707) handles `@TargetName message` as `RoundtableCommand::Private` — stored in `private.json`, not visible in public timeline.

### Design

Two separate entry points:

| Syntax | Mode | Behavior |
|--------|------|----------|
| `@Name msg` | **SingleTarget** (public) | Named participant answers, turn written to `timeline.jsonl` |
| `/dm @Name msg` | **Private** (existing) | Named participant answers privately, written to `private.json` |

This preserves backward compatibility — users who want private messaging use `/dm`.

### Naming Convention

| Layer | Value |
|-------|-------|
| Rust enum variant | `RoomTurnMode::SingleTarget` |
| Rust command enum | `RoundtableCommand::SingleTarget { target, message }` |
| TypeScript wire value | `"singletarget"` |
| UI label (i18n) | "Single Target" / "指名回答" |
| Prompt header | `@single-target` |

### Changes

**`src-tauri/src/room/models.rs`:**
- Add `SingleTarget` variant to `RoomTurnMode` enum

**`src-tauri/src/room/orchestrator.rs`:**
- Change `parse_roundtable_command()`:
  - `@TargetName message` → `RoundtableCommand::SingleTarget { target, message }`
  - `/dm @TargetName message` → `RoundtableCommand::Private { target, message }` (preserved)
- Add `build_singletarget_prompt()` — addressed to one participant, includes instruction that only this participant is answering
- In `run_roundtable_turn_with_runtime()`, handle `SingleTarget` mode: resolve target via `find_participant()`, send prompt only to that participant, write turn to public timeline
- **Ambiguity handling:** If `find_participant()` matches multiple participants (same label), return error `"Ambiguous participant name '{name}' — please use a unique label"`
- **Not found handling:** If `find_participant()` returns None, return error `"Participant '{name}' not found in this room"`

**`src/lib/utils/room-ui.ts`:**
- Add `"singletarget"` to `roomTurnModeLabel()` and `roomTurnModeColor()`

**`src/lib/types.ts`:**
- Add `"singletarget"` to `RoomTurnMode` union

**`messages/en.json`, `messages/zh-CN.json`:**
- Add labels for SingleTarget mode and `/dm` help text

### Prompt Template

```
[通用圆桌 · 第 {turn_num} 轮 · @single-target → {target_label}]

## 用户指名提问
{user_message}

你是本轮唯一被指名回答的参与者。请给出你的完整观点。
```

---

## 4. Room Session Grouping (Virtual "Rooms" Folder in Sidebar)

### Current State

Room participant runs are stored in `runs/` alongside standalone runs. `RunMeta` has no `room_id` field. The sidebar groups by `cwd` → `session_id`, so room runs appear mixed with regular sessions.

### Design

Add a virtual **"Rooms"** project folder at the top of the sidebar that groups all room participant runs by room name.

### Approach: New Backend Command + Frontend Grouping

To avoid N+1 queries (loading each room's detail individually), add a single lightweight Tauri command:

**New command: `list_room_run_index`**

```rust
#[tauri::command]
pub fn list_room_run_index() -> Result<Vec<RoomRunIndexEntry>, String>

pub struct RoomRunIndexEntry {
    pub room_id: String,
    pub room_name: String,
    pub room_kind: String,
    pub run_ids: Vec<String>,
}
```

Implementation: scan `rooms/` directory, read each `room.json`, extract `id`, `name`, `kind`, and `participants[].run_id`. Single IPC call returns the full mapping.

**Frontend data flow:**
1. On sidebar mount, call `list_room_run_index()` once
2. Build `Map<run_id, { room_id, room_name, room_kind }>` lookup
3. In `buildProjectFolders()`:
   - Runs whose ID is in the lookup are **extracted** from their normal cwd folder
   - They are grouped into a virtual `ProjectFolder` with `folderKey: "rooms"`, `cwd: "__rooms__"` (sentinel)
   - Within the Rooms folder, sub-grouped by room name

### Modified Files

- `src-tauri/src/commands/rooms.rs` — Add `list_room_run_index` command
- `src-tauri/src/lib.rs` — Register new command
- `src/lib/utils/sidebar-groups.ts`:
  - Add `roomRunIds: Map<string, { roomId, roomName, roomKind }>` parameter to `buildProjectFolders()`
  - Filter room runs out of normal cwd folders
  - Create virtual "Rooms" folder with room-run sub-groups
- `src/lib/components/` (sidebar host) — Call `list_room_run_index` on mount, pass to `buildProjectFolders()`
- `src/lib/components/ProjectFolderItem.svelte` — Handle the "Rooms" virtual folder (special icon, non-removable)
- `src/lib/api.ts` — Add API wrapper for `list_room_run_index`
- `messages/en.json`, `messages/zh-CN.json` — Add "Rooms" label

### UX Details

- The "Rooms" folder appears at the top of the sidebar (always pinned, non-removable)
- Each room is a sub-section showing room name and kind badge
- **Click behavior:**
  - Single-click a run entry → navigate to `/rooms?room={room_id}` (the room page, where the run is contextualized)
  - The room page is the natural context for room participant runs, not the standalone chat view
- The "Rooms" folder cannot be hidden/removed (unlike regular cwd folders)
- Room runs are excluded from their original cwd folder to avoid duplication

### Edge Cases

- Room participant runs that have been soft-deleted: filtered out by checking `run.status !== "deleted"`
- Rooms with no active participants: filtered out (empty `run_ids` after filtering)
- Room name empty/fallback: use `"未命名房间"` / `"Unnamed Room"` (i18n)
- **Pinned runs conflict:** If a run is both pinned and belongs to a room, pinned status takes precedence — the run stays in its pinned position and is NOT extracted into the Rooms folder. This avoids confusion.
- **Data refresh:** Listen for `ocv:runs-changed` Tauri event to re-fetch `list_room_run_index`. No new event needed — room changes always trigger run changes.

---

## 5. Roundtable Prompt Constraint

### Current State

The roundtable prompts in `orchestrator.rs` instruct participants to "answer independently" and "be concrete" but do not scope the output as non-actionable.

### Design

Add a **bilingual** constraint to the default seat prompt (frontend) and the fanout/debate/summary prompt builders (backend):

### Constraint Text (shared constant)

Define a Rust const in `orchestrator.rs`:

```rust
const ROUNDTABLE_SCOPE_CONSTRAINT: &str = "\
重要：圆桌输出属于「讨论性判断」——非研究报告，非可执行方案。当需要实际行动时，建议切换到独立会话执行。\n\
IMPORTANT: Roundtable outputs are \"discussable judgments\" — not research reports, \
not executable plans. When the user needs to take action, suggest switching to an \
independent session for implementation.";
```

### Modified Locations

**Frontend (`rooms/+page.svelte`):**
- `defaultSeatPromptWithLabel()` — append constraint (verify line number at implementation time)

**Backend (`orchestrator.rs`):**
- `build_fanout_prompt()` (line 810-821) — append constraint after the "请独立回答" instruction
- `build_debate_prompt()` (line 739-808) — append constraint after the debate instruction
- `build_summary_prompt()` (line 823-860) — append constraint after the summary instruction

Append this to each prompt template with a blank line separator.

---

## 6. Context Events Verification

### Current State

The app already has context usage UI components:
- `ContextUsageGrid` — visualizes context usage
- `CostSummaryView` — cost/token stats
- `session-store.svelte.ts` — tracks `contextWindow`, `rateLimitUtilization`, `rateLimitResetsAt`, token counts
- `parseContextMarkdown` — parses context info from session events

These components exist but it's unclear whether they are consistently visible for all CC session types.

### Goal

Ensure all CC session types (Claude native, Claude-compatible API providers, Codex native PTY) correctly emit and display context usage events in the chat UI.

### Execution Paths to Verify

| Path | Used by | context_window events | Status |
|------|---------|----------------------|--------|
| SessionActor / stream-session | Claude native, API providers (via `--settings`) | Source: Claude CLI stream-json | **Verify** |
| PipeExec | Legacy/deprecated | Source: stdout parsing | **Verify** |
| Native PTY | Codex | Source: transcript file | **Verify** |

### Investigation Steps

1. Start a Claude native session → check if `ContextUsageGrid` renders in chat header
2. Start a DeepSeek/MiMo session (provider-based) → check same
3. Start a Codex session → check same
4. For any path where context events are missing:
   - Trace the event emission path in `stream.rs` / `native_transcript.rs`
   - Add context_window event forwarding if absent

### Likely Outcome

Since Claude-compatible providers use `SessionActor` with `--settings` injection (same stream-session path as Claude native), context events should already work. Codex native PTY may be the gap — transcript parsing may not forward context_window events.

### Modified Files (after investigation)

- `src-tauri/src/agent/native_transcript.rs` — Forward context events from transcript if missing
- `src/routes/chat/+page.svelte` — Ensure `ContextUsageGrid` is rendered in the header area for all session types (not hidden in a sub-tab)
- No new components needed — existing components are sufficient

---

## Implementation Order

Two-phase approach: investigation spikes first, then implementation.

### Phase 0: Investigation Spikes (30 min)

1. Verify `ContextUsageGrid` visibility for each session type (Item 6)
2. Confirm `event_seq_start/end` usage in `get_room_turn_snapshot` approach (Item 2) — **DONE: fields confirmed to exist**
3. Confirm `list_room_run_index` feasibility — scan `rooms/` dir, read `room.json` (Item 4)

### Phase 1: Implementation

1. **Gemini Removal** (Item 1) — Clean foundation, no dependencies
2. **Prompt Constraint** (Item 5) — Trivial, single const + append
3. **@Name SingleTarget** (Item 3) — Small scope, backend + frontend
4. **Room Session Grouping** (Item 4) — New Tauri command + frontend grouping
5. **Context Events** (Item 6) — Fix any gaps found in investigation
6. **Stepper Mini-Map** (Item 2) — Largest scope, new component + Tauri command

---

## Testing Strategy

- **Gemini Removal:** Run `npm run check`, `npm test`, `cargo check` — all should pass with zero Gemini references. (Note: `cargo test` fails on this machine due to VC++ redistributable mismatch; use `cargo check` for Rust validation.)
- **Stepper:** Test with a multi-turn room, verify snapshot loads correct data, banner appears, exit works
- **SingleTarget:** Test `@Alice hello` produces a public turn with only Alice responding; test `/dm @Alice hello` produces private turn; test `@UnknownName` returns error
- **Room Grouping:** Create a room, verify runs appear in "Rooms" folder, not in cwd folder; verify pinned runs stay in their pinned position
- **Prompt Constraint:** Verify generated prompts include the bilingual constraint text
- **Context Events:** Test with Claude native and provider-based session, verify context % displays for both
