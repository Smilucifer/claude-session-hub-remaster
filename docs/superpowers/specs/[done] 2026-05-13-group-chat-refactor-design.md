# Group Chat Refactor Design Spec

**Date:** 2026-05-13
**Status:** ✅ Complete (v2.2.0) — All phases implemented including P1 context sharing, P2 env var optimization, P3 memory system
**Phase:** 10 (post-9.z)

## Problem Statement

The existing Room (圆桌会议室) system is a self-contained feature with its own page (`/rooms`), store (`room-store`), and concept model (RoomKind, RoomTurnMode, seat memories, research artifacts). From the user's perspective, a multi-participant conversation ("group chat") and a single-participant conversation ("chat") are the same thing — both are "chats." Having two separate pages violates this mental model.

Additionally, the Room system carries unused complexity:
- Three RoomKind variants (Roundtable/Driver/Research) where only Roundtable is commonly used
- Seven RoomTurnMode variants with overlapping semantics
- Seat memory system (insight/lesson/preference/fact + recall counting + expiry + inbox + profile + checkpoint) that is rarely used
- Research artifacts that only serve one mode

## Goals

1. **Eliminate the Room concept** — rename to GroupChat across the entire codebase (Rust, Tauri commands, frontend store, routes, storage)
2. **Unify chat experience** — group chat lives in `/chat` page alongside single chat, not in a separate `/rooms` page
3. **Add Plan mechanism** — lightweight PlanArtifact attached to GroupChat for planner→executor workflow
4. **Introduce flexible roles** — free-form role string + optional role_description for prompt enrichment
5. **Simplify sidebar** — "对话" and "群聊" grouped sections with "新对话" and "新群聊" entry points

## Non-Goals

- A2A auto-chaining (defer — user-driven dispatch is correct for desktop app)
- Whisper/visibility model (defer — existing `/dm` mechanism sufficient)
- Intent routing with #tags (rejected — explicit UI commands are richer)
- Message queue with delivery modes (rejected — turn-based model sufficient)
- Backward compatibility with old Room data (clean break — no migration script)
- Seat memory system (removed — rarely used, includes seat_memories, inbox, profile, checkpoint)
- Driver mode (removed — unused)
- Research mode (removed — unused)

## Decisions (formerly Open Questions)

| # | Question | Decision |
|---|----------|----------|
| 1 | Existing Room data preservation? | **Clean break.** No migration script. First launch: if `rooms/` directory exists, log info warning that old data is no longer loaded. User can manually delete. |
| 2 | Driver/Research mode elimination? | **Removed.** Delete `run_driver_turn`, `run_research_turn`, `DriverCommand`, `ResearchArtifact`, arena files, MCP bundle logic. |
| 3 | Seat memory retention? | **Deleted entirely.** Remove `seat_memories`, `seat_memory_inbox`, `SeatProfile`, `SeatMemoryEntry`, `PendingMemoryCandidate`, `MemoryKind` from models. Delete `memory.rs` module (~290 lines + 8 tests). |
| 4 | Group chat creation dialog config? | **Minimum: name + CWD.** Participants added after creation via "add participant" button. No fixed seat count requirement. |
| 5 | Plan creation timing? | **After at least one participant exists.** Plan needs assignees. |

## Data Model Changes

### GroupChat (replaces Room)

```rust
// src-tauri/src/group_chat/models.rs (renamed from room/models.rs)

pub struct GroupChat {
    pub id: String,
    pub name: String,
    pub description: String,                    // non-optional (matches existing Room)
    pub cwd: Option<String>,
    pub participants: Vec<GroupChatParticipant>,
    pub active_plan_id: Option<String>,         // references PlanArtifact by ID (not embedded)
    pub auto_chain: bool,                       // default false, Phase 10.d
    pub memo: String,                           // non-optional (matches existing Room)
    pub created_at: String,
    pub updated_at: String,
}
```

**Removed fields** (vs existing Room): `kind` (RoomKind), `seat_memories`, `seat_memory_inbox`, `seat_profile`, `last_checkpoint_turn`, `last_checkpoint_at`.

### GroupChatParticipant (replaces RoomParticipant)

