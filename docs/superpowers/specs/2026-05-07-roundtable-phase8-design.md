# Phase 8 Design: Roundtable Enhancements + Gemini Cleanup + Context HUD

**Date:** 2026-05-07
**Status:** Draft

---

## Overview

Six coordinated changes to the Claude Session Hub Tauri app:

1. **Gemini Removal** — Complete removal of all Gemini provider support (~54 files)
2. **Stepper Mini-Map** — Replace History strip with a turn-by-turn stepper for room replay
3. **@Name SingleTarget** — `@DisplayName msg` sends a public turn to only the named participant
4. **Room Session Grouping** — Virtual "Rooms" folder in sidebar for room participant runs
5. **Roundtable Prompt Constraint** — Add English "discussable judgment, not actionable plan" constraint
6. **Context Usage HUD** — Ensure all CC sessions report context usage to the frontend (replaces external ccstatusline need)

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

**i18n:** No Gemini-specific messages found — no changes needed.

### Approach

Systematic file-by-file removal. For type unions, remove the `"gemini"` variant. For match arms / if-else branches, remove the Gemini case. For functions that are Gemini-only, delete the entire function. For tests, remove Gemini test cases or adjust test data.

### Risk

Low. Gemini was already deprecated. No other code depends on Gemini paths.

---

## 2. Stepper Mini-Map (Replace History Strip)

### Current State

The History strip (`rooms/+page.svelte` lines 608-683) shows collapsible turn chips with `#turn.idx`, mode, user_input, and per-participant status dots. It's a flat list — no snapshot/replay capability.

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
- Each turn is a clickable dot (○) in a vertical stepper
- Dot color: green (all complete), amber (in progress), red (has failure), gray (pending)
- Clicking a dot enters **snapshot mode**:
  - Participant panes switch to show that turn's response data (loaded from `timeline.jsonl` `responses[]` refs)
  - Purple banner appears: 「只读历史 · 第 N 轮快照」with an「退出快照」button
  - Composer is disabled
- Clicking the latest turn or「退出快照」returns to **live mode**
- The stepper is scrollable when there are many turns
- Each dot shows a tooltip with turn mode and truncated user_input

### Data Flow

1. `RoomTurn` data already available in `store.room.turns[]`
2. `RoomResponseRef` contains `preview` (truncated response) and references to run events
3. For full snapshot: read run events between `event_seq_start` and `event_seq_end` via a new Tauri command `get_room_turn_snapshot(room_id, turn_id)`
4. New Tauri command in `commands/rooms.rs`: reads the referenced run events and returns the full response text for each participant in that turn

### New Files

- `src/lib/components/RoomStepper.svelte` — The stepper component
- New Tauri command: `get_room_turn_snapshot` in `src-tauri/src/commands/rooms.rs`

### Modified Files

- `src/routes/rooms/+page.svelte` — Replace History strip section with `RoomStepper`, add snapshot mode state
- `src/lib/stores/room-store.svelte.ts` — Add `snapshotTurn`, `exitSnapshot` methods, `activeSnapshot` state
- `src/lib/types.ts` — Add `RoomTurnSnapshot` type if needed
- `messages/en.json`, `messages/zh-CN.json` — Add i18n keys for snapshot banner

### Private Turns in Stepper

Existing private turns (stored in `private.json`) do NOT appear in the stepper — only public `timeline.jsonl` turns are shown. This is consistent with the current History strip behavior.

### Snapshot Mode State

```typescript
// In room-store
activeSnapshot: RoomTurn | null = null;  // null = live mode

snapshotTurn(turn: RoomTurn) { this.activeSnapshot = turn; }
exitSnapshot() { this.activeSnapshot = null; }
```

When `activeSnapshot` is set, the participant panes render snapshot data instead of live session data.

---

## 3. @Name SingleTarget (Public Turn to One Participant)

### Current State

`parse_roundtable_command()` in `orchestrator.rs` (lines 677-707) handles `@TargetName message` as `RoundtableCommand::Private` — stored in `private.json`, not visible in public timeline.

### Design

Change `@TargetName message` to produce a **public SingleTarget turn**:

- `target_participant_ids` contains only the named participant
- Stored in `timeline.jsonl` (public), not `private.json`
- Only the named participant receives the prompt and responds
- Other participants do NOT receive any message
- The turn appears in the stepper with mode `"singletarget"` (new variant)

