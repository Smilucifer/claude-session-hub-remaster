# Claw GO Remaster Implementation Roadmap

**Date:** 2026-04-30
**Status:** Active
**Source:** `thinking.md`, `docs/ADR-001-room-over-run.md`

## Progress Snapshot

Last updated: 2026-05-05 after native Codex/Gemini chat parity, connection-profile UI parity, and Windows npm shim suppression work.

### Done

- Phase 1.a / 1.b Memo foundation merged in `216d500 feat: add phase 1 memo foundation`.
- ADR-001 accepted: Room is an orchestration layer over Run.
- Migration Pending pool cleared; future Hub files must be classified directly into Migrate / Reference / Drop.
- Phase 2 Room as Run Group merged in `ad8c159 merge: phase 2 rooms`.
- Phase 2 AgentAdapter boundary exists for run-backed participants with conservative Claude capabilities.
- Release package smoke test passed for the Phase 2 build: `ClawGO.exe`, MSI, and NSIS bundles were produced and the release exe rendered `/rooms`.
- Phase 2.x backend spawn environment resolver merged in `1d03536 merge: phase 2x msvc env` and pushed to `origin/master`: settings shape, conservative native-project detection, `vswhere` / `VsDevCmd.bat` derivation boundary, success cache, protected PATH-like merge, and integration for Claude session actor, Codex pipe-exec, `fork_session`, and side-question one-shot local spawns.
- `README.md` was updated in `949dfe5 docs: update user-facing readme` to describe user-visible functions only. Engineering process details remain in local `docs/` / `review/` / `thinking.md` and are intentionally not pushed to the remote repository.
- Temporary Phase 2.x worktree `D:\ClaudeWorkspace\Code\claude-session-hub-remaster-phase2x-msvc` and branch `feat/phase2x-msvc-env` were removed after merge.
- Phase 2.x settings/status UI and Windows validation merged to `master` in `merge: phase 2x msvc status ui`. This slice adds the Settings mode control, safe status summary command, spawn-updated status snapshot, ignored manual Windows validation tests, and fixes real `VsDevCmd.bat` invocation for paths under `Program Files (x86)`.
- `README.md` updated after the Phase 2.x status UI merge to describe the now-user-visible MSVC Settings/status surface.
- Phase 3 General Roundtable merged to `master` in `b882c12 merge: phase 3 roundtable`. This adds Roundtable room storage, actor-backed adapter streaming, fanout/debate/summary/private orchestration, `send_room_message`, and the `/rooms` timeline/input UI. Review fixes reject inactive explicit targets, dispatch fanout/debate participant tasks before waiting, serialize same-room orchestration to prevent duplicate turn indexes, serialize room-originated sends per run to protect response event ranges across rooms, update room activity timestamps on turn append, guard stale frontend send responses, and style completed responses.
- Phase 4 Driver / Copilot MVP merged in `72a7dab feat: add driver copilot rooms`. This adds Driver room creation, single-driver role normalization, copilot review routing through `/review`, read-only review prompts with stable run references, review turns in the public room timeline, and room-local `.arena/context.md`, `.arena/state.md`, and `.arena/memory` generation.
- Phase 4.5 Research Room MVP merged in `25dc4fe feat: Implement Phase 4.5 MVP for Research Room with structured artifacts`. This adds `Room.kind = Research`, research fanout prompts, `RoomTurnMode::Research`, room-local latest-snapshot `rooms/{id}/research/artifact.json`, Research UI labels/placeholders, and README semantics for latest artifact snapshot vs timeline history.
- Phase 1.d Workbench Polish merged in `103abe5 feat: add phase 1d workbench polish`. This adds last-message previews, unread counts, pinned conversation sorting/context-menu pinning, and a command-palette resume-last-active entry while keeping session orchestration untouched.
- Research Room follow-up merged in `a2ec04c feat: add research artifact memory candidates`. This adds schema-versioned research artifacts with append-only `rooms/{id}/research/artifacts.jsonl` history, Arena Memory candidates parsed from `[fact]`, `[decision]`, and `[lesson]` markers in participant results, latest artifact exposure through `RoomDetail`, and a compact `/rooms` Research artifact panel.
- Phase 5 Multi-CLI Capability Matrix merged in `9224281 feat: add phase 5 capability matrix`. This adds explicit Claude / Codex / Gemini capability records, exposes participant capabilities through `RoomDetail`, filters Room dispatch targets by `stream_session` capability, makes run execution-path defaults capability-driven, and updates `/rooms` attach filtering to use the shared frontend capability helper.
- Phase 5.5 Native CLI Chat Parity completed on `master` on 2026-05-05. Codex and Gemini pipe-exec runs now render in the normal chat timeline, live deltas are finalized as assistant messages, completed run replay preserves multi-turn `user -> assistant` order, Settings shows Codex/Gemini connection auth cards matching CC, and Windows npm `.cmd` shims are resolved to `node.exe + CLI js` to avoid transient command windows.