```rust
pub struct GroupChatParticipant {
    pub id: String,
    pub run_id: String,
    pub agent: String,
    pub label: String,
    pub role: String,                              // free-form (e.g. "planner", "executor", "reviewer")
    pub role_description: Option<String>,          // optional prompt prefix, Phase 10.d
    pub preferred_for: Option<Vec<String>>,        // UI hint only (e.g. ["review", "architecture"]), Phase 10.d
    pub joined_at: String,
}
```

### GroupChatTurnMode (replaces RoomTurnMode)

```rust
pub enum GroupChatTurnMode {
    Fanout,         // broadcast to all participants (default)
    Debate,         // participants see each other's previous opinions
    Summary,        // @summary @Name — one participant synthesizes discussion
    Private,        // /dm @Name — private turn, not on public timeline
    SingleTarget,   // @Name — public turn to one participant
}
```

**Removed variants** (vs existing RoomTurnMode): `Review` (Driver-only), `Research` (Research-only).

### GroupChatCommand (replaces RoundtableCommand)

```rust
pub enum GroupChatCommand {
    Fanout(String),                                    // default: message to all
    Debate(String),                                    // @debate
    Summary { target: String, message: String },       // @summary @Name
    Private { target: String, message: String },       // /dm @Name
    SingleTarget { target: String, message: String },  // @Name
}
```

### PlanArtifact (new, stored separately)

```rust
// Stored at: group-chats/{group_chat_id}/plan.json

pub struct PlanArtifact {
    pub id: String,
    pub group_chat_id: String,
    pub title: String,
    pub tasks: Vec<PlanTask>,
    pub status: PlanStatus,       // Draft | Active | Completed
    pub user_notes: Option<String>,  // supplementary instructions for executor
    pub created_at: String,
    pub updated_at: String,
}

pub struct PlanTask {
    pub id: String,
    pub description: String,
    pub assignee_id: Option<String>,   // participant id, None = unassigned
    pub status: TaskStatus,            // Todo | InProgress | Done | Blocked
}

pub enum PlanStatus { Draft, Active, Completed }
pub enum TaskStatus { Todo, InProgress, Done, Blocked }
```

**Design note**: PlanArtifact is stored as a separate file (`plan.json`), not embedded in GroupChat. GroupChat only holds `active_plan_id: Option<String>` as a reference. This follows the existing Run/Room separation pattern and allows independent Plan lifecycle management.

**Removed**: `PlanTask.depends_on` (no implementation plan for task dependency graph in Phase 10).

## Frontend Architecture

### Route Unification

- `/rooms` route **removed**
- `/chat` route handles both single chat and group chat
- Navigation model: clicking a group chat in sidebar loads it in the same `/chat` page. The page detects `is_group_chat` and renders the multi-participant layout (participant panes + stepper + composer) instead of the single-chat layout. No sub-routes needed.

### Sidebar (Grouped Sections)

```
┌─────────────────────────┐
│ 🔍 搜索对话...           │
│ + 新对话  👥 新群聊      │
├─────────────────────────┤
│ ▼ 群聊 (2)               │
│   👥 API 重构讨论         │
│   👥 性能优化  [debate]   │
├─────────────────────────┤
│ ▼ 对话 (3)               │
│   💬 Fix login bug       │
│   💬 Explain Rust...     │
│   💬 Write unit tests... │
└─────────────────────────┘
```

- Group chats and conversations in separate collapsible sections
- Grouping logic: `participant_count > 1` determines "群聊" vs "对话". No `RoomKind` or `is_group_chat` flag needed.
- Group chat items show participant count badge
- Debate status shown as red tag
- Both types sorted by `updated_at` descending
- Current `__rooms__` virtual folder in `sidebar-groups.ts` is replaced by this new grouping mechanism

### Group Chat UI Enhancements (over single chat)

When a group chat is active, the chat page shows additional overlays:

1. **Participant Panel** — collapsible side panel or header bar showing:
   - Participant list with role badges, provider/model info, status indicators
   - Add/Remove participant buttons
   - Role input (free-form text)

2. **Plan Panel** — when a plan exists:
   - Plan title and status badge
   - Task checklist with assignee and status
   - Approve / Execute buttons
   - User notes input for executor

3. **Composer Enhancements**:
   - @mention autocomplete for participant names
   - Default: broadcast to all participants (fanout)
   - `@name`: route to specific participant (SingleTarget)
   - `/dm @name`: private message
   - `@debate`: trigger debate among participants
   - `@summary @name`: request summary from specific participant

