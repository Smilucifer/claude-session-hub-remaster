# Claude Session Hub Remaster / Claude Session Hub 重制版

> English follows Chinese in each section.
> 每个章节均先中文、后英文。

## 致谢 / Acknowledgements

本项目基于 [AnyiWang/OpenCovibe](https://github.com/AnyiWang/OpenCovibe) 的本地优先桌面架构与 UI 基础继续演进，并吸收了 [TianLin0509/claude-session-hub](https://github.com/TianLin0509/claude-session-hub) 在会议室、备忘录、协作协议和多 CLI 工作流上的产品启发。感谢这两个项目对本 remaster 方向的帮助。

This project evolves from the local-first desktop architecture and UI foundation of [AnyiWang/OpenCovibe](https://github.com/AnyiWang/OpenCovibe), while drawing product inspiration from [TianLin0509/claude-session-hub](https://github.com/TianLin0509/claude-session-hub), especially around rooms, memos, collaboration protocols, and multi-CLI workflows. Many thanks to both projects for shaping this remaster.

## 项目定位 / Project Positioning

Claude Session Hub Remaster 的目标不是重写 OpenCovibe，也不是直接搬运 Claude Session Hub。它以 OpenCovibe 当前的 Tauri + Svelte + Rust 架构为主体，逐步叠加 Claude Session Hub 中有差异化价值的协作能力。

Claude Session Hub Remaster is not a rewrite of OpenCovibe and not a direct port of Claude Session Hub. It keeps OpenCovibe's current Tauri + Svelte + Rust architecture as the foundation, then incrementally adds the collaboration ideas that make Claude Session Hub distinctive.

当前架构原则：

- `Run` 继续作为最小执行单元。
- `Room` 作为协作编排层，建立在 `Run` 之上。
- Memo、Room、Roundtable、Driver/Copilot、Research 已落地；Arena Memory 候选提升仍在后续阶段。
- 现有 `/chat` 路径保持稳定。

Current architecture principles:

- `Run` remains the smallest execution unit.
- `Room` serves as the orchestration layer above `Run`.
- Memo, Room, Roundtable, Driver/Copilot, and Research are implemented; Arena Memory promotion is planned.
- The existing `/chat` path remains stable.

## 当前功能 / Current Features

### Memo

你可以在应用内使用快速备忘（全局备忘录），用来保存长期偏好、项目约定、临时上下文和后续待办。备忘面板从页面右上角剪贴板图标滑出，不干扰当前工作流。也可以通过 Command Palette 打开。

You can use Quick Memo (global memos) inside the app for preferences, project conventions, temporary context, and follow-up notes. The memo panel slides out from a clipboard-icon button in the top-right corner of every page, staying out of your way. Also accessible via Command Palette.

支持：

- 添加、复制、删除备忘条目。
- 每条备忘显示内容与时间戳。
- 重启后保留备忘内容。
- 非聊天页顶部工具栏和聊天页 SessionStatusBar 均有入口。

Supported:

- Add, copy, and delete memo entries.
- Each memo shows text and timestamp.
- Keep memo content after restart.
- Accessible from the top bar on non-chat pages and SessionStatusBar on the chat page.

### Rooms, Roundtable, and Driver/Copilot

Rooms 是多智能体协作的入口。你可以创建 Room、添加 Claude / Codex participant，并把 participant 关联到已有或新建的 Run。Room 删除不会删除对应 Run。Research Room 作为 Room kind 在同一入口中创建。Room participant 会话在侧边栏中以虚拟"会议室"文件夹单独分组，与项目会话分离。

Rooms are the entry point for multi-agent collaboration. You can create a Room, add a Claude / Codex participant, and link that participant to an existing or newly created Run. Deleting a Room does not delete the linked Run. Research Room is available as another Room kind from the same entry point. Room participant sessions are grouped in a virtual "Rooms" folder in the sidebar, separate from project sessions.

当前适合用来：

- 把相关 Run 聚合到一个 Room 中查看。
- 在 Roundtable 时间线里向多个活跃 participant 分发同一个问题。
- 使用 `@debate` 让 participant 基于上一轮公开回复互相比较观点。
- 使用 `@summary @name` 指定一个 participant 总结公开 Room 历史。
- 使用 `@DisplayName message`（SingleTarget）发送公开回合，仅由被点名的 participant 回答。
- 使用 `/dm @Name message` 发送私有回合，私有内容不会出现在公开时间线中。
- 通过 Stepper mini-map 逐轮回放——点击任意回合加载快照，紫色横幅 + pane 内容叠加显示。
- 创建 Driver Room，让一个 Driver 通过 `/review` 向一个或多个 Copilot 请求只读审查。
- Driver review 会生成 room-local `.arena/context.md`、`.arena/state.md` 和 `.arena/memory`，用于稳定引用 room / run 上下文。
- `.arena` 文件是本地运行上下文镜像，可能包含 run references、memo 和最近的公开 preview；不要把它当成对外分享材料。
- 创建 Research Room，把一个研究主题分发给多个活跃 participant。
- Research Room 会把本轮结果汇总到 room-local `research/artifact.json` 结构化产物，向 `research/artifacts.jsonl` 追加 artifact 历史，并把 `[fact]`、`[decision]`、`[lesson]` 标记行展示为 Arena Memory 候选。

Currently useful for:

- Grouping related Runs in a Room.
- Sending one prompt to multiple active participants in a Roundtable timeline.
- Using `@debate` to ask participants to compare positions based on previous public replies.
- Using `@summary @name` to ask one participant to summarize the public Room history.
- Using `@DisplayName message` (SingleTarget) to send a public turn answered only by the named participant.
- Using `/dm @Name message` for private turns that do not appear in the public timeline.
- Turn-by-turn Stepper mini-map for room replay — click any turn to load its snapshot with purple banner and pane content overlay.
- Creating a Driver Room where one Driver asks one or more Copilots for read-only review with `/review`.
- Driver review generates room-local `.arena/context.md`, `.arena/state.md`, and `.arena/memory` files for stable room / run context references.
- `.arena` files are local runtime context mirrors and may include run references, memo text, and recent public previews; do not treat them as shareable artifacts.
- Creating a Research Room that fans out one research topic to multiple active participants.
- Research Room writes a room-local structured `research/artifact.json` artifact for the latest research turn, appends artifact history to `research/artifacts.jsonl`, and surfaces `[fact]`, `[decision]`, and `[lesson]` lines as Arena Memory candidates.

### Providers and CLI Authentication

当前主力 provider：Claude、Codex、DeepSeek、GLM、QWEN、KIMI、MiMo Pro。Codex 通过 PTY 原生 CLI 执行，默认带 `--dangerously-bypass-approvals-and-sandbox`；DeepSeek、GLM、QWEN、KIMI、MiMo Pro 作为一等 provider 显示，执行层复用 Claude Code compatible session，并通过 `platform_id` 注入对应 API 配置。

The current primary providers are: Claude, Codex, DeepSeek, GLM, QWEN, KIMI, and MiMo Pro. Codex uses PTY-based native CLI execution and defaults to `--dangerously-bypass-approvals-and-sandbox`. DeepSeek, GLM, QWEN, KIMI, and MiMo Pro are first-class providers in the UI, but execute through Claude Code compatible sessions with provider configuration injected by `platform_id`.

设置页提供每个 provider 独立的模型和认证配置。Claude、Codex 使用官方 CLI 认证；DeepSeek、GLM、QWEN、KIMI、MiMo Pro 支持 API key / base URL / model 动态配置，每次启动从最新设置生成 per-session 临时配置。Codex 使用 PTY 原生 CLI 执行路径，通过 transcript 文件监听完成判定而非进程退出语义。

The settings page provides independent model and auth configuration per provider. Claude and Codex use official CLI authentication. DeepSeek, GLM, QWEN, KIMI, and MiMo Pro support dynamic API key / base URL / model configuration, generating a per-session temp config from the latest settings on each launch. Codex uses a PTY-based native CLI execution path with transcript-based completion detection rather than process-exit semantics.

Claude-compatible API profiles can still be represented under `user.cc_agent_profiles` in `~/.opencovibe/settings.json` for backward compatibility. API providers should keep provider identity separate from the Claude execution agent:

为了向后兼容，Claude-compatible API profile 仍可保留在 `~/.opencovibe/settings.json` 的 `user.cc_agent_profiles` 下。API provider 应保持 provider identity 和 Claude execution agent 分离：

```json
{
  "user": {
    "cc_agent_profiles": [
      {
        "id": "qwen-via-claude",
        "label": "QWEN via Claude",
        "agent": "claude",
        "platform_id": "qwen",
        "model": "qwen3-coder-plus",
        "prompt": "You are the QWEN seat in this roundtable.",
        "role": "participant",
        "enabled": true
      },
      {
        "id": "kimi-via-claude",
        "label": "KIMI via Claude",
        "agent": "claude",
        "platform_id": "kimi",
        "model": "kimi2",
        "prompt": "You are the KIMI seat in this roundtable.",
        "role": "researcher",
        "enabled": true
      },
      {
        "id": "native-codex",
        "label": "Native Codex",
        "agent": "codex",
        "model": "gpt-5.5",
        "prompt": "You are the Codex CLI seat in this roundtable.",
        "role": "participant",
        "enabled": true
      },
      {
        "id": "mimo-via-claude",
        "label": "MiMo Pro via Claude",
        "agent": "claude",
        "platform_id": "mimo",
        "model": "mimo-pro",
        "prompt": "You are the MiMo Pro seat in this roundtable.",
        "role": "participant",
        "enabled": true
      }
    ]
  }
}
```

`agent` 可为 `claude` 或 `codex`，省略时默认 `claude`。API provider（DeepSeek/GLM/QWEN/KIMI/MiMo Pro）通过 `agent: "claude"` 加 `platform_id` 表达。`model` 只应在用户明确选择模型或 API provider 需要模型时作为 per-run snapshot；不要把官方 CLI provider 的展示默认值强塞进启动参数。

`agent` can be `claude` or `codex`; omitted values default to `claude`. API providers (DeepSeek/GLM/QWEN/KIMI/MiMo Pro) are represented as `agent: "claude"` plus `platform_id`. `model` should become a per-run snapshot only when explicitly selected by the user or required by an API provider; do not inject display defaults into official CLI launches.

### Windows Native Toolchain Support

在 Windows 上，如果你从普通桌面窗口启动应用，Claude / Codex 子进程通常拿不到 Visual Studio Developer Prompt 里的 `cl`、`link`、Windows SDK 等环境。当前版本会在明确需要 native toolchain 的项目中，自动为本地 CLI 子进程补充 MSVC developer environment。Codex 如果通过 npm `.cmd` shim 安装，应用会在 Windows 上直接以 `node.exe + CLI js` 方式启动，避免对话时闪出临时 `cmd` 窗口。

On Windows, when the app is launched from the normal desktop environment, Claude / Codex child processes usually do not inherit the `cl`, `link`, Windows SDK, and related variables from a Visual Studio Developer Prompt. This version can automatically add an MSVC developer environment for local CLI child processes when the current project clearly needs native tooling. When Codex is installed through npm `.cmd` shims, the app launches `node.exe + the CLI js` directly on Windows to avoid transient `cmd` windows during chat.

**Auto-detection markers**（`auto` 模式下的项目根目录检测）：

- Tauri 项目：`src-tauri/` 目录、`binding.gyp`
- Node native 依赖：`package.json` 中含 `sharp`、`node-sass`、`bcrypt` 等
- Rust native 项目：`Cargo.toml` 含 `cc` build-dependency、`build.rs` 存在
- CMake / vcpkg：`CMakeLists.txt`、`vcpkg.json`
- Visual Studio / Qt：`*.sln`、`*.vcxproj`、`*.pro`、`*.pri`

检测仅限项目根目录，不递归子目录。若项目在子目录中有 `.sln` 等文件，请切换到 `always` 模式。

**Chat / Room 策略分离**：Chat 会话允许 MSVC 注入（按模式判断）；Room participant 会话由后端策略强制禁用注入（`MsvcPolicy::Disabled`），确保隔离。

**MSVC 状态徽标**：Chat 状态栏会在确认注入成功时显示 `MSVC` 徽标（amber 样式），鼠标悬停可查看 tooltip。Room 会话不显示此徽标。

模式：

- `auto`：默认，仅在保守 native project 信号下启用。
- `always`：强制为本地子进程启用。
- `off`：关闭自动注入。

Modes:

- `auto`: default, enabled only for conservative native-project signals.
- `always`: force injection for local child processes.
- `off`: disable automatic injection.

你可以在 Settings 中切换模式，并查看当前项目的 MSVC 环境状态。状态提示会说明是否已注入、无需注入、已关闭或需要安装 Visual Studio C++ build tools；不会展示完整环境变量。

You can switch the mode in Settings and view the MSVC environment status for the current project. The status tells you whether injection is active, unnecessary, disabled, or blocked by missing Visual Studio C++ build tools; it does not expose the full environment values.

## 当前限制 / Current Limitations

- Driver/Copilot 目前是 MVP：Copilot 只读行为通过 review prompt 约束，危险操作审批和硬权限限制仍在后续阶段。
- Research Room 支持研究分发、artifact 历史归档和标记式 Arena Memory 候选抽取；候选提升为永久项目 Arena Memory 仍在后续阶段。
- 余额查询仅覆盖 DeepSeek（API key 认证）和 MiMo（cookie 认证），其余 provider 暂无余额检查。
- Rust 单元测试受限于本地 VCRUNTIME140.dll 版本不匹配（VS 18 BuildTools vs System32），需用 `cargo check` 替代 `cargo test`。

Current limitations:

- Driver/Copilot is currently an MVP: copilot read-only behavior is guided by the review prompt, while dangerous-operation review and hard permission enforcement remain later work.
- Research Room can fan out research, keep artifact history, and extract marked Arena Memory candidates; promotion into permanent project Arena Memory remains later work.
- Balance queries only cover DeepSeek (API key auth) and MiMo (cookie auth); other providers lack balance checking.
- Rust unit tests are blocked by a local VCRUNTIME140.dll version mismatch (VS 18 BuildTools vs System32); use `cargo check` instead of `cargo test`.

## 开发 / Development

当前版本：**v1.1.2** · Current version: **v1.1.2**

```bash
npm install
npx svelte-kit sync
npm run dev
```

桌面端运行：

```bash
npm run tauri dev
```

打包（生成 `.exe` / `.msi` 安装包）：

```bash
npm run tauri build
```

产物路径：`src-tauri/target/release/bundle/`

版本号统一更新：

```bash
npm run release <version|patch|minor|major>
```

全量验证（lint + format + i18n + test + build + Rust check）：

```bash
npm run verify
```

单点验证：

```bash
npm run lint
npm run build
npm run rust:check
npm run i18n:check
npm test -- src/lib/stores/memo-store.test.ts
cargo test --manifest-path src-tauri/Cargo.toml storage::memos::tests:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml commands::memos::tests:: -- --nocapture
```

Desktop run:

```bash
npm run tauri dev
```

Packaging (produces `.exe` / `.msi` installers):

```bash
npm run tauri build
```

Output: `src-tauri/target/release/bundle/`

Version bumping across all config files:

```bash
npm run release <version|patch|minor|major>
```

Full verification (lint + format + i18n + test + build + Rust check):

```bash
npm run verify
```

Common checks:

```bash
npm run lint
npm run build
npm run rust:check
npm run i18n:check
npm test -- src/lib/stores/memo-store.test.ts
cargo test --manifest-path src-tauri/Cargo.toml storage::memos::tests:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml commands::memos::tests:: -- --nocapture
```

## 许可证 / License

本 remaster 仍应遵守上游项目及其依赖的许可证要求。合并上游代码或迁移 Claude Session Hub 设计时，应保留必要的 attribution 与 license 说明。

This remaster should continue to respect the license requirements of upstream projects and dependencies. When integrating upstream code or migrating Claude Session Hub ideas, keep the required attribution and license notes.
