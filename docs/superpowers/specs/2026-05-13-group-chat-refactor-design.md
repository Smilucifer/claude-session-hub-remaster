# Group Chat Refactor Design Spec

**Date:** 2026-05-13
**Status:** Draft
**Phase:** 10 (post-9.z)

## Problem Statement

The existing Room (圆桌会议室) system is a self-contained feature with its own page (`/rooms`), store (`room-store`), and concept model (RoomKind, RoomTurnMode, seat memories, research artifacts). From the user's perspective, a multi-participant conversation ("group chat") and a single-participant conversation ("chat") are the same thing — both are "chats." Having two separate pages violates this mental model.

Additionally, the Room system carries unused complexity:
- Three RoomKind variants (Roundtable/Driver/Research) where only Roundtable is commonly used
- Seven RoomTurnMode variants with overlapping semantics
- Seat memory system (insight/lesson/preference/fact + recall counting + expiry) that is rarely used
- Research artifacts that only serve one mode

## Goals

1. **Eliminate the Room concept** — rename to GroupChat across the entire codebase (Rust, Tauri commands, frontend store, routes, storage)
2. **Unify chat experience** — group chat lives in `/chat` page alongside single chat, not in a separate `/rooms` page
3. **Add Plan mechanism** — lightweight PlanArtifact attached to GroupChat for planner→executor workflow
4. **Introduce flexible roles** — free-form role string + optional role_type tag for routing hints
5. **Simplify sidebar** — "对话" and "群聊" grouped sections with "新对话" and "新群聊" entry points

## Non-Goals

- A2A auto-chaining (defer — user-driven dispatch is correct for desktop app)
- Whisper/visibility model (defer — existing `/dm` mechanism sufficient)
- Intent routing with #tags (rejected — explicit UI commands are richer)
- Message queue with delivery modes (rejected — turn-based model sufficient)
- Backward compatibility with old Room data (clean break — no migration script)

## Data Model Changes

### GroupChat (replaces Room)

```rust
// src-tauri/src/group_chat/models.rs (renamed from room/models.rs)

pub struct GroupChat {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub cwd: Option<String>,
    pub participants: Vec<GroupChatParticipant>,
    pub active_plan: Option<PlanArtifact>,    // NEW
    pub auto_chain: bool,                      // NEW (default false)
    pub memo: Option<String>,
    pub seat_memories: HashMap<String, Vec<SeatMemoryEntry>>,
    pub created_at: String,
    pub updated_at: String,
}
```

### GroupChatParticipant (replaces RoomParticipant)

```rust
pub struct GroupChatParticipant {
    pub id: String,
    pub run_id: String,
    pub agent: String,
    pub label: String,
    pub role: String,                              // free-form (e.g. "planner", "executor", "reviewer")
    pub role_description: Option<String>,          // NEW: optional prompt prefix
    pub preferred_for: Option<Vec<String>>,        // NEW: routing hints (e.g. ["review", "architecture"])
    pub joined_at: String,
}
```

### PlanArtifact (new)

```rust
pub struct PlanArtifact {
    pub title: String,
    pub tasks: Vec<PlanTask>,
    pub status: PlanStatus,       // Draft | Active | Completed
    pub created_at: String,
    pub updated_at: String,
}

pub struct PlanTask {
    pub id: String,
    pub description: String,
    pub assignee_id: Option<String>,   // participant id, None = unassigned
    pub status: TaskStatus,            // Todo | InProgress | Done | Blocked
    pub depends_on: Vec<String>,       // task IDs
}

pub enum PlanStatus { Draft, Active, Completed }
pub enum TaskStatus { Todo, InProgress, Done, Blocked }
```

### TurnVisibility (new)

```rust
pub enum TurnVisibility {
    Public,    // default, visible to all
    Whisper,   // visible only to target + user (future use)
}
```

## Frontend Architecture

### Route Unification

- `/rooms` route **removed**
- `/chat` route handles both single chat and group chat
- Group chat is a "chat with multiple participants" — same page, same interface, additional UI overlays

### Sidebar (Plan B: Independent Grouped Areas)

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
- Group chat items show participant count badge
- Debate status shown as red tag
- Both types sorted by `updated_at` descending

### Group Chat UI Enhancements (over single chat)

When a group chat is active, the chat page shows additional overlays:

1. **Participant Panel** — collapsible side panel or header bar showing:
   - Participant list with role badges, provider/model info, status indicators
   - Add/Remove participant buttons
   - Role type selector (planner/executor/custom)

2. **Plan Panel** — when a plan exists:
   - Plan title and status badge
   - Task checklist with assignee and status
   - Approve / Execute buttons
   - User notes input for executor

3. **Composer Enhancements**:
   - @mention autocomplete for participant names
   - Default: broadcast to all participants (fanout)
   - `@name`: route to specific participant
   - `/dm @name`: private message
   - `@debate`: trigger debate among participants

4. **Stepper** — existing RoomStepper component reused for turn history

### Store Architecture