4. **Stepper** — existing RoomStepper component reused for turn history

### Store Architecture

- `room-store.svelte.ts` → `group-chat-store.svelte.ts`
- Session store remains single-chat focused
- Group chat store manages:
  - Group chat CRUD
  - Participant management (add/remove/update role)
  - Plan management (create/approve/complete)
  - Message routing (fanout, @name, /dm, @debate, @summary)
  - Turn polling and incremental response display

## Backend Changes

### Tauri Command Renames

| Old Command | New Command |
|-------------|-------------|
| `create_room` | `create_group_chat` |
| `list_rooms` | `list_group_chats` |
| `get_room` | `get_group_chat` |
| `delete_room` | `delete_group_chat` |
| `attach_room_run` | `attach_group_chat_run` |
| `create_room_participant` | `create_group_chat_participant` |
| `create_room_claude_participant` | `create_group_chat_claude_participant` |
| `send_room_message` | `send_group_chat_message` |
| `cancel_room_turn` | `cancel_group_chat_turn` |
| `update_room_memo` | `update_group_chat_memo` |
| `list_room_run_index` | `list_group_chat_run_index` |
| `get_room_turn_snapshot` | `get_group_chat_turn_snapshot` |
| `add_seat_memory_entry` | **removed** (seat memory deleted) |
| `delete_seat_memory_entry` | **removed** |
| `clear_seat_memory` | **removed** |

### New Commands

| Command | Purpose |
|---------|---------|
| `create_plan` | Create a PlanArtifact on a GroupChat |
| `update_plan` | Update plan tasks/status |
| `approve_plan` | Set plan status to Active |
| `complete_plan` | Set plan status to Completed |

### Orchestrator Refactor

- `src-tauri/src/room/` → `src-tauri/src/group_chat/`
- `orchestrator.rs` changes:
  - `run_roundtable_turn` → `run_group_chat_turn`
  - `run_driver_turn` → **deleted** (Driver mode eliminated)
  - `run_research_turn` → **deleted** (Research mode eliminated)
  - `DriverCommand` enum → **deleted**
  - `build_driver_review_prompt`, `build_research_prompt` → **deleted**
  - `write_driver_arena_files`, `write_driver_mcp_bundle` → **deleted**
  - `parse_roundtable_command` → `parse_group_chat_command` (same parsing logic)
  - `RoundtableCommand` → `GroupChatCommand`
  - `build_summary_prompt` → **preserved** (Summary mode retained)
  - `build_debate_prompt` → **preserved**
  - `build_fanout_prompt` → **preserved**
  - `build_singletarget_prompt` → **preserved**

- `adapter.rs` changes:
  - File kept as `adapter.rs` (it describes agent adapter, not room concept)
  - `PromptScope::Room` → `PromptScope::GroupChat`
  - All other content preserved (AgentAdapter trait, RunBackedAgentAdapter, AgentCapabilities)

- `memory.rs` → **deleted entirely** (~290 lines + 8 tests, seat memory system removed)

- `models.rs` changes:
  - `Room` → `GroupChat`
  - `RoomParticipant` → `GroupChatParticipant`
  - `RoomTurn` → `GroupChatTurn`
  - `RoomTurnMode` → `GroupChatTurnMode`
  - `RoomResponseRef` → `GroupChatResponseRef`
  - `RoomSummary` → `GroupChatSummary`
  - `RoomKind` → **deleted**
  - `SeatMemoryEntry`, `PendingMemoryCandidate`, `SeatProfile`, `MemoryKind` → **deleted**

### Storage

- `~/.claw-go/rooms/` → `~/.claw-go/group-chats/`
- Directory structure: `group-chats/{id}/group_chat.json`, `timeline.jsonl`, `private.json`, `plan.json`
- `room.json` → `group_chat.json`
- Old `rooms/` directory: left as-is. First launch detects its presence and logs info warning.
- `ResearchArtifact` storage functions → **deleted**
- `DriverMcpBundle`, arena files → **deleted**

### Plan Injection to Executor

