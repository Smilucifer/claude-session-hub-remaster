# Phase 6.a Driver MCP Bundle Implementation Plan

**Feature:** Phase 6.a - Driver MCP Bundle
**Status:** In progress in `feat/phase6-driver-mcp`.
**Goal:** Start the Driver MCP port by writing a structured, room-local MCP bundle for every Driver review turn.
**Acceptance Criteria:**
- `/review` in a Driver room writes `.arena/driver-mcp.json`.
- The bundle contains stable room metadata, participants, cwd, room description, current review request, final review prompt, arena path, and recent public turns.
- Existing Driver review routing and public review turns remain unchanged.
**Architecture:** Keep Room over Run. Driver review still dispatches through `run_driver_turn`; before dispatch, storage writes the existing Arena markdown files plus a structured JSON bundle for future MCP server wiring.
**Tech Stack:** Rust/Tauri backend, room storage JSON, Rust unit tests.
**Frontend Verification:** No frontend change in this slice.

---

## Finish Line

Phase 6.a is complete when Driver reviews produce a deterministic `.arena/driver-mcp.json` bundle alongside the existing `.arena/context.md`, `.arena/state.md`, and `.arena/memory` files.

Out of scope:

- Registering or launching a stdio MCP server.
- Research MCP server port.
- Changing `/chat` or `/plugins` MCP UI.

## Implementation Notes

- `storage::rooms::write_driver_mcp_bundle` writes via temp file + rename.
- The bundle keeps only the latest eight public turns, matching the existing Driver arena context window.
- `run_driver_turn` writes the bundle before sending prompts to copilots, so reviewers can inspect stable context while their turn runs.

## Verification Evidence

Recorded during local implementation:

```powershell
cargo test driver_review_routes_to_copilots_and_records_review_turn --manifest-path src-tauri/Cargo.toml
cargo test room::orchestrator --manifest-path src-tauri/Cargo.toml
cargo test rooms --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings
git diff --check
```
