# ADR-001: Room as an Orchestration Layer over Run

**Date:** 2026-04-30
**Status:** Accepted
**Decision Owner:** Claw GO remaster planning
**Source:** `thinking.md`

Accepted on 2026-04-30 after Opus 4.7 review confirmation.

## Context

Claw GO already has a solid single-session execution model:

- `Run` is the persisted execution unit.
- `SessionActor` owns a Claude Code process lifecycle.
- `agent/turn_engine.rs` manages single-run turn sequencing, timeouts, internal jobs, permission prompts, hook callbacks, and result boundaries.
- `BusEvent` and `events.jsonl` are the canonical event stream for a run.

Claude Session Hub has several collaboration features that sit above a single CLI session:

- Meeting rooms with multiple sub-sessions.
- Roundtable fanout / debate / summary turns.
- Private side conversations.
- Driver / Copilot review flows.
- Research Mode.
- Arena Memory and shared project facts.

The core architecture question is whether to port Hub's meeting-room implementation directly, or model collaboration as a native Claw GO layer above existing runs.

## Decision

Claw GO will introduce **Room** as a first-class orchestration workspace above `Run`.

`Run` remains the smallest execution unit. A `Room` coordinates multiple runs through participants and collaboration turns.

```text
Room
├── Participant -> Run
├── RoomTurn -> one collaborative round
├── Memo -> room-local notes
└── Arena Memory -> project-level facts / decisions / lessons
```

Room orchestration must not replace `agent/turn_engine.rs`. Each participant's internal turn still runs through the existing single-run actor and turn engine. `RoomTurn` only coordinates cross-run behavior:

- fanout dispatch
- debate prompt construction
- summary target selection
- private-message routing
- result collection
- timeout policy
- room-level event emission

## Alternatives Considered

### Alternative A: Direct port of Hub `core/meeting-room.js`

Reuse the JavaScript meeting-room module behind a Tauri command shim.

Rejected because:

- Hub's meeting-room manages session lifecycle in JS, conflicting with Claw GO's Rust `SessionActor`.
- Forces Claw GO to keep two parallel session models (Rust actor + JS meeting room).
- Loses the ability to surface room state through `BusEvent` and the existing web server broadcast path.

### Alternative B: Extend `Run` to support multiple participants

Treat collaboration as a special run mode (`run.kind = "room"`).

Rejected because:

- `Run` is the persisted execution unit. Overloading it makes `events.jsonl` semantics ambiguous.
- Run deletion would imply room deletion, breaking the requirement that rooms outlive runs.
- Resume / fork / soft-delete logic on `Run` is already complex; multi-participant orchestration on top would compound it.

### Alternative C: Build collaboration entirely in the frontend

Keep the backend single-run; let Svelte stores manage rooms.

Rejected because:

- Web remote access (`web_server`) needs server-side room state to broadcast.
- Persistence and crash recovery would have to be reimplemented in the renderer.
- Multi-CLI orchestration (waiting on turn completion across runs) requires backend coordination.

### Alternative D: Defer collaboration entirely

Ship Memo only; postpone Room until usage data justifies it.

Rejected because:

- The strategic synthesis with Claude Session Hub depends on Room as the load-bearing concept.
- Memo without Room collapses to a fancier scratchpad and loses the differentiation argument for the remaster.

The chosen path (Room as orchestration above Run) is the only one that satisfies: native Claw GO domain model, reuse of existing Run / Actor / Storage / WebServer, and multi-CLI orchestration support.

## Target Model

```rust
Room {
  id,
  title,
  kind: Free | Roundtable | Driver | Research,
  project_cwd,
  participants: Vec<RoomParticipant>,
  active_round,
  created_at,
  updated_at,
}

RoomParticipant {
  id,
  run_id,
  agent: "claude" | "codex" | "gemini" | "...",
  role: Driver | Copilot | Peer,
  label,
  status,
  capabilities,
}

RoomTurn {
  idx,
  mode: Fanout | Debate | Summary | Private,
  user_input,
  targets,
  responses: Vec<RoomResponseRef>,
  started_at,
  completed_at,
}

RoomResponseRef {
  participant_id,
  run_id,
  event_seq_start,
  event_seq_end,
  preview,
  status,
}
```

`RoomTurn.responses` should store references and previews, not duplicate complete assistant output. The canonical source for full text remains the participant run's `events.jsonl`.

## Storage