### Changes

**`src-tauri/src/room/models.rs`:**
- Add `SingleTarget` variant to `RoomTurnMode` enum
- Update TypeScript type in `src/lib/types.ts` to match

**`src-tauri/src/room/orchestrator.rs`:**
- Change `parse_roundtable_command()`: `@TargetName message` returns `RoundtableCommand::SingleTarget { target, message }` instead of `Private`
- Keep the `Private` variant in `RoundtableCommand` enum for backward compatibility (existing private turns in `private.json` still need to be readable), but no new Private turns will be created via `@name`
- Add `build_singletarget_prompt()` — similar to fanout but addressed to one participant, includes instruction that only this participant is answering
- In `run_roundtable_turn_with_runtime()`, handle `SingleTarget` mode: resolve target via `find_participant()`, send prompt only to that participant, write turn to public timeline
- If `find_participant()` fails (name not found), return an error to the frontend instead of falling through to fanout

**`src/lib/utils/room-ui.ts`:**
- Add `singletarget` to `roomTurnModeLabel()` and `roomTurnModeColor()`

**`messages/en.json`, `messages/zh-CN.json`:**
- Add label for "SingleTarget" mode

### Prompt Template

```
[通用圆桌 · 第 {turn_num} 轮 · @singletarget → {target_label}]

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

### Approach: Frontend-Only (No Backend Schema Change)

Instead of adding `room_id` to RunMeta (which requires migration), derive room membership from the existing room data at sidebar build time.

**Data flow:**
1. When building sidebar groups, also load `RoomSummary[]` via the existing `list_rooms` Tauri command
2. For each room, load its `RoomDetail` to get `participants[].run_id`
3. Build a `Map<run_id, room_name>` lookup
4. In `buildProjectFolders()`:
   - Runs whose ID is in the lookup are **extracted** from their normal cwd folder
   - They are grouped into a virtual `ProjectFolder` with `folderKey: "rooms"`, `cwd: "__rooms__"` (sentinel)
   - Within the Rooms folder, they are sub-grouped by room name (each room becomes a `ConversationGroup` or sub-section)

### Modified Files

- `src/lib/utils/sidebar-groups.ts`:
  - Add `roomRunIds: Map<string, string>` parameter to `buildProjectFolders()` (run_id → room_name)
  - Filter room runs out of normal cwd folders
  - Create virtual "Rooms" folder with room-run sub-groups
- `src/lib/components/Sidebar.svelte` (or wherever sidebar is rendered):
  - Load room summaries on mount
  - Pass `roomRunIds` to `buildProjectFolders()`
- `src/lib/components/ProjectFolderItem.svelte`:
  - Handle the "Rooms" virtual folder (special icon, non-removable)
- `messages/en.json`, `messages/zh-CN.json`:
  - Add "Rooms" label

### UX Details

- The "Rooms" folder appears at the top of the sidebar (pinned)
- Each room is a sub-section showing room name and kind badge
- Clicking a room run navigates to `/rooms?room={room_id}` (the room page)
- The "Rooms" folder cannot be hidden/removed (unlike regular cwd folders)
- Room runs are excluded from their original cwd folder to avoid duplication

### Edge Cases

- Room participant runs that have been soft-deleted should not appear
- Rooms with no active participants (all runs deleted) should not appear
- If a room is deleted, its runs disappear from the sidebar automatically (they're soft-deleted by `delete_room`)
- If a room has no `name` (empty string), use the room `id` (truncated) as the display name

---

## 5. Roundtable Prompt Constraint

### Current State

The roundtable prompts in `orchestrator.rs` instruct participants to "answer independently" and "be concrete" but do not scope the output as non-actionable.

### Design

Add the following English constraint to the **default seat prompt** (frontend) and the **fanout/debate/summary prompt builders** (backend):

```
IMPORTANT: Roundtable outputs are "discussable judgments" — not research reports, not executable plans. When the user needs to take action, the conclusion will suggest switching to an independent session for implementation.
```

### Modified Locations

**Frontend (`rooms/+page.svelte`):**
- `defaultSeatPromptWithLabel()` (line 342-343) — append constraint

**Backend (`orchestrator.rs`):**
- `build_fanout_prompt()` (line 810-821) — append constraint after the "请独立回答" instruction
- `build_debate_prompt()` (line 739-808) — append constraint after the debate instruction
- `build_summary_prompt()` (line 823-860) — append constraint after the summary instruction

### Constraint Text (shared constant)

Define a Rust const in `orchestrator.rs`:

```rust
const ROUNDTABLE_SCOPE_CONSTRAINT: &str = "\
IMPORTANT: Roundtable outputs are \"discussable judgments\" — not research reports, \
not executable plans. When the user needs to take action, suggest switching to an \
independent session for implementation.";
```

Append this to each prompt template with a blank line separator.

---

## 6. Context Usage HUD (Replace External ccstatusline Need)

### Current State

The app already has:
- `ContextUsageGrid` component — visualizes context usage
- `CostSummaryView` component — cost/token stats
- `session-store.svelte.ts` — tracks `contextWindow`, `rateLimitUtilization`, `rateLimitResetsAt`, token counts
- `parseContextMarkdown` utility — parses context info from session events

However, context usage events (`context_window` type) may not be consistently reported for all session types (especially provider-based sessions using the pipe-exec path).

### Design

Ensure all CC sessions emit context usage events that the frontend can display.

### Investigation Needed

1. Check if `context_window` events are emitted by the stream-session path (Claude native)
2. Check if `context_window` events are emitted by the pipe-exec path (provider-based)
3. If pipe-exec doesn't emit them, add event forwarding from the agent stream

### Approach

**If events are already emitted for all paths:**
- No backend changes needed
- Verify the UI components are rendered in the chat page for all session types
- Ensure the `ContextUsageGrid` and rate limit indicators are visible in the chat header/status area

**If events are missing for some paths:**
- In the stream/pipe-exec handlers, parse context usage from the agent's output stream
- Emit `context_window` events to the frontend via the existing event system
- The session store already handles these events — just need to ensure they arrive

### Modified Files (likely)

- `src-tauri/src/agent/stream.rs` — Ensure context events are forwarded for all agent types
- `src/routes/chat/+page.svelte` — Ensure `ContextUsageGrid` is rendered and visible
- `src/lib/stores/session-store.svelte.ts` — Verify event handling is complete

### UI Placement

The context usage indicator should be visible in the chat page header area (near the session name/model display), not hidden in a sub-panel. Consider a compact horizontal bar showing:
- Context % fill (with color: green < 50%, amber 50-80%, red > 80%)
- Rate limit utilization (if available)
- Token count summary

---

## Implementation Order

Recommended sequence based on dependencies and risk:

1. **Gemini Removal** (Item 1) — Clean foundation, no dependencies
2. **Prompt Constraint** (Item 5) — Trivial, single-file change
3. **@Name SingleTarget** (Item 3) — Small scope, backend + frontend
4. **Room Session Grouping** (Item 4) — Frontend-only, no backend schema change
5. **Context Usage HUD** (Item 6) — Needs investigation first
6. **Stepper Mini-Map** (Item 2) — Largest scope, new component + Tauri command

Items 1 and 5 can be done first as a quick win. Items 3 and 4 are independent of each other. Item 6 depends on investigation findings. Item 2 is the most complex and should be last.

---

## Testing Strategy

- **Gemini Removal:** Run `npm run check`, `npm test`, `cargo check`, `cargo test` — all should pass with zero Gemini references
- **Stepper:** Test with a multi-turn room, verify snapshot loads correct data, banner appears, exit works
- **SingleTarget:** Test `@Alice hello` produces a public turn with only Alice responding
- **Room Grouping:** Create a room, verify runs appear in "Rooms" folder, not in cwd folder
- **Prompt Constraint:** Verify generated prompts include the constraint text
- **Context HUD:** Test with native Claude session and provider-based session, verify context % displays for both

---

## Open Questions

1. **Stepper snapshot data:** Should we load full event data on click (lazy), or pre-load all turn snapshots on room select (eager)? Lazy is better for performance.
2. **Room grouping performance:** Loading all room details for sidebar could be slow with many rooms. Consider caching or a dedicated `list_room_run_ids` command that returns just the mapping.
3. **Context HUD for pipe-exec:** Need to verify whether Claude Code's stream-json format includes context_window events for non-native sessions.