- `room-store.svelte.ts` → `group-chat-store.svelte.ts`
- Session store remains single-chat focused
- Group chat store manages:
  - Group chat CRUD
  - Participant management (add/remove/update role)
  - Plan management (create/approve/complete)
  - Message routing (fanout, @name, /dm, @debate)
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
| `add_seat_memory_entry` | `add_group_chat_seat_memory` |
| `delete_seat_memory_entry` | `delete_group_chat_seat_memory` |
| `clear_seat_memory` | `clear_group_chat_seat_memory` |

### New Commands

| Command | Purpose |
|---------|---------|
| `create_plan` | Create a PlanArtifact on a GroupChat |
| `update_plan` | Update plan tasks/status |
| `approve_plan` | Set plan status to Active |
| `complete_plan` | Set plan status to Completed |

### Orchestrator Refactor

- `src-tauri/src/room/` → `src-tauri/src/group_chat/`
- `orchestrator.rs` logic preserved, entry points renamed:
  - `run_roundtable_turn` → `run_group_chat_turn`
  - `run_driver_turn` → **removed** (Driver mode eliminated)
  - `run_research_turn` → **removed** (Research mode eliminated)
- `parse_roundtable_command` → `parse_group_chat_command` (same parsing logic)
- Plan-aware prompt building: when executor is triggered, inject plan tasks + user notes into prompt via instruction_file

### Storage Migration

- `~/.claw-go/rooms/` → `~/.claw-go/group-chats/`
- Directory structure preserved: `group-chats/{id}/group_chat.json`, `timeline.jsonl`, `private.json`
- `room.json` → `group_chat.json`
- Old Room data: no migration script, no backward compatibility. Clean break.

### Plan Injection to Executor

When user triggers an executor via `@executor-name`:

1. Orchestrator reads `active_plan` from GroupChat
2. Builds plan context: task list, status, user notes
3. Writes plan context to temp file (`plan-context-{run_id}.md`)
4. Executor session launched with `--instruction-file plan-context-{run_id}.md`
5. Executor receives plan as structured instruction, executes accordingly

## Routing and Message Flow

### Default Behavior (no @mention)

Message → broadcast to all participants (fanout). Each participant responds independently.

### @name Routing

`@Alice review this code` → SingleTarget to Alice only. Public turn.

### /dm Routing

`/dm @Alice this is private` → Private turn to Alice. Written to `private.json`, not visible on public timeline.

### @debate

`@debate` → All participants see each other's previous round opinions and respond with counter-arguments.

### @executor Trigger

`@executor implement the plan` → Orchestrator:
1. Reads active_plan from GroupChat
2. Injects plan context into executor's session
3. Sends user message + plan as instruction
4. Executor responds with implementation

### Auto-chain (opt-in, default off)

When `auto_chain: true` on GroupChat:
- After a participant completes a turn, orchestrator scans response for `@other_participant` mentions
- If found, UI shows a confirmation prompt: "Alice 的回复提到了 @Bob — 路由到 Bob？"
- User confirms → triggers SingleTarget to Bob
- User ignores → no action

## Open Questions

1. **Existing Room data**: Are there any Room data that needs to be preserved? If yes, we need a migration script. If no, clean break.

2. **Driver/Research mode elimination**: These modes are being dropped. If any user relies on them, they lose functionality. Is this acceptable?

3. **Seat memory retention**: The spec keeps seat_memories on GroupChat (unused but preserved). Should we drop them entirely to simplify?

4. **Group chat creation flow**: "新群聊" button → what's the minimum viable creation dialog? Name + first participant? Or name + role selection + CWD?

5. **Plan creation timing**: Can a plan be created before any participants exist? Or only after at least one planner is added?

## Implementation Order

### Phase 10.a: Rename and Restructure (mechanical)

1. Rename `Room` → `GroupChat` in Rust models
2. Rename `room/` → `group_chat/` directory
3. Rename all Tauri commands
4. Rename `room-store` → `group-chat-store`
5. Rename `rooms/` storage → `group-chats/`
6. Update all frontend imports and API calls
7. Remove `/rooms` route
8. Remove Driver and Research orchestrator entry points
9. Verify: `cargo check`, `npm run check`, `npm run lint`

### Phase 10.b: UI Unification

1. Integrate group chat into `/chat` page
2. Add sidebar grouped sections (Plan B)
3. Add "新群聊" button and creation dialog
4. Add participant management panel
5. Add @mention autocomplete in composer
6. Verify: manual testing of single chat + group chat in same page

### Phase 10.c: Plan Mechanism

1. Add `PlanArtifact` and `PlanTask` types
2. Add `active_plan` field to GroupChat
3. Implement plan CRUD commands
4. Add Plan panel UI
5. Implement plan injection to executor via instruction_file
6. Verify: create plan → approve → trigger executor → executor receives plan

### Phase 10.d: Role and Routing Enhancements

1. Add `role_description` and `preferred_for` to GroupChatParticipant
2. Add `auto_chain` to GroupChat
3. Implement auto-chain detection with UI confirmation
4. Add `TurnVisibility` enum (future-proofing, no UI change yet)
5. Verify: role descriptions appear in participant panel, auto-chain works when enabled