```text
~/.claw-go/
├── runs/
├── memos/
│   └── global.json
├── projects/
│   └── {project-hash}/
│       └── memo.json
└── rooms/
    └── {room-id}/
        ├── meta.json
        ├── timeline.jsonl
        ├── private.json
        ├── memo.json
        ├── prompts/
        └── arena/
            ├── state.md
            ├── context.md
            └── memory/
```

Room deletion must not delete underlying runs unless the user explicitly chooses that destructive action.

## Events

Add room-level events to the existing event stream and WebSocket broadcast path:

- `RoomCreated`
- `RoomUpdated`
- `RoomParticipantAdded`
- `RoomTurnStarted`
- `RoomParticipantPartial`
- `RoomParticipantCompleted`
- `RoomTurnCompleted`
- `RoomPrivateMessage`
- `RoomMemoUpdated`
- `ArenaMemoryUpdated`

These events are for UI synchronization. They should not duplicate full run history.

**MUST NOT**: room-level events MUST NOT be appended to any participant run's `events.jsonl`. They belong to either:

- `rooms/{id}/timeline.jsonl` — canonical room history, separate from per-run event logs;
- in-memory broadcast over the existing event bus / WebSocket channels — for transient UI updates that do not need durable storage.

Mixing room events into a run's `events.jsonl` would break the canonical single-run semantics that history view, soft-delete, resume, fork, and replay all depend on. The run event log must remain a pure record of one CLI session.

## Agent Adapter Boundary

Room orchestration needs a stable participant abstraction before Roundtable can be implemented. Phase 2 must end with an adapter boundary similar to:

```rust
trait AgentAdapter {
    async fn wait_turn_complete(&mut self) -> Result<TurnOutcome>;
    async fn stream_message(&mut self, msg: &str) -> Result<()>;
    fn inject_prompt(&mut self, scope: PromptScope, body: &str) -> Result<()>;
    fn capabilities(&self) -> AgentCapabilities;
}
```

Implementation detail to decide during design: native async trait methods are not object-safe without an explicit pattern. Use one of:

- an enum-based adapter dispatch,
- associated future types,
- or `async_trait` if the dependency tradeoff is acceptable.

Do not let Room orchestration call CLI-specific parser or process code directly.

## UI Boundary

Create `/rooms` as a separate workspace. Do not merge it into `/teams`.

- `/teams` remains the read-only observer for Claude Code native teams.
- `/rooms` is Claw GO's own active multi-CLI collaboration workspace.

Default UI should be discussion-first, not a three-terminal wall:

- top member/status bar
- room timeline
- current turn cards
- private-message indicator
- memo and arena-memory panels
- raw terminal available only as an expanded detail view

## Non-Goals

- Do not port Electron or Hub's main process.
- Do not use node-pty as the default execution path.
- Do not make Room replace `Run`, `SessionActor`, or `turn_engine.rs`.
- Do not automatically import old Hub state into Claw GO.
- Do not make `/teams` and `/rooms` share one data model.

## Hub Data Migration Decision

Hub data will not be reverse-compatible with Claw GO Room storage.

Reasons:

- Hub state is spread across `~/.claude-session-hub/`, project `.arena/`, and prompt files.
- Hub schemas changed over time.
- Automatic migration creates long-term support cost disproportionate to the expected user base.

Accepted fallback:

- provide an export-only migration command later, e.g. `claw-go migrate-hub --export-only`;
- dump Hub rooms, memos, and arena memory into Markdown;
- let users manually paste important material back into the new workspace.

## Feature Flag Policy

Room, Memo, and AgentAdapter work must be isolated from the existing chat path.

Recommended policy:

- code may compile into the app by default;
- runtime settings flags control UI entry points and command availability;
- optional Cargo feature gating can be added later if build-size or release-channel pressure requires it;
- flag-off behavior must be verified through behavior and snapshot tests for existing chat/run flows.

Avoid claiming byte-for-byte identity. The practical acceptance criterion is unchanged behavior for existing chat usage.

## Consequences

Positive:

- Collaboration becomes a native Claw GO domain model.
- Existing run history, tool cards, permission UI, storage, and web server remain reusable.
- Roundtable, Driver/Copilot, Research, Memo, and Arena Memory can share one room foundation.

Costs:

- Requires new storage and event models.
- Requires a real AgentAdapter boundary before robust multi-CLI rooms.
- Adds UI surface area that must be kept separate from existing chat complexity.

## Follow-Up Documents

- `docs/migration-decisions.md`
- `docs/implementation-roadmap.md`
