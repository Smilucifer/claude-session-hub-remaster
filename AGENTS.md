# Repository Guidelines

## Project Structure & Module Organization

This repository is a Tauri desktop app with a SvelteKit frontend and Rust backend. Frontend source lives in `src/`, with routes under `src/routes`, components in `src/lib/components`, stores in `src/lib/stores`, and utilities in `src/lib/utils`. Rust commands, storage, models, and agent adapters live in `src-tauri/src`. Static assets are in `static/`, translations are in `messages/`, and implementation notes are in `docs/`. Tests are colocated where possible, for example `src/lib/utils/*.test.ts` and Rust `#[cfg(test)]` modules.

## Build, Test, and Development Commands

- `npm install`: install Node dependencies.
- `npm run dev`: start the Vite development server.
- `npm run tauri dev`: run the desktop app locally.
- `npm test`: run Vitest tests.
- `npm run lint`: run ESLint on `src/`.
- `npm run check`: run Svelte type checks.
- `npm run build`: build the frontend.
- `cargo test --manifest-path src-tauri/Cargo.toml`: run Rust tests.
- `cargo fmt --manifest-path src-tauri/Cargo.toml`: format Rust code.

## Coding Style & Naming Conventions

Use TypeScript strict-mode patterns and Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props()`). Run Prettier for frontend files and `cargo fmt` for Rust before submitting changes. Use PascalCase for Svelte components, camelCase for TypeScript functions and variables, and snake_case for Rust functions, modules, and fields. Keep provider identity separate from execution identity: DeepSeek/GLM are displayed as providers but execute through Claude-compatible paths.

## Testing Guidelines

Add Vitest coverage for frontend stores/utilities and Rust unit tests for backend command or adapter behavior. Name frontend tests `*.test.ts`; keep Rust tests near the module under test. For provider work, cover new chat, resume/continue, room participants, provider identity, and command argv generation.

## Commit & Pull Request Guidelines

Git history uses Conventional Commits such as `feat:`, `fix:`, `chore:`, and `merge:`. Keep PRs focused and include a concise description, linked issue when applicable, screenshots for UI changes, and the exact checks run. When adding UI text, update both `messages/en.json` and `messages/zh-CN.json`.

## Security & Configuration Tips

Do not commit API keys, local settings, or generated runtime state. Claude, Codex, and Gemini should use official CLI authentication. DeepSeek/GLM API configuration belongs in local settings only.