### Next

- Continue Phase 6.a Driver MCP Bundle.

### Pending

- Phase 6.b Driver MCP server wiring after the bundle schema stabilizes.
- Research MCP server port.

### Known Baseline

- `npm run check` currently fails on pre-existing Svelte/type diagnostics outside the Rooms work: 105 errors / 74 warnings. Rooms-related paths had no diagnostics during Phase 2 validation.
- On Windows, `cargo test --manifest-path src-tauri/Cargo.toml --lib` currently has 9 pre-existing path-normalization test failures on local `master` (`clipboard`, `files`, `history`, `community_skills`). Phase 2.x targeted tests pass; these baseline failures are outside the MSVC resolver touched paths.

## Finish Line

Claw GO becomes the primary architecture and UI shell. Claude Session Hub contributes product protocols: Memo, Room, Roundtable, Driver/Copilot, Research, multi-CLI participants, and Arena Memory.

## What We Are Not Building

- No Electron migration.
- No default node-pty multi-terminal wall.
- No automatic Hub state import.
- No rewrite of existing `/chat` before Room is feature-flagged and isolated.
- No merge of `/teams` and `/rooms`.

## Phase 1: Memo and Workbench Baseline

Goal: deliver visible value without touching session orchestration.

### Phase 1.a: Global Memo

Status: Done. Merged in `216d500 feat: add phase 1 memo foundation`.

Target files in Claw GO:

- `src-tauri/src/storage/memos.rs`
- `src-tauri/src/commands/memos.rs`
- `src-tauri/src/lib.rs`
- `src/lib/api.ts`
- `src/lib/types.ts`
- a small Svelte memo panel or command-palette entry

Scope:

- Store global memo at `~/.claw-go/memos/global.json`.
- Provide list / add / update / delete / clear commands.
- Frontend entry starts as a floating panel or command-palette action, not a permanent sidebar.

Acceptance:

- User can add memo text.
- User can edit and delete memo items.
- User can read memo text after restart.
- User can clear all global memo items.
- Existing chat behavior is unchanged.

### Phase 1.b: Project Memo

Status: Done. Merged in `216d500 feat: add phase 1 memo foundation`.

Scope:

- Store project memo at `~/.claw-go/projects/{project-hash}/memo.json`.
- Reuse Claw GO's existing project/cwd scoping.
- Switch memo content when the active project/cwd changes.

Acceptance:

- Global and project memo do not mix.
- Two projects keep separate project memos.
- Missing or deleted cwd degrades gracefully.

### Phase 1.c: Deferred

Status: Completed and merged through Phase 2.x.

Windows MSVC Environment Injection was intentionally deferred until after Phase 2 established Room-as-run-group and the AgentAdapter direction. The backend resolver and local spawn integration are merged in `1d03536`. The status/settings UI and Windows validation are merged in `merge: phase 2x msvc status ui`.

### Phase 1.d: Workbench Polish

Status: Done. Merged in `103abe5 feat: add phase 1d workbench polish`.

Pull a small set of "feels nice" enhancements from Hub without touching session orchestration. Strictly scoped — UI polish must not steal momentum from the Room main line.

Target files:

- `src/lib/components/SessionListItem.svelte` (or equivalent)
- `src/lib/stores/session-store.svelte.ts`
- localized message catalogs for any new copy

Scope:

- Last-message preview on the session list item.
- More visible unread indicator (badge / dot / count).
- Pinned session: pin / unpin via context menu, pinned items sort to top.
- "Resume last active session" entry in the command palette.

Acceptance:

- Restart and verify last-message preview survives.
- Unread count clears when the session is opened.
- Pinned state persists across restarts.
- No regression in existing session list rendering.

