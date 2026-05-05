# Phase 3 Roundtable Implementation Plan

**Feature:** Phase 3 - General Roundtable
**Status:** Done. Merged to `master` in `b882c12 merge: phase 3 roundtable`.
**Goal:** Add native Room roundtable orchestration above existing Run / SessionActor primitives.
**Acceptance Criteria:**
- `Room.kind = Roundtable`.
- Plain input fanouts to active peer participants.
- `@debate` builds prompts from previous public turn responses and excludes the target participant's own previous answer.
- `@summary @who` routes a full public room-history summary task to exactly one participant.
- `@who text` creates a private turn and writes `rooms/{id}/private.json`.
- Private messages do not appear in the public room timeline.
- Room turn completion is based on `AgentAdapter` outcomes, not PTY buffer scanning.
**Architecture:** Keep `Run` as the canonical execution unit and add a room-level orchestration layer that talks to active participant actors through `AgentAdapter`. Public room history is stored as response references and previews, not full assistant output. Private turns use a separate private store.
**Tech Stack:** Rust/Tauri backend, existing `SessionActor` mailbox, room storage JSON/JSONL, Svelte 5 room UI, Vitest/Rust unit tests.
**Frontend Verification:** Yes. `/rooms` must expose a roundtable input/timeline surface and preserve `/chat` behavior.

---

## Finish Line

Phase 3 is complete when a Roundtable room can send one prompt to multiple active Claude participants, run debate and summary commands, store public turn references in `rooms/{id}/timeline.jsonl`, store private turns in `rooms/{id}/private.json`, and render the resulting public timeline in `/rooms`.

Out of scope:

- Driver/Copilot review flow.
- Research room artifacts.
- Gemini/Codex interactive participant support beyond capability records.
- Full assistant transcript duplication into room storage.
- PTY transcript scanning.

## Terminal Schema

