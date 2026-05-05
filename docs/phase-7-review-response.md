# Phase 7 Review Response

**Date:** 2026-05-05  
**Sources:** `C:\Users\InBlu\review\claude.md`, `codex.md`, `gemini.md`, and `user.md`

## Owner Requirement

Codex and Gemini are not considered complete just because their native CLIs can be launched. Their output must be parsed and rendered back into the normal chat timeline like Claude Code, not archived as raw terminal output or treated as a long-running TUI dump.

## Accepted Blocking Items

- **P0 native adapter:** Codex/Gemini interactive CLI sessions cannot use the old `pipe_exec` contract of `stdin=null`, read stdout/stderr until EOF, and process exit equals turn completion. A native adapter must prove prompt delivery, completion detection, assistant archival, stop/cancel, and timeline rendering.
- **P1 provider routing:** DeepSeek/GLM platform ids must survive all Claude-compatible session paths, including start, continue, fork, approval restart, side question, and room participant launches.
- **P1 launch model injection:** Official CLI providers must not receive provider catalog display defaults as actual launch model ids unless the user explicitly selected a model.
- **P2 permission policy:** Codex/Gemini bypass/yolo behavior is Phase 7 provider policy, not a user-controlled setting that can be disabled through stale `yolo_mode=false` state.

## Implementation Decisions

- Keep provider identity separate from execution identity. DeepSeek and GLM stay first-class provider choices while using Claude Code compatible execution.
- Preserve old settings on disk, but remove obsolete login-method/profile complexity from the visible settings surface.
- Continue documenting Codex/Gemini native command generation as incomplete until the adapter renders parsed conversation content.
- Treat external review as input to verify against this codebase, not as automatic implementation orders.

## Current Follow-Up Order

1. Keep bypass/yolo centralized as provider policy and prevent stale `yolo_mode=false` from changing the visible native provider mode.
2. Keep DeepSeek/GLM platform routing helper covered by Rust tests.
3. Continue validating the Rust native transcript adapter. Real PTY smoke tests now prove Codex/Gemini transcript binding and assistant extraction for new turns; remaining validation must cover resume, UI stop/cancel, room-turn completion, and repeated multi-turn stability.
4. Continue Roundtable parity after the initial prompt/action port: remove room memo, add global memo pop-out, expand memory management, and complete the three-pane room layout/history presentation.

## Task 6/7 Review Response - 2026-05-05

Sources: `C:\Users\InBlu\review\claude.md`, `codex.md`, and `gemini.md`.

Accepted and fixed:

- **P1 native resume transcript baseline:** Codex/Gemini native transcript parsing now captures the provider transcript baseline before launching the PTY process and only accepts completions appended after that baseline for the same transcript file. This prevents resume/latest turns from archiving an older completed answer as the current turn.
- **P2 debate context after summary:** Debate prompts now use the latest completed `fanout` or `debate` turn as peer context, skipping `summary` and private turns. The Room Debate button is also enabled from completed fanout/debate history, not any public turn.

Accepted follow-up, not blocking this repair:

- Keep Memory candidates and launch-time instruction conventions aligned by moving both toward a shared provider instruction registry.
- Surface Memory provider metadata more explicitly in the UI when the Memory page is polished.
- Continue strengthening prompt tests around attribution and truncation boundaries as Roundtable presentation work continues.