Out of scope: full session list redesign, multi-column workspaces, anything that touches the chat path.

## Phase 2: Room as Run Group

Goal: introduce Room storage and UI without full orchestration.

Status: Done. Merged in `ad8c159 merge: phase 2 rooms`.

Target files:

- `src-tauri/src/room/mod.rs`
- `src-tauri/src/room/models.rs`
- `src-tauri/src/storage/rooms.rs`
- `src-tauri/src/commands/rooms.rs`
- `src-tauri/src/lib.rs`
- `src/routes/rooms/+page.svelte`
- `src/lib/stores/room-store.svelte.ts`

Scope:

- Create a room.
- Add Claude participant by creating or attaching a run.
- Store `RoomParticipant.run_id`.
- Display room member state and recent run preview.
- Add room memo storage.

Acceptance:

- Room can be created and reopened.
- Room can reference an existing run without copying run state.
- Deleting room does not delete run.
- `/chat` continues to work with room flag off.

### Phase 2 Exit Requirement: AgentAdapter Boundary

Status: Done for Phase 2 scope.

Phase 2 added `src-tauri/src/room/adapter.rs` with a run-backed adapter and conservative capability reporting. It provides `wait_turn_complete` for existing run status polling. Streaming and prompt injection remain intentionally unsupported until orchestration requires them.

Before Phase 3 starts, add an adapter boundary for participant orchestration.

Design constraints:

- Room orchestration must not call CLI-specific parsers directly.
- Decide enum dispatch vs associated futures vs `async_trait`.
- `wait_turn_complete` must be available before Roundtable fanout/debate.

Sketch:

```rust
trait AgentAdapter {
    async fn wait_turn_complete(&mut self) -> Result<TurnOutcome>;
    async fn stream_message(&mut self, msg: &str) -> Result<()>;
    fn inject_prompt(&mut self, scope: PromptScope, body: &str) -> Result<()>;
    fn capabilities(&self) -> AgentCapabilities;
}
```

## Phase 2.x: Spawn Environment Resolver

Status: Done. Merged to `master`.

Phase 1.c introduces the Windows MSVC resolver for current local spawn paths. Phase 2.x should generalize that same spawn environment layer to new AgentAdapter / Room participants rather than creating a second resolver.

Scope:

- Reuse Phase 1.c's conservative native-project detection, MSVC cache, warning model, and protected env merge rules.
- Inject derived env into new Claude / Codex / Gemini child processes that are introduced through AgentAdapter / Room work.
- Keep the same user setting: `windows_msvc_env_mode: "auto" | "always" | "off"`.
- Do not reintroduce broad `Cargo.toml`-alone auto detection.

Acceptance:

- Existing local session actor, Codex pipe-exec, `fork_session`, and side-question one-shot paths use the Phase 1.c resolver. Done in `1d03536`.
- Room participant spawn reuses the same resolver through existing run creation paths. Future direct participant spawn APIs must not create a second resolver.
- Users can disable automatic env injection through Settings with `windows_msvc_env_mode = "off"`.

Follow-up acceptance:

- User-visible settings/status UI exposes `auto`, `always`, and `off`. Done.
- Warning state is actionable and does not expose raw environment values. Done.
- Manual Windows validation confirms `where cl` behavior from a non-Developer PATH, plus `off`, remote, and non-native auto no-op behavior. Done with ignored manual tests.

## Phase 3: General Roundtable

Prerequisite: AgentAdapter exists.

Status: Done. Merged to `master` in `b882c12 merge: phase 3 roundtable`.

Scope:

- `Room.kind = Roundtable`.
- Plain input fanouts to peer participants.
- `@debate` builds prompts from previous turn responses.
- `@summary @who` routes full room history summary task to one participant.
- `@who text` creates a private turn and writes `rooms/{id}/private.json`.

Acceptance:

- Fanout sends the same user question to all active peers.
- Debate excludes the target participant's own previous answer.
- Summary targets exactly one participant.
- Private messages do not appear in public room timeline.
- Room turn completion is based on adapter outcomes, not PTY buffer scanning.

## Phase 4: Driver / Copilot Room

Status: MVP merged in `72a7dab feat: add driver copilot rooms`.

Scope:

- `Room.kind = Driver`.
- One participant has `Driver` role.
- Copilots default to read-only review behavior.
- Review request is first implemented as internal command or slash command.
- `.arena/context.md`, `.arena/state.md`, and `.arena/memory` are generated from room/run references.
- MVP command syntax is `/review [@copilot ...] request`. Without explicit targets, all active copilots are used.

Acceptance:

- Driver can request review from one or more copilots.
- Review prompt includes recent context and stable run references.
- Copilot outputs are recorded as room review turns.
- Dangerous-operation review is documented but not required for MVP.

MVP notes:

- Driver rooms are selectable in `/rooms` during creation.
- The first participant in a Driver room defaults to `driver`; later participants default to `copilot`. Assigning a new `driver` role demotes any previous driver to `copilot`.
- Copilot review behavior is prompt-level read-only guidance for the MVP. Hard permission enforcement and dangerous-operation approval remain future work.

## Phase 4.5: Research Room

Status: MVP merged in `25dc4fe feat: Implement Phase 4.5 MVP for Research Room with structured artifacts`.

Research depends on Roundtable fanout and parts of Driver review. Split it into two levels:

- MVP Research Room after Phase 3: structured fanout research tasks and room-local research output.
- Full Research Room after Phase 4: review, fact extraction, and arena-memory candidates.

MVP notes:

- Research rooms are selectable in `/rooms` during creation.
- Plain input is treated as a research topic and fans out to all active participants.
- Research participants default to `researcher`; generic `participant` roles are normalized to `researcher` for Research rooms.
- `rooms/{id}/research/artifact.json` is the latest research artifact snapshot. Historical research turns remain in `timeline.jsonl`; artifact history is deferred.
- The orchestrator writes the latest artifact before appending the public research turn, so artifact write failures do not leave a public research turn without the promised artifact.

Follow-up notes:

- Research artifact schema version 2 records append-only artifact history in `rooms/{id}/research/artifacts.jsonl` while preserving `artifact.json` as the latest snapshot.
- Research prompts ask participants to mark durable project facts, decisions, and lessons with `[fact]`, `[decision]`, and `[lesson]`.
- Marked lines become Arena Memory candidates in the artifact with source participant, run, and turn references.
- `RoomDetail.research_artifact` exposes the latest artifact for `/rooms`, which renders a compact Research artifact / memory-candidate panel.

Scope:

- `Room.kind = Research`.
- Store outputs under `rooms/{id}/research/`.
- Reuse fanout and later review/fact extraction.

Acceptance:

- Research room can split a topic across participants.
- Results are collected into a structured room artifact.
- Full version can promote facts/decisions/lessons into Arena Memory.

## Phase 5: Multi-CLI Capability Matrix

Status: Done. Merged in `9224281 feat: add phase 5 capability matrix`.

Scope:

```ts
AgentCapabilities = {
  streamSession: boolean,
  pipeExec: boolean,
  interactivePty: boolean,
  resume: "session_id" | "latest" | "none",
  promptInjection: "system_prompt" | "append_file" | "instruction_file" | "env",
  mcpConfig: boolean,
  contextUsage: boolean,
  permissionProtocol: boolean,
}
```

Acceptance:

- Room orchestrator queries capabilities instead of agent names. Done locally: active Room targets are filtered by `stream_session`.
- Claude and Codex have explicit capability records. Done locally in Rust and frontend helper tests.
- Gemini integration waits until the adapter contract can support it cleanly. Done locally: Gemini is represented but unsupported for Room dispatch and pipe execution.

## Phase 5.5: Native CLI Chat and Connection Parity

Status: Done on `master` on 2026-05-05.

Goal: make native Codex and Gemini feel like first-class chat agents without pretending they are long-lived stream actors.

Scope:

- Render Codex / Gemini pipe-exec output through the same chat timeline surface used by CC instead of the terminal pane.
- Replay completed pipe-exec run events in event order, preserving multi-turn user / assistant history.
- Keep live pipe-exec deltas in `streamingText` and finalize them into assistant timeline entries when the run completes.
- Mirror CC's connection Settings pattern for Codex / Gemini: CLI Auth and App API Key cards, saved connection-profile JSON, native launch settings, and No-review mode.
- On Windows, resolve npm `.cmd` / `.bat` shims for Codex and Gemini into `node.exe + codex.js/gemini.js` so native chat sends do not flash a transient command window.
- Quote display-only command arguments for prompts containing spaces or non-ASCII text without changing structured process spawning.