Rust additions:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoomKind {
    Roundtable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoomTurnMode {
    Fanout,
    Debate,
    Summary,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoomTurn {
    pub id: String,
    pub idx: u64,
    pub mode: RoomTurnMode,
    pub user_input: String,
    pub target_participant_ids: Vec<String>,
    pub responses: Vec<RoomResponseRef>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoomResponseRef {
    pub participant_id: String,
    pub run_id: String,
    pub event_seq_start: u64,
    pub event_seq_end: u64,
    pub preview: Option<String>,
    pub status: String,
    pub error: Option<String>,
}
```

TypeScript mirrors these types and adds `turns: RoomTurn[]` to `RoomDetail`.

## Task 1: Add Roundtable Storage Model

Status: Done in `5af586a feat: add roundtable room storage`.

**Files:**
- Modify: `src-tauri/src/room/models.rs`
- Modify: `src-tauri/src/storage/rooms.rs`
- Modify: `src/lib/types.ts`

**Steps:**
1. Write failing storage tests for room kind default, public timeline append/list, private turn append/list, and legacy room JSON deserialization.
2. Add `RoomKind`, `RoomTurnMode`, `RoomTurn`, and `RoomResponseRef`.
3. Persist public turns under `rooms/{id}/timeline.jsonl`.
4. Persist private turns under `rooms/{id}/private.json`.
5. Ensure `get_room` remains compatible with Phase 2 room JSON.

**Verify:**

```powershell
cargo test rooms --manifest-path src-tauri/Cargo.toml
```

## Task 2: Wire Active Actor Messaging into AgentAdapter

Status: Done in `10863bf feat: stream room messages through adapter`.

**Files:**
- Modify: `src-tauri/src/room/adapter.rs`
- Modify if needed: `src-tauri/src/commands/session.rs`

**Steps:**
1. Write failing adapter tests using a mock `ActorCommand` receiver or a mock adapter.
2. Add an actor-backed constructor that can stream a message through an active `SessionActor` mailbox.
3. Keep `wait_turn_complete` polling run metadata.
4. Report `can_stream_message = true` only when the participant has an active actor sender.

**Verify:**

```powershell
cargo test adapter --manifest-path src-tauri/Cargo.toml
```

## Task 3: Add Roundtable Orchestrator

Status: Done in `9336b58 feat: add roundtable orchestrator`.

**Files:**
- Create: `src-tauri/src/room/orchestrator.rs`
- Modify: `src-tauri/src/room/mod.rs`
- Modify: `src-tauri/src/commands/rooms.rs`

**Steps:**
1. Write failing pure tests for command parsing:
   - plain text -> fanout
   - `@debate text` -> debate
   - `@summary @who` -> summary target
   - `@who text` -> private target
2. Write failing prompt-builder tests:
   - debate prompt excludes the target participant's previous response
   - summary prompt includes public room history
3. Implement target selection by participant id, run id, or label.
4. Implement fanout:
   - resolve active participant adapters
   - send the same prompt to all active peers
   - wait through adapter outcomes
   - store response references/previews in public timeline
5. Implement debate, summary, and private routing.
6. Return updated `RoomDetail`.

**Verify:**

```powershell
cargo test roundtable --manifest-path src-tauri/Cargo.toml
cargo test rooms --manifest-path src-tauri/Cargo.toml
```

## Task 4: Expose Frontend API and Store

Status: Done in `2f97eaf feat: add roundtable room UI`.

**Files:**
- Modify: `src/lib/api.ts`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/stores/room-store.svelte.ts`
- Modify: `src/lib/stores/room-store.test.ts`

**Steps:**
1. Add `sendRoomMessage(roomId, message)`.
2. Add roundtable types.
3. Add store method `sendMessage(message)`.
4. Test that store updates selected room with returned timeline.

**Verify:**

```powershell
npm run lint
npm test -- src/lib/stores/room-store.test.ts
```

## Task 5: Add Roundtable UI Surface

Status: Done in `2f97eaf feat: add roundtable room UI`.

**Files:**
- Modify: `src/routes/rooms/+page.svelte`
- Modify: `messages/en.json`
- Modify: `messages/zh-CN.json`

**Steps:**
1. Add a public timeline panel showing mode, input, targets, response status, and preview.
2. Add a roundtable input at the bottom of the room workspace.
3. Support plain text, `@debate`, `@summary @who`, and `@who text` without visible instructional copy.
4. Keep participant cards and memo usable.

**Verify:**

```powershell
npm run lint
npm run build
```

## Phase Gate

Targeted gate before external review:

```powershell
cargo test roundtable rooms adapter --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings
npm run lint
npm run i18n:check
npm run build
npm test -- src/lib/stores/room-store.test.ts
```

Known baseline remains separate: full `npm run check`, full `npm run test`, and full Rust test suite may still expose pre-existing diagnostics documented in `docs/implementation-roadmap.md`.

## Verification Evidence

Last verified on 2026-04-30 in `D:\ClaudeWorkspace\Code\claude-session-hub-remaster-phase3-roundtable`.

- `npm test -- src/lib/stores/room-store.test.ts`: pass, 7 tests.
- `npm run lint`: pass.
- `npm run i18n:check`: pass with the existing 10 zh-CN untranslated warnings.
- `npm run build`: pass with existing Svelte a11y/chunk warnings.
- `npx prettier --check messages/en.json messages/zh-CN.json src/lib/api.ts src/lib/types.ts src/lib/stores/room-store.svelte.ts src/lib/stores/room-store.test.ts src/routes/rooms/+page.svelte`: pass.
- `cargo test room::orchestrator --manifest-path src-tauri/Cargo.toml`: pass, 7 tests.
- `cargo test rooms --manifest-path src-tauri/Cargo.toml`: pass, 11 tests.
- `cargo test adapter --manifest-path src-tauri/Cargo.toml`: pass, 19 tests.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`: pass.
- `git diff --check`: pass.

Known baseline not resolved by this phase:

- `npm run format:check` still fails on 183 pre-existing `src/` formatting mismatches outside the touched-file scope.
- `npm run check`, full `npm run test`, and full Rust test suite remain governed by the baseline notes in `docs/implementation-roadmap.md`.

## External Review Fixes

Applied on 2026-05-01 in `89ec6b8 fix: harden roundtable turn orchestration`.

Accepted as in-phase:

- Explicit `@summary @who` / `@who text` targets must be attached to an active actor; inactive explicit targets now return an error and do not write a fake turn.
- Fanout/debate now start all participant tasks before waiting for turn completion, so one participant's completion does not block delivery to the next participant.
- Same-room orchestration is serialized with a per-room async lock to avoid duplicate turn indexes from concurrent `send_room_message` calls.
- Public/private turn append updates `room.updated_at`, keeping room list activity order meaningful.
- `RoomStore.sendMessage` ignores stale responses if the selected room changed while the send was in flight.
- `/rooms` styles `complete` / `completed` response statuses as successful.

Deferred as later-phase or non-blocking:

- Full per-participant pending/progress UI is deferred beyond the Phase 3 MVP.
- Multi-CLI capability matrix expansion remains Phase 5.
- Deeper preview/full-context summary tuning remains a future quality improvement; Phase 3 keeps response refs/previews and does not duplicate full transcripts.
- Corrupt JSONL quarantine/reporting remains P3 follow-up; it is not required for the current Roundtable acceptance gate.

Review-fix verification on 2026-05-01:

- `cargo test room::orchestrator --manifest-path src-tauri/Cargo.toml`: pass, 10 tests.
- `cargo test rooms --manifest-path src-tauri/Cargo.toml`: pass, 11 tests.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`: pass.
- `npm test -- src/lib/stores/room-store.test.ts`: pass, 8 tests.
- `npm run lint`: pass.
- `npm run i18n:check`: pass with the existing 10 zh-CN untranslated warnings.
- `npm run build`: pass with existing Svelte a11y/chunk warnings.
- `npx prettier --check src/lib/stores/room-store.svelte.ts src/lib/stores/room-store.test.ts src/routes/rooms/+page.svelte`: pass.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`: pass.
- `git diff --check`: pass.

## Second-Pass Review Fix

Applied on 2026-05-01 in `b067d2b fix: serialize roundtable sends per run`.

Accepted as in-phase:

- A single active run can be attached to multiple rooms. Per-room serialization alone does not protect `RoomResponseRef.event_seq_start` / `event_seq_end` when two rooms send to the same run concurrently.
- Room-originated sends now also take a per-run async lock around `event_seq_start`, `stream_message`, `wait_turn_complete`, and `event_seq_end`. This preserves response range integrity for room-to-room shared-run concurrency while keeping fanout to different runs concurrent.

Still deferred:

- Full actor-level turn identity / exact event-span reporting remains a deeper adapter/actor protocol improvement. Phase 3 uses the per-run lock as the MVP correctness guard for room-originated sends.

Second-pass verification on 2026-05-01:

- `cargo test room::orchestrator::tests::shared_run_across_rooms_is_not_sent_concurrently --manifest-path src-tauri/Cargo.toml`: red before fix, pass after fix.
- `cargo test room::orchestrator --manifest-path src-tauri/Cargo.toml`: pass, 11 tests.
- `cargo test rooms --manifest-path src-tauri/Cargo.toml`: pass, 12 tests.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`: pass.
- `npm test -- src/lib/stores/room-store.test.ts`: pass, 8 tests.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`: pass.
- `npm run lint`: pass.
- `npm run i18n:check`: pass with the existing 10 zh-CN untranslated warnings.
- `npm run build`: pass with existing Svelte a11y/chunk warnings.
- `git diff --check`: pass.

## Post-Merge Cleanup

Applied on 2026-05-02 after the Phase 3 merge check:

- Roundtable command parsing now treats `@debate` and `@summary` as command words only when followed by whitespace or the end of input. Participant names such as `@debateAlice` and `@summaryBot` route as private targets instead of being captured by command-prefix parsing.
- `npm run doc:check` now points to an existing documentation sanity script.
- `README.zh-CN.md` is aligned with the remaster README and documents current Memo, Rooms/Roundtable, and Windows native toolchain support.
- Phase 3 touched frontend/message files were reformatted with Prettier after the earlier verification record drifted.

Post-merge cleanup verification on 2026-05-02:

- `cargo test room::orchestrator::tests::command_words_do_not_capture_similarly_named_private_targets --manifest-path src-tauri/Cargo.toml`: red before fix, pass after fix.
- Full command list is recorded in the final review notes for this cleanup pass.

## Merge Result

Merged on 2026-05-01:

- Merge commit: `b882c12 merge: phase 3 roundtable`.
- User-facing README updated after merge to describe Roundtable fanout, debate, summary, and private turns.
- Remaining follow-ups stay outside Phase 3: Driver/Copilot, Research Room, Arena Memory, and the full multi-CLI capability matrix.
