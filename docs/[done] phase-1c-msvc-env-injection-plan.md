# Windows MSVC Environment Injection Implementation Plan

**Feature:** Phase 2.x — Windows MSVC Environment Injection
**Status:** Done. Merged to `master`.
**Previous label:** Phase 1.c. Deferred until after Phase 2 Rooms; now the next planned implementation slice.
**Goal:** Local Claude/Codex-style CLI child processes launched from the OpenCovibe window can receive a Visual Studio / MSVC developer environment on Windows, without requiring the user to start OpenCovibe from Developer PowerShell.
**Acceptance Criteria:**
- On Windows projects that need native tooling, local child CLI processes can receive a derived Visual Studio / MSVC environment.
- `auto` mode injects only for conservative native-project signals; `always` forces injection; `off` disables it.
- SSH remote sessions and non-Windows platforms remain unchanged.
- The resolver is reused by Claude session actor spawn, Codex pipe-exec spawn, fork/one-shot, and later Room participant spawn paths.
- Env parsing and merging are tested without requiring a real Visual Studio installation in CI.
- Logs and UI/status warnings never dump full environment values or secrets.
**Architecture:** Add a small Windows-only MSVC env resolver module that decides whether injection is needed, derives env through `vswhere` + `VsDevCmd.bat`, caches the sanitized result, and exposes a merge plan to local spawn callers. `SpawnEnvPlan` only owns PATH + MSVC build variables + warnings; existing auth/base-url/model/provider-extra-env injection remains in the current caller paths.
**Tech Stack:** Rust/Tauri backend, `tokio::process::Command`, existing settings storage, Svelte settings/status UI only if the backend status needs to be visible.
**前端验证:** Yes, only for the mode/status surface. Backend behavior is verified primarily through Rust unit tests and manual Windows validation.

---

## Progress Snapshot

Last updated: 2026-04-30 after merging Task 7 and Task 8 to `master`.

### Done

- Task 1: settings shape added with `windows_msvc_env_mode: "auto" | "always" | "off"` and old-settings default coverage.
- Task 2: pure resolver core added for `set` output parsing, allowlist filtering, conservative native-project detection, PATH-like merge ordering, dedupe, and length warnings.
- Task 3: Windows derivation boundary added behind fakeable IO, with warning states for missing `vswhere`, missing VC tools workload, missing `VsDevCmd.bat`, sanitized inject output, and success cache by installation path / arch / host_arch.
- Task 4: Claude session actor local spawn integrated with `SpawnEnvPlan`; remote SSH branch remains unchanged.
- Task 5: Codex pipe-exec `run_agent` path integrated with `InheritUnlessInjected`, preserving inherited PATH unless MSVC injection is active.
- Task 6: both `fork_session -> claude_stream::fork_oneshot` and side-question one-shot local spawn integrated with the same resolver and protected extra-env merge.
- External review issue resolved before merge: GPT identified that `fork_session` still bypassed the resolver; fixed with `fork_oneshot_env_plan_injects_msvc_and_merges_extra_env`.
- Targeted validation passed before merge and again on merged `master`: `cargo test settings`, `cargo test windows_msvc_env`, `cargo test spawn_env`, `cargo test fork_oneshot_env_plan`, and `cargo clippy --lib -- -D warnings`.
- Merged to local `master` in `1d03536 merge: phase 2x msvc env`; pushed to `origin/master` together with README update `949dfe5`.
- Cleanup complete: temporary worktree `D:\ClaudeWorkspace\Code\claude-session-hub-remaster-phase2x-msvc` and branch `feat/phase2x-msvc-env` removed.
- Task 7 implemented and merged through `cfc13b8 feat: surface msvc env status` and `merge: phase 2x msvc status ui`: Settings exposes `auto | always | off`; backend exposes `get_windows_msvc_env_status`; status snapshots are updated by local spawn resolution; UI warning/status text does not expose raw environment values.
- Task 8 implemented as ignored Windows manual validation tests. Evidence on 2026-04-30: current non-Developer PATH returns `cl_not_on_current_path`; `cargo test manual_windows_msvc -- --ignored` passes native auto injection, off no-op, remote no-op, and plain non-native auto no-op.
- Real validation exposed and fixed a resolver bug: `cmd.exe` invocation of `VsDevCmd.bat` failed for `Program Files (x86)` paths because Rust `Command` escaped inner quotes for `cmd`; fixed by invoking `cmd` with Windows `CommandExt::raw_arg` and `call "...\\VsDevCmd.bat" ... >nul && set`.

