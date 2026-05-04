# Phase 4 Driver / Copilot Implementation Plan

**Feature:** Phase 4 - Driver / Copilot Room
**Status:** MVP implemented locally on 2026-05-02.
**Goal:** Add a Driver room mode where one driver can request read-only review from one or more copilot participants.
**Acceptance Criteria:**
- `Room.kind = Driver`.
- One participant has `Driver` role.
- Copilots default to read-only review behavior.
- Review request is first implemented as internal command or slash command.
- `.arena/context.md`, `.arena/state.md`, and `.arena/memory` are generated from room/run references.
- Driver can request review from one or more copilots.
- Review prompt includes recent context and stable run references.
- Copilot outputs are recorded as room review turns.
- Dangerous-operation review is documented but not required for MVP.
**Architecture:** Keep Run as the execution unit. Driver rooms reuse the Phase 3 room orchestrator path but dispatch only to active copilot participants, record `review` turns in the public timeline, and generate room-local Arena files before sending the review prompt.
**Tech Stack:** Rust/Tauri backend, existing `SessionActor` mailbox, room storage JSON/JSONL, Svelte 5 room UI, Vitest/Rust unit tests.
**Frontend Verification:** Yes - `/rooms` exposes Driver room creation and review turn rendering.

---

## Finish Line

Phase 4 MVP is complete when a Driver room can be created, exactly one participant is normalized to `driver`, copilots receive `/review` prompts in read-only review mode, outputs are stored as public `review` turns, and `.arena/context.md`, `.arena/state.md`, and `.arena/memory` are generated under the room directory.

Out of scope:

- Dangerous-operation approval protocol.
- Hard permission-mode enforcement for copilot review runs.
- Research Room artifacts.
- Full Arena Memory promotion and fact extraction.
- Multi-CLI capability matrix expansion.

## Implementation Notes

- `RoomKind` now includes `driver`.
- `RoomTurnMode` now includes `review`.
- `create_room` accepts optional `kind`; missing kind remains `roundtable`.
- Driver room role rules:
  - First participant defaults to `driver`.
  - Later participants default to `copilot`.
  - Assigning a new `driver` demotes the previous driver to `copilot`.
- Driver command syntax:
  - `/review request` sends to all active copilots.
  - `/review @name request` sends to one or more named active copilots.
  - Plain Driver room input is rejected; review dispatch must be explicit.
- Review prompts include:
  - Read-only behavior instruction.
  - Current request.
  - Room cwd and `.arena` path.
  - Stable participant/run references.
  - Room memo.
  - Recent public timeline previews.
- `.arena` files are local runtime context mirrors. They can contain stable run references, memo text, and recent public previews; they are not designed as shareable review reports.

## Verification Evidence

Recorded during implementation:

```powershell
cargo test room::orchestrator::tests::parses_driver_review_commands --manifest-path src-tauri/Cargo.toml
cargo test storage::rooms::tests::driver_room_enforces_single_driver_role --manifest-path src-tauri/Cargo.toml
npm test -- src/lib/stores/room-store.test.ts
cargo test room::orchestrator --manifest-path src-tauri/Cargo.toml
cargo test rooms --manifest-path src-tauri/Cargo.toml
```

Remaining gate before merge:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings
npm run lint
npm run i18n:check
npm run build
git diff --check
```