When user triggers an executor via `@Alice` (where Alice's role is "executor"):

1. Orchestrator reads `active_plan_id` from GroupChat, loads PlanArtifact from `plan.json`
2. Builds plan context string: task list with status, assignee info, user notes
3. **Mechanism**: plan context is prepended to the user's message as an instruction prefix. The combined message (plan context + user message) is sent to the executor's existing session via `stream_message`. This is option (a) — inject plan as part of the user message to an existing session.
4. Rationale: `--instruction-file` is a CLI launch parameter, not a runtime injection. The executor's session is already running. Prepending plan context to the user message is the simplest approach and consistent with how the orchestrator currently builds prompts.
5. The plan context is formatted as a structured markdown block with clear boundaries (e.g. `--- PLAN CONTEXT ---` ... `--- END PLAN ---`) so the executor can parse it.

## Routing and Message Flow

### Default Behavior (no @mention)

Message → broadcast to all participants (fanout). Each participant responds independently.

### @name Routing

`@Alice review this code` → SingleTarget to Alice only. Public turn. Matches by `label` (participant name), not by `role`.

### /dm Routing

`/dm @Alice this is private` → Private turn to Alice. Written to `private.json`, not visible on public timeline.

### @debate

`@debate` → All participants see each other's previous round opinions and respond with counter-arguments.

### @summary

`@summary @Alice` → Alice synthesizes the full discussion into a final summary. Preserved from existing Room system.

### @executor Trigger

`@Alice implement the plan` → SingleTarget to Alice. If a PlanArtifact exists and is Active, plan context is injected into the message (see Plan Injection section above). If no plan exists, works as normal SingleTarget.

**Routing clarification**: `@executor` is NOT a special keyword. It means "@Alice" where Alice happens to have role "executor". All @mention routing is by participant label, not by role.

### Auto-chain (Phase 10.d, opt-in, default off)

When `auto_chain: true` on GroupChat:
- After a SingleTarget turn completes, orchestrator scans response for `@other_participant` mentions
- **Fanout turns are excluded** — only SingleTarget responses are scanned (avoids N parallel scans and conflicting suggestions)
- If found, UI shows an inline banner (not modal): "Alice 的回复提到了 @Bob — 路由到 Bob？"
- User confirms → triggers SingleTarget to Bob
- User ignores → no action
- Confirmation has no timeout — banner stays until user acts or dismisses

## Implementation Order

### Phase 10.a: Rename, Restructure, and Clean Up

This phase combines mechanical renaming with Driver/Research/seat-memory deletion. They must be atomic because removing `RoomKind` breaks kind-dependent dispatch code.

**Step 1: Rust model changes**
- Rename `Room` → `GroupChat`, `RoomParticipant` → `GroupChatParticipant`, `RoomTurn` → `GroupChatTurn`, `RoomTurnMode` → `GroupChatTurnMode`, `RoomResponseRef` → `GroupChatResponseRef`, `RoomSummary` → `GroupChatSummary`
- Delete `RoomKind` enum
- Delete `SeatMemoryEntry`, `PendingMemoryCandidate`, `SeatProfile`, `MemoryKind`
- Remove `seat_memories`, `seat_memory_inbox`, `seat_profile`, `last_checkpoint_turn`, `last_checkpoint_at` fields from GroupChat struct
- Verify: `cargo check` passes

**Step 2: Delete Driver/Research/seat-memory code**
- Delete `orchestrator.rs`: `run_driver_turn`, `run_research_turn`, `DriverCommand`, `build_driver_review_prompt`, `build_research_prompt`, `write_driver_arena_files`, `write_driver_mcp_bundle`
- Delete `memory.rs` entirely
- Delete `storage/rooms.rs`: `driver_mcp_file`, `research_artifact_file`, `write_research_artifact`, seat memory functions
- Delete `models.rs`: `ResearchArtifact`, `ResearchResult`, `ArenaMemoryCandidate`, `DriverMcpBundle`
- Remove `send_room_message`'s kind-based dispatch
- Simplify `normalize_participant_role` (remove Driver/Research branches)
- Verify: `cargo check` passes

**Step 3: Rename directory and Tauri commands**
- `src-tauri/src/room/` → `src-tauri/src/group_chat/`
- Rename all 15 Tauri commands (see rename table above, minus 3 seat memory commands)
- Remove 3 seat memory Tauri commands
- Update `lib.rs` command registrations
- Verify: `cargo check` passes

**Step 4: Rename storage**
- `~/.claw-go/rooms/` → `~/.claw-go/group-chats/`
- `room.json` → `group_chat.json`
- Add first-launch check: if `rooms/` exists, log info warning
- Verify: `cargo check` passes

**Step 5: Frontend rename**
- `room-store.svelte.ts` → `group-chat-store.svelte.ts`
- Update all imports in `api.ts`, types, components
- Update `RoomRunMapping` → `GroupChatRunMapping`, `RoomRunIndexEntry` → `GroupChatRunIndexEntry`
- Add `participant_count: usize` to `GroupChatRunIndexEntry` (for sidebar grouping)
- Remove `/rooms` route
- Verify: `npm run check` passes

**Step 6: i18n updates**
- Rename `room_*` i18n keys to `groupChat_*` in `messages/en.json` and `messages/zh-CN.json`
- Add new keys: `groupChat.newGroupChat`, `groupChat.groupChats`, `groupChat.conversations`
- Verify: `npm run i18n:check` passes

**Step 7: Test updates**
- Update Rust tests in `storage/rooms.rs` and `commands/rooms.rs` (rename + remove seat memory tests)
- Update frontend tests in `room-store.test.ts` → `group-chat-store.test.ts`
- Verify: `cargo test` (via cargo check due to known VC++ issue), `npm run test` pass

**Step 8: Final verification**
- `npm run lint` passes
- `npm run check` passes
- `cargo check` passes
- `cargo clippy` passes
- `npm run i18n:check` passes

### Phase 10.b: UI Unification

1. Integrate group chat into `/chat` page (detect `is_group_chat`, render multi-participant layout)
2. Add sidebar grouped sections (对话/群聊 collapsible groups)
3. Add "新群聊" button and creation dialog (name + CWD only)
4. Add participant management panel (add/remove, role input, status display)
5. Add @mention autocomplete in composer
6. Add @summary to composer toolbar
7. Verify: create group chat → add 2 participants → fanout turn → both respond; single chat and group chat switch without state loss
8. i18n: add all new UI string keys

### Phase 10.c: Plan Mechanism

1. Add `PlanArtifact` and `PlanTask` types (stored in `plan.json`)
2. Add `active_plan_id` field to GroupChat
3. Implement plan CRUD commands (`create_plan`, `update_plan`, `approve_plan`, `complete_plan`)
4. Add Plan panel UI (task checklist, status badge, approve/execute buttons, user notes)
5. Implement plan injection to executor (prepend plan context to user message via `stream_message`)
6. Verify: create plan → add tasks → approve → trigger executor with @name → executor receives plan context

### Phase 10.d: Role and Routing Enhancements

1. Add `role_description` to GroupChatParticipant (prompt prefix injection)
2. Add `preferred_for` to GroupChatParticipant (UI hint only, no auto-routing)
3. Add `auto_chain` to GroupChat (opt-in, SingleTarget only, inline banner confirmation)
4. Verify: role descriptions appear in participant panel, auto-chain suggests routing on SingleTarget responses

## Appendix: File Impact Summary

### Files to delete
- `src-tauri/src/room/memory.rs` (~290 lines)
- `src/routes/rooms/+page.svelte` (replaced by /chat integration)

### Files to rename (directory move)
- `src-tauri/src/room/` → `src-tauri/src/group_chat/` (all files within)

### Files requiring significant edits
- `src-tauri/src/group_chat/models.rs` (struct renames + field removal)
- `src-tauri/src/group_chat/orchestrator.rs` (rename + Driver/Research deletion)
- `src-tauri/src/group_chat/storage.rs` (rename + seat memory/driver/research deletion)
- `src-tauri/src/commands/group_chat.rs` (rename + seat memory command deletion)
- `src-tauri/src/lib.rs` (command registration update)
- `src/lib/api.ts` (API function renames)
- `src/lib/types.ts` (type renames)
- `src/lib/stores/group-chat-store.svelte.ts` (store rename + seat memory removal)
- `src/lib/utils/sidebar-groups.ts` (grouping logic change)
- `messages/en.json`, `messages/zh-CN.json` (key renames)

### Files requiring minor edits
- `src/routes/+layout.svelte` (remove /rooms references)
- `src/lib/components/` (import path updates)
- Various test files (rename references)
