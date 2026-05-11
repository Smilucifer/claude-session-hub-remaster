# Phase 5.5 Native CLI Chat Parity Implementation Plan

**Feature:** Phase 5.5 - Native Codex/Gemini Chat and Connection Parity
**Status:** Done on `master` on 2026-05-05.
**Goal:** Make Codex and Gemini native CLI runs feel like normal Claw GO chat sessions while preserving one-shot pipe-exec semantics.

## Scope

- Codex and Gemini pipe-exec runs render in the normal chat timeline instead of defaulting to terminal replay.
- Completed pipe-exec histories replay `user` and `assistant` events in order.
- Live pipe-exec deltas append to `streamingText` and are finalized into assistant timeline entries on `chat-done`.
- Codex and Gemini Settings connection panels match CC's auth-mode pattern: CLI Auth, App API Key, saved connection profiles, native launch settings, and No-review mode.
- Windows npm `.cmd` / `.bat` shims for Codex and Gemini resolve to `node.exe + CLI js` before spawning, avoiding transient `cmd` windows.
- Display-only started-command messages quote prompts with spaces, quotes, or non-ASCII text; actual process spawning remains structured and shell-free.

## Acceptance

- A single-turn completed native CLI run reopens as `user -> assistant` chat messages.
- A multi-turn completed native CLI run reopens as `user1 -> assistant1 -> user2 -> assistant2`.
- A live native CLI response is archived as an assistant message when the backend emits `chat-done`.
- Codex and Gemini Settings pages show the same CLI/API auth mode surface as CC while keeping native settings such as model, extra args, add-dir/include-directories, ephemeral mode, and No-review mode.
- Windows shim resolution is covered for both `codex.cmd` and `gemini.cmd`.

## Review Fixes

- External review identified a P1 replay regression where only the final assistant event survived in multi-turn pipe-exec history. Fixed by flushing assistant content in event order and adding `preserves multi-turn native CLI replay order`.
- External review requested Gemini shim parity coverage. Added a Windows-only `gemini.cmd` shim test beside the Codex test.

## Verification

- `npm run test -- src/lib/stores/session-store.test.ts` -> 268 passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --no-run` -> passed.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` -> passed.
- `npm run build` -> passed with existing Svelte/Vite warnings.

## Out of Scope

- Persistent Codex/Gemini stream actors.
- Codex thread resume UI.
- Full native CLI MCP/plugin management parity.
- Changing Room storage to duplicate full assistant output.