Acceptance:

- Completed native CLI history loads as normal chat messages.
- Multi-turn pipe-exec replay keeps `user1 -> assistant1 -> user2 -> assistant2` order.
- Live native CLI output is archived as an assistant message on `chat-done`.
- Codex and Gemini Settings connection pages visually match CC's auth-mode pattern while keeping their native CLI toggles.
- Windows shim resolution has tests for both Codex and Gemini.

Out of scope:

- Persistent Codex/Gemini stream actors.
- Codex thread resume UI.
- Full MCP server management parity for native CLIs.

## Phase 6.a: Driver MCP Bundle

Status: In progress in `feat/phase6-driver-mcp`.

Goal: start the Driver MCP port by producing a structured, room-local MCP bundle for every Driver review turn.

Scope:

- Write `.arena/driver-mcp.json` before Driver review prompts are dispatched.
- Include stable room metadata, room description, cwd, participants, current review request, final review prompt, arena path, and recent public turns.
- Keep existing `.arena/context.md`, `.arena/state.md`, and `.arena/memory` generation unchanged.

Acceptance:

- `/review` in a Driver room writes `.arena/driver-mcp.json`.
- The bundle contains the current review request and stable room context.
- Existing Driver review routing and public review turns still behave unchanged.

Out of scope:

- Registering or launching a stdio MCP server.
- Research MCP server port.
- Changing `/chat` MCP configuration UI.

## Quality Gates

Every phase must verify:

- existing `/chat` behavior with feature flag off;
- storage round-trip for new JSON files;
- corrupt/missing file behavior;
- no full assistant output duplicated into room timeline unless explicitly required;
- Windows path handling where files are touched.

For frontend phases:

- run normal Svelte checks/tests;
- manually verify `/chat` remains reachable;
- verify `/rooms` hidden or disabled when feature flag is off.

## Testing Strategy

### Rust backend

- Unit tests live in `#[cfg(test)] mod tests` blocks colocated with the module.
- Integration tests for storage and command boundaries live under `src-tauri/tests/`.
- Each new `storage/*.rs` requires:
  - happy-path round-trip (`write -> read -> assert`)
  - corrupt-file fallback (truncated / invalid JSON)
  - missing-file default behavior
- Each new `commands/*.rs` requires at least one Tauri-side smoke test plus an error-path assertion.
- The `AgentAdapter` trait must have at least one mock implementation used in Roundtable orchestrator tests.

### Svelte frontend

- Vitest tests under `src/**/__tests__/` or co-located `*.test.ts`.
- Store tests for `room-store`, `memo-store` covering state transitions and event-driven updates.
- Manual smoke checklist per phase exit:
  - `/chat` reachable and behaves identically with the room feature flag off
  - `/rooms` hidden when feature flag off, visible when on
  - Doctor cards render correctly per platform (no Windows-specific cards on macOS / Linux)

### Cross-cutting

- Each phase exit must run `npm run verify` green — Claw GO's canonical aggregate gate, which wraps `lint`, `format:check`, `i18n:check`, `vitest`, `build`, `rustfmt`, and `clippy -D warnings`. Raw `cargo` / `eslint` / `vitest` invocations are implementation details inside the npm scripts; reference the script name as the gate, not the underlying command.
- `npm run check` (svelte-check) must additionally pass for type-checked Svelte changes.
- For Windows-specific work (Phase 1.c, Phase 2.x), CI must include a Windows runner before declaring the phase done.
- For multi-CLI work (Phase 3+), CI uses **mock `AgentAdapter` implementations plus deterministic integration tests** as the blocking gate. Real Claude + Codex sandbox runs are manual or nightly evidence collected before each release — they depend on accounts, network, model availability, rate limits, and cost, none of which should hold a phase exit hostage.

## Documentation Outputs

Keep these docs updated:

- `thinking.md` for raw synthesis and discussion history.
- `docs/ADR-001-room-over-run.md` for architectural decision.
- `docs/migration-decisions.md` for Hub capability mapping.
- `docs/implementation-roadmap.md` for delivery sequence.
- `docs/phase-1-memo-implementation-plan.md` for the first implementation slice.
