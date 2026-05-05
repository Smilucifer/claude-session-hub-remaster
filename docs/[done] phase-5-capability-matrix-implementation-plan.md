# Phase 5 Multi-CLI Capability Matrix Implementation Plan

**Feature:** Phase 5 - Multi-CLI Capability Matrix
**Status:** Implemented locally on 2026-05-03 in `feat/phase5-capability-matrix`.
**Goal:** Make Room orchestration and Room UI consume explicit agent capability records instead of relying on hard-coded agent names.
**Acceptance Criteria:**
- Room orchestrator queries capabilities instead of agent names.
- Claude and Codex have explicit capability records.
- Gemini integration waits until the adapter contract can support it cleanly.
**Architecture:** Keep Run as the execution unit. Add a serializable capability matrix to the Room adapter boundary, expose it through Room participant details, and migrate attach/orchestration eligibility checks to capability predicates.
**Tech Stack:** Rust/Tauri backend, Svelte 5 room UI, TypeScript API types, Rust unit tests, Vitest store tests.
**Frontend Verification:** Yes - `/rooms` attach filtering uses the shared frontend capability helper.

---

## Finish Line

Phase 5 is complete when each Room participant has a capability snapshot, orchestration filters participants by capability predicates, Claude and Codex capabilities are explicit and tested, and Gemini remains represented as unsupported until a clean adapter contract exists.

Out of scope:

- Building a Gemini adapter.
- Multi-terminal PTY wall.
- Making Codex a long-lived Room participant before the app has a session actor equivalent for Codex.
- Rewriting existing `/chat` execution paths.

## Implementation Notes

- Backend `AgentCapabilities` now records `stream_session`, `pipe_exec`, `interactive_pty`, `resume`, `prompt_injection`, `mcp_config`, `context_usage`, and `permission_protocol`.
- Claude supports stream sessions and pipe exec; Codex supports pipe exec only in the current app; Gemini is explicitly unsupported for Room dispatch for now.
- `RoomParticipantDetail` exposes a participant capability snapshot.
- Room active target resolution filters by `stream_session`.
- Run execution-path defaults are capability-driven instead of name-driven.
- `/rooms` attach filtering uses the shared frontend capability helper.

## Verification Evidence

Recorded during local implementation:

```powershell
cargo test room::adapter --manifest-path src-tauri/Cargo.toml
cargo test rooms --manifest-path src-tauri/Cargo.toml
npm test -- src/lib/utils/agent-capabilities.test.ts --run
npm test -- src/lib/stores/room-store.test.ts --run
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings
npm run lint
npm run i18n:check
npm run build
git diff --check
```

Known baseline:

```powershell
npm run check
```

Still fails on pre-existing repo-wide Svelte/TypeScript diagnostics: 105 errors / 71 warnings in 45 files. The Phase 5 Room/capability files were not part of the reported failure list.