### Pending

- Optional final interactive smoke before release: launch the desktop app normally, start a real Claude/Codex local session in this repo, and ask it to run `where cl`. The backend spawn-plan validation already covers the shared Codex pipe-exec env plan; a real Codex account/window run is still useful release evidence.
- Performance follow-up if needed: `VsDevCmd.bat` derivation is currently synchronous in the spawn path and cached after success. If first-spawn latency becomes visible in UI, consider moving derivation behind `spawn_blocking` or a prewarm/status cache.

---

## Finish Line

The finish line is narrow: when OpenCovibe is launched as a normal Windows desktop app, local Claude session actor, Codex pipe-exec, and fork/one-shot sessions started inside a native Windows project can run tools such as `cl`, `link`, Rust crates with MSVC targets, Tauri builds, and native npm modules because their child process env includes the Visual Studio developer variables.

We are not building CLI install/login/onboarding in this phase. That already exists in the app. We are also not installing Visual Studio, managing workloads, or rewriting the spawn architecture for all future agents.

## Terminal Schema

Use final-shaped types from the first implementation step so later spawn work extends them instead of rewriting them.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowsMsvcEnvMode {
    Auto,
    Always,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsvcEnvDecision {
    Skip { reason: MsvcEnvSkipReason },
    Inject { env: std::collections::HashMap<String, String>, source: MsvcEnvSource },
    Warn { message: String, next_action: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsvcEnvSkipReason {
    NonWindows,
    RemoteSession,
    DisabledByUser,
    ProjectDoesNotNeedNativeToolchain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsvcEnvSource {
    pub installation_path: std::path::PathBuf,
    pub arch: String,
    pub host_arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnEnvPlan {
    pub path_override: Option<String>,
    pub msvc_env: std::collections::HashMap<String, String>,
    pub warnings: Vec<MsvcEnvWarning>,
    pub status: MsvcEnvStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsvcEnvWarning {
    pub code: String,
    pub message: String,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathMergeResult {
    pub value: String,
    pub warnings: Vec<MsvcEnvWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsvcEnvStatus {
    Skipped(MsvcEnvSkipReason),
    Injected(MsvcEnvSource),
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnPathPolicy {
    /// Preserve the child process' inherited PATH unless MSVC injection is active.
    InheritUnlessInjected,
    /// Preserve existing Claude behavior by setting the app's augmented PATH even when MSVC is skipped.
    AlwaysUseAugmentedPath,
}
```

`SpawnEnvPlan` is not the complete child-process environment. It only represents the MSVC augmentation layer: optional PATH override, MSVC build variables, status, and warnings. Auth variables, base URL, model tier variables, `OPENCOVIBE_*`, and provider/user `extra_env` remain owned by the existing spawn callers and are applied after the MSVC plan according to the merge rules below.

The public backend API must not return the full injected env to the frontend. A status command can return mode, status, source path, and warning text, but not secrets or the full process environment.

## Default Policy

Recommended default: `auto` on Windows.

Reasoning: the user benefit is immediate for Tauri/Rust/native Node projects, while conservative project detection keeps the blast radius low for ordinary chat sessions. `off` remains available for users who already manage custom envs. `always` is useful for unusual native projects where heuristics miss the signal.

## Native Project Detection

MVP `auto` should return true for clear local signals:

- `src-tauri/` directory exists.
- `binding.gyp` exists.
- `package.json` contains dependency/devDependency names that commonly require native builds: `node-gyp`, `@tauri-apps/cli`, `tauri`, `electron`, `sharp`, `better-sqlite3`, `sqlite3`, `canvas`, `node-sass`, `esbuild`.
- Rust-native signal exists: `Cargo.toml` plus at least one stronger marker such as `build.rs`, `.cargo/config.toml` mentioning `msvc` or `windows`, dependency names commonly linked to native code (`cc`, `cmake`, `bindgen`, `windows`, `windows-sys`, `winapi`), or `rust-toolchain` / `rust-toolchain.toml` together with a Windows/MSVC target hint.

Keep this conservative. `Cargo.toml` alone is not enough for `auto`. False negatives can be handled by `always`; false positives cost spawn time and may surprise users.

## Env Derivation

Windows resolver sequence:

1. Locate `%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe`.
2. Run:

```powershell
vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
```

3. Build `VsDevCmd.bat` path:

```text
{installationPath}\Common7\Tools\VsDevCmd.bat
```

4. Run:

```cmd
cmd /d /s /c ""{VsDevCmd.bat}" -arch={arch} -host_arch={host_arch} >nul && set"
```

5. Parse `KEY=VALUE` lines from `set` output.
6. Keep only build-tool-relevant variables:

```text
PATH
INCLUDE
LIB
LIBPATH
VCToolsInstallDir
VCINSTALLDIR
VSINSTALLDIR
WindowsSdkDir
WindowsSDKVersion
UniversalCRTSdkDir
UCRTVersion
DevEnvDir
FrameworkDir
FrameworkDir64
FrameworkVersion
FrameworkVersion64
__VSCMD_PREINIT_PATH
VSCMD_ARG_TGT_ARCH
VSCMD_ARG_HOST_ARCH
VSCMD_VER
```

MVP architecture support is x64-first. On `std::env::consts::ARCH == "x86_64"`, use `-arch=x64 -host_arch=x64`. On other Windows host architectures, either map to a known `VsDevCmd` arch (`aarch64` to `arm64`) or return an actionable warning rather than guessing. The chosen arch and host_arch must be included in `MsvcEnvSource`.

## Merge Precedence

For local spawn commands:

1. Start with the app process env.
2. Choose PATH behavior through `SpawnPathPolicy`.
3. If MSVC injection is enabled, merge MSVC PATH-like values with the current base PATH and set MSVC build variables.
4. Apply auth env (`ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN`) and base URL exactly as today.
5. Apply model tier env exactly as today.
6. Apply provider/user `extra_env` last, with protected merge behavior for build-tool path variables.

PATH policy:

- Claude session actor and fork/one-shot currently set `PATH = augmented_path()`. They must use `AlwaysUseAugmentedPath` so existing behavior is preserved when MSVC is skipped.
- Codex pipe-exec currently does not set PATH in `src-tauri/src/agent/stream.rs::run_agent`. It must use `InheritUnlessInjected`, so non-Windows, non-native, and `off` mode preserve inherited PATH exactly as today.
- When MSVC injection is active, both policies produce a `path_override` that merges MSVC PATH entries with the selected base PATH.

PATH merge should deduplicate case-insensitively on Windows and preserve MSVC entries before the selected base PATH so `cl`, `link`, and Windows SDK tools resolve first.

Protected merge behavior:

- `PATH` / `Path`: merge `extra_env` entries before the planned PATH, dedupe case-insensitively, do not replace the entire PATH.
- `INCLUDE`, `LIB`, `LIBPATH`: merge `extra_env` entries before the MSVC-derived value using the Windows path separator, do not replace the entire variable.
- Other `extra_env` keys keep existing behavior: last writer wins.

This keeps explicit user/provider additions effective without silently deleting the injected MSVC toolchain.

Length guard: `merge_path_like` should warn if the merged PATH-like value crosses a conservative Windows safety threshold. The warning must not truncate the value silently; it should surface that the environment is unusually long and may fail on older toolchains.

## Cache Key

Cache only successful derived MSVC env results, not warning states. The cache key is:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MsvcEnvCacheKey {
    pub installation_path: std::path::PathBuf,
    pub arch: String,
    pub host_arch: String,
}
```

`cwd` and mode are not part of this cache key because they decide whether to request injection, not how `VsDevCmd.bat` derives the Visual Studio environment. The selected base PATH is not part of this cache key; final PATH is rebuilt per spawn by merging cached MSVC PATH with the current spawn policy's base PATH.

If `vswhere` resolves to a different installation path, use a different cache entry. If mode changes to `off`, skip cache lookup entirely.

## Task 1: Add Settings Shape

Status: Done. Merged in `1d03536`.

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/storage/settings.rs`

**Step 1: Write failing settings tests**

Add tests that deserialize old settings without the new field and assert the default is `auto`; deserialize explicit `"off"` and `"always"` values.

Run:

```powershell
cargo test settings --manifest-path src-tauri/Cargo.toml
```

Expected: fail because the new enum/field does not exist.

**Step 2: Implement minimal settings model**

Add `WindowsMsvcEnvMode` and a `windows_msvc_env_mode` field on `UserSettings` with default `Auto`.

**Step 3: Verify**

Run:

```powershell
cargo test settings --manifest-path src-tauri/Cargo.toml
```

Expected: settings tests pass or existing unrelated failures remain clearly unchanged.

**Step 4: Commit**

```powershell
git add src-tauri/src/models.rs src-tauri/src/storage/settings.rs
git commit -m "feat: add windows msvc env setting"
```

## Task 2: Add Pure MSVC Resolver Core

Status: Done. Merged in `1d03536`.

**Files:**
- Create: `src-tauri/src/agent/windows_msvc_env.rs`
- Modify: `src-tauri/src/agent/mod.rs`

**Step 1: Write failing parser and detection tests**

Test:

- `parse_set_output` keeps `KEY=VALUE` and supports values containing `=`.
- allowlist filtering excludes unrelated env keys.
- `project_needs_msvc` detects `src-tauri/`, `binding.gyp`, native package hints, and Rust projects only when stronger native/MSVC markers are present.
- `Cargo.toml` alone and non-native package projects return false.
- `merge_path_like` preserves MSVC tool paths before the selected base PATH.
- `merge_path_like` emits a warning when a merged PATH-like value exceeds the configured Windows safety threshold.

Run:

```powershell
cargo test windows_msvc_env --manifest-path src-tauri/Cargo.toml
```

Expected: fail because module/functions do not exist.

**Step 2: Implement pure functions**

Implement functions with no real process spawning:

```rust
pub fn parse_set_output(output: &str) -> HashMap<String, String>;
pub fn filter_msvc_env(raw: HashMap<String, String>) -> HashMap<String, String>;
pub fn project_needs_msvc(cwd: &Path) -> bool;
pub fn merge_path_like(base: &str, injected: &str) -> PathMergeResult;
```

**Step 3: Verify**

Run:

```powershell
cargo test windows_msvc_env --manifest-path src-tauri/Cargo.toml
```

Expected: tests pass.

**Step 4: Commit**

```powershell
git add src-tauri/src/agent/mod.rs src-tauri/src/agent/windows_msvc_env.rs
git commit -m "feat: add msvc env resolver core"
```

## Task 3: Add Windows Derivation Boundary

Status: Done. Merged in `1d03536`.

**Files:**
- Modify: `src-tauri/src/agent/windows_msvc_env.rs`

**Step 1: Write failing command-boundary tests**

Abstract command execution behind a small trait or function parameter so tests can inject sample `vswhere` and `set` outputs without requiring Visual Studio.

Test:

- missing `vswhere` returns `Warn` with next action.
- `vswhere` succeeds but returns empty stdout returns `Warn` that names the Visual Studio Installer workload "Desktop development with C++".
- missing `VsDevCmd.bat` returns `Warn`.
- successful sample output returns `Inject`.
- result is cached for the process after the first success.

Run:

```powershell
cargo test windows_msvc_env --manifest-path src-tauri/Cargo.toml
```

Expected: fail because derivation boundary does not exist.

**Step 2: Implement derivation**

Implement Windows-only process execution under `#[cfg(windows)]`; return `Skip { NonWindows }` under `#[cfg(not(windows))]`.

Do not log raw env output. Log only source path, mode, decision, and count of injected keys.

**Step 3: Verify**

Run:

```powershell
cargo test windows_msvc_env --manifest-path src-tauri/Cargo.toml
```

Expected: tests pass without a local Visual Studio dependency.

**Step 4: Commit**

```powershell
git add src-tauri/src/agent/windows_msvc_env.rs
git commit -m "feat: derive msvc developer env"
```

## Task 4: Integrate Claude Session Actor Spawn

Status: Done. Merged in `1d03536`.

**Files:**
- Modify: `src-tauri/src/commands/session.rs`
- Test: `src-tauri/src/commands/session.rs` unit tests or `src-tauri/src/agent/windows_msvc_env.rs` integration-style tests

**Step 1: Write failing env-plan tests**

Add a test around a pure helper such as:

```rust
fn build_spawn_env_plan(
    cwd: &Path,
    is_remote: bool,
    mode: WindowsMsvcEnvMode,
    path_policy: SpawnPathPolicy,
    base_path: Option<&str>,
    decision: MsvcEnvDecision,
) -> SpawnEnvPlan
```

Assert:

- remote returns no MSVC injection.
- `off` returns no MSVC injection.
- `AlwaysUseAugmentedPath` returns a PATH override matching existing Claude behavior even when MSVC is skipped.
- `InheritUnlessInjected` returns no PATH override when MSVC is skipped, preserving Codex pipe-exec's existing inherited PATH behavior.
- `auto` with native project merges MSVC PATH before the selected base PATH.
- final PATH order keeps MSVC compiler/SDK paths before ordinary PATH entries after protected merge.
- missing `vswhere` / `VsDevCmd.bat` warnings survive into `SpawnEnvPlan.warnings`.
- missing VC tools workload / empty `vswhere` result warnings survive into `SpawnEnvPlan.warnings`.
- provider `extra_env` can override a non-PATH variable after MSVC env is applied.
- provider `extra_env` containing `PATH`, `INCLUDE`, `LIB`, or `LIBPATH` is merged rather than replacing the MSVC value.

Run:

```powershell
cargo test spawn_env --manifest-path src-tauri/Cargo.toml
```

Expected: fail before helper exists.

**Step 2: Apply env plan in `spawn_cli_process`**

In the local branch only, replace the direct `PATH = augmented_path()` call with an env plan that may include MSVC variables. Keep the existing auth/model/extra-env order after the plan is applied.

Remote SSH branch remains unchanged.

Also sanitize existing local spawn logging in this task: remove full PATH logging and stop logging `extra_env` values. Logs may include env key names/counts and warning codes, but not full values.

**Step 3: Verify**

Run:

```powershell
cargo test spawn_env windows_msvc_env --manifest-path src-tauri/Cargo.toml
```

Expected: new tests pass.

**Step 4: Commit**

```powershell
git add src-tauri/src/commands/session.rs src-tauri/src/agent/windows_msvc_env.rs
git commit -m "feat: inject msvc env into claude session spawn"
```

## Task 5: Integrate Codex Pipe-Exec Spawn

Status: Done. Merged in `1d03536`.

**Files:**
- Modify: `src-tauri/src/commands/chat.rs`
- Modify: `src-tauri/src/agent/stream.rs`
- Modify: `src-tauri/src/agent/windows_msvc_env.rs`

**Step 1: Write failing coverage for pipe-exec env planning**

Current Codex pipe mode builds the command in `src-tauri/src/commands/chat.rs` and spawns it in `src-tauri/src/agent/stream.rs::run_agent`. Add tests around the shared env-plan helper proving this path receives the same MSVC plan as Claude session actor spawn.

Run:

```powershell
cargo test spawn_env --manifest-path src-tauri/Cargo.toml
```

Expected: fail until pipe-exec accepts and applies the shared plan.

**Step 2: Thread the env plan into `run_agent`**

Pass a prepared `SpawnEnvPlan` or the minimal inputs needed to derive one from `chat.rs` into `run_agent` using `SpawnPathPolicy::InheritUnlessInjected`. When MSVC injection is active, use the current process `PATH` as the selected base PATH and set the merged `path_override`. Apply `path_override` only when it is `Some`; this preserves current inherited PATH behavior for non-Windows, non-native, warning-only, and `off` decisions. Apply any MSVC env entries before `OPENCOVIBE_TASK_ID` / `OPENCOVIBE_RUN_ID`; keep existing pipe stdout/stderr behavior unchanged.

**Step 3: Verify**

Run:

```powershell
cargo test spawn_env windows_msvc_env --manifest-path src-tauri/Cargo.toml
```

Expected: tests pass.

**Step 4: Commit**

```powershell
git add src-tauri/src/commands/chat.rs src-tauri/src/agent/stream.rs src-tauri/src/agent/windows_msvc_env.rs
git commit -m "feat: inject msvc env into pipe agent spawn"
```

## Task 6: Integrate Fork/One-Shot Local Spawn

Status: Done. Merged in `1d03536`. Covers both `fork_session -> fork_oneshot` and side-question one-shot.

**Files:**
- Modify: `src-tauri/src/agent/claude_stream.rs`
- Modify: `src-tauri/src/commands/session.rs` if settings must be threaded into `fork_oneshot`

**Step 1: Write failing coverage for fork env planning**

Reuse the same pure env-plan helper so fork/one-shot cannot drift from normal chat behavior.

Run:

```powershell
cargo test spawn_env --manifest-path src-tauri/Cargo.toml
```

Expected: fail until fork path calls the shared env plan.

**Step 2: Thread the mode into fork spawn**

`fork_oneshot` currently receives auth/model/extra env but not full user settings. Pass the MSVC mode or a prepared `SpawnEnvPlan` from `session.rs` so `claude_stream.rs` does not need to load settings directly.

**Step 3: Verify**

Run:

```powershell
cargo test spawn_env windows_msvc_env --manifest-path src-tauri/Cargo.toml
```

Expected: tests pass.

**Step 4: Commit**

```powershell
git add src-tauri/src/agent/claude_stream.rs src-tauri/src/commands/session.rs
git commit -m "feat: reuse msvc env for local fork spawn"
```

## Task 7: Add Minimal Status Surface

Status: Done and merged.

**Files:**
- Modify: `src-tauri/src/commands/diagnostics.rs` or create a small dedicated command if diagnostics is too broad
- Modify: relevant Svelte settings/status component
- Modify: localized message catalogs if new UI copy is added

**Step 1: Write backend status test**

Test that status output includes:

- mode
- skipped/injected/warn state
- source path when available
- next action when unavailable
- no raw environment map
- warning state propagation for missing `vswhere` / missing `VsDevCmd.bat`
- warning state propagation for empty `vswhere` output / missing VC tools workload

Run:

```powershell
cargo test msvc_status --manifest-path src-tauri/Cargo.toml
```

Result: completed. Added tests for injected status, warning propagation, cheap pending precheck, and cwd/mode invalidation.

**Step 2: Implement minimal status**

Expose a command used by settings/session UI. Avoid making this only a Doctor card; the useful state should be visible where users configure or launch CLI sessions.

Status semantics:

- Status is a snapshot for the current selected project cwd.
- Opening settings must not force an expensive `VsDevCmd.bat` run.
- Local spawn is allowed to trigger resolver execution and update the snapshot.
- Warning snapshots remain current until the next successful resolver decision for the same cwd/mode, or until cwd/mode changes invalidate them.
- Cwd or mode changes invalidate the displayed snapshot until the next cheap precheck or spawn result.
- Warnings should point to the Visual Studio Installer workload "Desktop development with C++" when the VC tools workload is missing.
- Background pre-warming is allowed only after a native-project signal is detected and must not block first render; if implemented, it should update the same snapshot/cache path as normal spawn.

Optional UI: a subtle settings/status indicator or toolbar tooltip can show `MSVC env: auto/injected/warning/off` for the active cwd. It must not expose env values.

Result: completed. Status lives in Settings near connection/session configuration rather than only Doctor. Opening Settings performs only cheap precheck / cached snapshot read; `VsDevCmd.bat` is only invoked during local spawn resolution or explicit manual validation.

**Step 3: Verify UI manually**

Run the app and confirm the mode control/status text renders without overlapping existing settings UI.

```powershell
pnpm tauri dev
```

Result: completed via frontend build and local `/settings` Vite smoke. Full `npm run check` still fails on existing Svelte/type baseline, not on the MSVC UI paths.

**Step 4: Commit**

```powershell
git add src-tauri/src/commands/diagnostics.rs src/**/*.svelte src/lib/**/*.ts
git commit -m "feat: surface msvc env status"
```

## Task 8: Manual Windows Validation

Status: Done and merged.

**Files:**
- No required code files.
- Optional: update `README.md` if user-facing behavior needs a short note after implementation is accepted.

**Step 1: Validate without Developer PowerShell**

Start OpenCovibe normally from Explorer or Start Menu, open this repo or another Tauri/Rust project, launch a local CLI session, and ask it to run:

```powershell
where cl
cl
```

Expected: `cl` resolves and prints Microsoft compiler version/help.

Result: covered by ignored manual test `manual_windows_msvc_auto_injects_cl_for_native_project`. On this machine, ordinary PATH does not find `cl`; the injected spawn plan PATH makes `where cl` resolve `cl.exe`.

**Step 2: Validate `off`**

Set mode to `off`, restart or relaunch a session, and run:

```powershell
where cl
```

Expected: if the app was not started from a developer shell, `cl` is not found.

Result: current shell evidence: `where cl` returns not found; ignored manual test `manual_windows_msvc_off_mode_does_not_inject` confirms no env/path injection when mode is `off`.

**Step 3: Validate remote no-op**

Launch an SSH remote session.

Expected: remote command construction is unchanged; no local MSVC variables are forwarded.

Result: ignored manual test `manual_windows_msvc_remote_session_does_not_inject` confirms remote plan has no path override and no MSVC env.

**Step 4: Validate non-native project**

Open a plain non-native project in `auto` mode.

Expected: no MSVC injection; existing `/chat` behavior remains unchanged.

Result: ignored manual test `manual_windows_msvc_auto_skips_plain_non_native_project` confirms plain non-native project is skipped in `auto`.

**Step 5: Validate Codex pipe-exec**

In a native project with mode `auto` or `always`, launch a Codex pipe-mode run and ask it to run:

```powershell
where cl
```

Expected: `cl` resolves through the injected MSVC env.

Result: shared spawn-plan regression `cargo test spawn_env` passes and Codex pipe-exec keeps `SpawnPathPolicy::InheritUnlessInjected`. A real interactive Codex account/window run remains optional release evidence after merge.

## Risks and Review Focus

- **Spawn latency:** `VsDevCmd.bat` can be slow. Cache successful derivations by `MsvcEnvCacheKey`; mode/cwd decide whether to request injection, while final PATH is rebuilt per spawn.
- **Secret leakage:** never log full env maps. Only log key names/counts and source path.
- **PATH length and ordering:** preserve MSVC entries before app augmented PATH, dedupe case-insensitively.
- **False positives in `auto`:** keep detection conservative; users can select `always`.
- **Settings migration:** old settings must deserialize without manual migration.
- **Fork drift:** fork/one-shot currently has its own local spawn path; it must share the same env-plan logic or be explicitly deferred. This plan includes it.
- **Pipe-exec drift:** Codex pipe mode spawns through `commands/chat.rs` and `agent/stream.rs`, not the Claude session actor. This plan includes it.

## Suggested Review Questions

- Is `auto` the right default, or should first release default to `off` with an opt-in status prompt?
- Are the Rust-native markers conservative enough, or should the first MVP only auto-detect `src-tauri/`, `binding.gyp`, and native npm dependencies?

---

## Phase 8 Enhancements (2026-05-08)

**Status:** Done. Merged to `master`.

### Changes

1. **Extended auto detection for Qt/CMake/vcpkg/VS projects**
   - Added root-only detection for: `CMakeLists.txt`, `vcpkg.json`, `*.sln`, `*.vcxproj`, `*.pro`, `*.pri`
   - Detection is intentionally root-only to avoid false positives from build/, cache dirs, or submodules
   - Projects with solution files in subdirectories may need `always` mode

2. **Chat/Room MSVC injection policy split**
   - Added `MsvcPolicy` enum (`AllowByMode` / `Disabled`)
   - Added `resolve_spawn_env_plan_with_policy()` function
   - Chat sessions: continue using `AllowByMode` (existing behavior)
   - Room participant sessions: explicitly use `Disabled` policy (backend truly disables injection, not just UI hiding)

3. **Real injection status propagated to frontend**
   - Added `msvc_injected: Option<bool>` field to `BusEvent::SessionInit`
   - Derived from `SpawnEnvPlan.status` at spawn time (`Injected` → `true`, others → `false`)
   - Frontend `SessionStore` tracks `msvcInjected` state (null = unknown, true/false = actual status)
   - Status resets to `null` on `_clearContentState()` and is not guessed from historical runs

4. **MSVC badge in chat status bar**
   - Added `MSVC` badge in `SessionStatusBar` component, positioned left of `bypass` badge
   - Same styling as `bypass`: `bg-amber-500/15 text-amber-500`
   - Only shown when `msvcInjected === true`
   - Added i18n: `statusbar_msvcTitle` (en: "MSVC build environment injected", zh-CN: "MSVC 编译环境已注入")

### Key Design Decisions

- **Root-only detection**: Avoids false positives from build/, cache dirs, or submodules. Trade-off: some projects with solution files in subdirectories may need `always` mode.
- **Backend-enforced room policy**: Room participant sessions truly skip MSVC injection at the backend level, not just UI hiding. This ensures consistent behavior regardless of frontend state.
- **Ephemeral session fact**: `msvc_injected` is a chat-only session state, not a global setting or run metadata. Historical runs don't carry this field unless explicitly replayed with it.

### Known Limits / Intentional Scope

These behaviors are by design, not bugs:

1. **Auto detection is root-only**: `project_needs_msvc()` only checks markers in the project root directory. It does not recurse into subdirectories. If a project has `.sln` or `.vcxproj` in a subdirectory (e.g., `vendor/` or `third_party/`), auto mode will not detect it. Users should switch to `always` mode for such projects.

2. **MSVC badge only reflects chat sessions**: The `msvcInjected` badge in `SessionStatusBar` only appears for chat sessions with confirmed injection. Room participant sessions never show this badge, even if the underlying project could use MSVC. This is intentional: room participants use `MsvcPolicy::Disabled`.

3. **Badge is ephemeral**: The `msvc_injected` field is not persisted to run metadata. Historical runs replayed from storage will show `null` (no badge), not the original injection status. This is a session-time fact, not a project property.

4. **`RoomPolicy` skip reason**: When MSVC injection is skipped due to `MsvcPolicy::Disabled` (room participants), the skip reason is `RoomPolicy`, distinct from `DisabledByUser` (user turned off MSVC mode in settings). This distinction aids diagnostics.

### Files Modified

- `src-tauri/src/agent/windows_msvc_env.rs`: Extended `project_needs_msvc()`, added `MsvcPolicy` enum and `resolve_spawn_env_plan_with_policy()`
- `src-tauri/src/room/orchestrator.rs`: Changed to use `MsvcPolicy::Disabled`
- `src-tauri/src/models.rs`: Added `msvc_injected` field to `BusEvent::SessionInit`
- `src-tauri/src/commands/session.rs`: Refactored `spawn_cli_process` to return `SpawnCliResult` with `msvc_injected`
- `src-tauri/src/agent/session_actor.rs`: Added `msvc_injected` field, injects into `SessionInit` event
- `src-tauri/src/agent/claude_protocol.rs`: Added `msvc_injected: None` to `SessionInit` construction
- `src/lib/types.ts`: Added `msvc_injected` to `BusEvent` type
- `src/lib/stores/session-store.svelte.ts`: Added `msvcInjected` state, reducer handling, and clear logic
- `src/lib/stores/session-store.test.ts`: Added tests for `msvc_injected` handling
- `src/lib/components/SessionStatusBar.svelte`: Added `msvcInjected` prop and MSVC badge
- `src/routes/chat/+page.svelte`: Passes `msvcInjected` to `SessionStatusBar`
- `messages/en.json`: Added `statusbar_msvcTitle`
- `messages/zh-CN.json`: Added `statusbar_msvcTitle`
- Should status live in settings only, or also near session launch errors?
- Should warning snapshots be persisted across app restarts, or only held in process memory?
