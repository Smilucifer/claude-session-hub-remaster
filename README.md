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
- `Room` 作为后续协作编排层，建立在 `Run` 之上。
- Memo、Room、Roundtable、Driver/Copilot、Research、Arena Memory 等能力分阶段落地。
- 不在早期破坏现有 `/chat` 路径。

Current architecture principles:

- `Run` remains the smallest execution unit.
- `Room` will become an orchestration layer above `Run`.
- Memo, Room, Roundtable, Driver/Copilot, Research, and Arena Memory will land in phases.
- The existing `/chat` path should not be disrupted early.

## 当前功能 / Current Features

### Memo

你可以在应用内维护全局备忘录和项目备忘录，用来保存长期偏好、项目约定、临时上下文和后续待办。项目备忘录会随当前项目目录切换，避免不同项目的上下文混在一起。

You can keep global and project-scoped memos inside the app for preferences, project conventions, temporary context, and follow-up notes. Project memos switch with the active project directory so context does not leak across projects.

支持：

- 添加、编辑、复制、删除、清空 memo。
- Global / Project scope 切换。
- 重启后保留 memo 内容。
- 通过 Command Palette 打开 Memo 面板。

Supported:

- Add, edit, copy, delete, and clear memos.
- Switch between Global and Project scope.
- Keep memo content after restart.
- Open the Memo panel from the Command Palette.

### Rooms, Roundtable, and Driver/Copilot

Rooms 是多智能体协作的入口。你可以创建 Room、添加 Claude / Codex / Gemini participant，并把 participant 关联到已有或新建的 Run。Room 删除不会删除对应 Run。Research Room 作为 Room kind 在同一入口中创建。

Rooms are the entry point for multi-agent collaboration. You can create a Room, add a Claude / Codex / Gemini participant, and link that participant to an existing or newly created Run. Deleting a Room does not delete the linked Run. Research Room is available as another Room kind from the same entry point.

当前适合用来：

- 把相关 Run 聚合到一个 Room 中查看。
- 为 Room 保存局部 memo。
- 在 Roundtable 时间线里向多个活跃 participant 分发同一个问题。
- 使用 `@debate` 让 participant 基于上一轮公开回复互相比较观点。
- 使用 `@summary @name` 指定一个 participant 总结公开 Room 历史。
- 使用 `@name message` 发送私有回合，私有内容不会出现在公开时间线中。
- 创建 Driver Room，让一个 Driver 通过 `/review` 向一个或多个 Copilot 请求只读审查。
- Driver review 会生成 room-local `.arena/context.md`、`.arena/state.md` 和 `.arena/memory`，用于稳定引用 room / run 上下文。
- `.arena` 文件是本地运行上下文镜像，可能包含 run references、memo 和最近的公开 preview；不要把它当成对外分享材料。
- 创建 Research Room，把一个研究主题分发给多个活跃 participant。
- Research Room 会把本轮结果汇总到 room-local `research/artifact.json` 结构化产物，向 `research/artifacts.jsonl` 追加 artifact 历史，并把 `[fact]`、`[decision]`、`[lesson]` 标记行展示为 Arena Memory 候选。

Currently useful for:

- Grouping related Runs in a Room.
- Keeping Room-local memos.
- Sending one prompt to multiple active participants in a Roundtable timeline.
- Using `@debate` to ask participants to compare positions based on previous public replies.
- Using `@summary @name` to ask one participant to summarize the public Room history.
- Using `@name message` for private turns that do not appear in the public timeline.
- Creating a Driver Room where one Driver asks one or more Copilots for read-only review with `/review`.
- Driver review generates room-local `.arena/context.md`, `.arena/state.md`, and `.arena/memory` files for stable room / run context references.
- `.arena` files are local runtime context mirrors and may include run references, memo text, and recent public previews; do not treat them as shareable artifacts.
- Creating a Research Room that fans out one research topic to multiple active participants.
- Research Room writes a room-local structured `research/artifact.json` artifact for the latest research turn, appends artifact history to `research/artifacts.jsonl`, and surfaces `[fact]`, `[decision]`, and `[lesson]` lines as Arena Memory candidates.

### Providers and CLI Authentication

普通聊天入口和会议室入口正在统一为五个 provider：Claude、Codex、Gemini、DeepSeek、GLM。Claude、Codex、Gemini 使用官方 CLI 认证；Codex 启动命令不使用 `exec`，并默认带 `--dangerously-bypass-approvals-and-sandbox`；Gemini 默认使用 yolo approval mode。DeepSeek 和 GLM 作为一等 provider 显示，但执行层复用 Claude Code compatible session，并通过 `platform_id` 注入对应 API 配置。

The chat and room entries are being unified around five providers: Claude, Codex, Gemini, DeepSeek, and GLM. Claude, Codex, and Gemini use official CLI authentication. Codex launches without `exec` and defaults to `--dangerously-bypass-approvals-and-sandbox`; Gemini defaults to yolo approval mode. DeepSeek and GLM are first-class providers in the UI, but execute through Claude Code compatible sessions with provider configuration injected by `platform_id`.

设置页正在简化为五个模型行，不再为 Codex/Gemini 提供自定义 API key、base URL 或保存登录方式。DeepSeek 只需要官方 API key；GLM 支持 API key、base URL 和 model。当前 Phase 7 仍在实现 Codex/Gemini 原生交互适配器；在完成前，不能把“CLI 已启动”视为和 Claude Code 一样的解析后会话渲染。

The settings page is being simplified to five model rows. Codex/Gemini no longer expose custom API key, base URL, or saved login method controls. DeepSeek needs only its official API key; GLM supports API key, base URL, and model. Phase 7 is still implementing the native Codex/Gemini interaction adapter; until that is complete, CLI launch alone is not considered Claude-like parsed conversation rendering.

Claude-compatible API profiles can still be represented under `user.cc_agent_profiles` in `~/.opencovibe/settings.json` for backward compatibility. DeepSeek/GLM should keep provider identity separate from the Claude execution agent:

为了向后兼容，Claude-compatible API profile 仍可保留在 `~/.opencovibe/settings.json` 的 `user.cc_agent_profiles` 下。DeepSeek/GLM 应保持 provider identity 和 Claude execution agent 分离：

```json
{
  "user": {
    "cc_agent_profiles": [
      {
        "id": "gemini-via-ccr",
        "label": "Gemini via CCR",
        "agent": "claude",
        "platform_id": "ccr",
        "model": "gemini-2.5-pro",
        "prompt": "You are the Gemini seat in this roundtable.",
        "role": "researcher",
        "enabled": true
      },
      {
        "id": "codex-via-ccswitch",
        "label": "Codex via CCSwitch",
        "agent": "claude",
        "platform_id": "ccswitch",
        "model": "gpt-5.5",
        "prompt": "You are the Codex seat in this roundtable.",
        "role": "participant",
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
        "id": "native-gemini",
        "label": "Native Gemini",
        "agent": "gemini",
        "model": "gemini-2.5-pro",
        "prompt": "You are the Gemini CLI seat in this roundtable.",
        "role": "researcher",
        "enabled": true
      }
    ]
  }
}
```

`agent` 可为 `claude`、`codex` 或 `gemini`，省略时默认 `claude`。DeepSeek/GLM 通过 `agent: "claude"` 加 `platform_id` 表达。`model` 只应在用户明确选择模型或 API provider 需要模型时作为 per-run snapshot；不要把官方 CLI provider 的展示默认值强塞进启动参数。

`agent` can be `claude`, `codex`, or `gemini`; omitted values default to `claude`. DeepSeek/GLM are represented as `agent: "claude"` plus `platform_id`. `model` should become a per-run snapshot only when explicitly selected by the user or required by an API provider; do not inject display defaults into official CLI launches.

### Windows Native Toolchain Support

在 Windows 上，如果你从普通桌面窗口启动应用，Claude / Codex 子进程通常拿不到 Visual Studio Developer Prompt 里的 `cl`、`link`、Windows SDK 等环境。当前版本会在明确需要 native toolchain 的项目中，自动为本地 CLI 子进程补充 MSVC developer environment。Codex / Gemini 如果通过 npm `.cmd` shim 安装，应用会在 Windows 上直接以 `node.exe + CLI js` 方式启动，避免对话时闪出临时 `cmd` 窗口。

On Windows, when the app is launched from the normal desktop environment, Claude / Codex child processes usually do not inherit the `cl`, `link`, Windows SDK, and related variables from a Visual Studio Developer Prompt. This version can automatically add an MSVC developer environment for local CLI child processes when the current project clearly needs native tooling. When Codex / Gemini are installed through npm `.cmd` shims, the app launches `node.exe + the CLI js` directly on Windows to avoid transient `cmd` windows during chat.

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

- Claude Code Room 参与者仍依赖活跃 stream session；Codex / Gemini 原生 interactive CLI 适配器仍在 Phase 7 中实现，完成前不能把其视为 Claude Code 同等的解析后会话渲染。
- 继续上次会话目前只展示后端已支持继续的 agent / run；Codex/Gemini resume-last 路径已开始接入，但仍需要原生适配器证明 prompt 输入、完成判定、停止和归档语义。
- Driver/Copilot 目前是 MVP：Copilot 只读行为通过 review prompt 约束，危险操作审批和硬权限限制仍在后续阶段。
- Research Room 支持研究分发、artifact 历史归档和标记式 Arena Memory 候选抽取；候选提升为永久项目 Arena Memory 仍在后续阶段。
- 仍有部分上游基线检查需要后续清理。

Current limitations:

- Claude Code Room participants still depend on active stream sessions; the Codex/Gemini native interactive adapter is still part of Phase 7 work, so it is not yet Claude-equivalent parsed conversation rendering.
- Continue-last-session only appears for agent / run combinations that the backend can actually resume; Codex/Gemini resume-last wiring has started, but the native adapter still must prove prompt input, completion detection, stop, and archival semantics.
- Driver/Copilot is currently an MVP: copilot read-only behavior is guided by the review prompt, while dangerous-operation review and hard permission enforcement remain later work.
- Research Room can fan out research, keep artifact history, and extract marked Arena Memory candidates; promotion into permanent project Arena Memory remains later work.
- Some upstream baseline checks still need cleanup.

## 后续计划 / Roadmap

计划：

- Arena Memory 候选提升：项目事实、决策、经验沉淀。
- Multi-CLI capability matrix。
- Codex/Gemini native adapter for Claude-like parsed conversation rendering.

Plan:

- Arena Memory promotion for project facts, decisions, and lessons.
- Multi-CLI capability matrix.
- Codex/Gemini native adapter for Claude-like parsed conversation rendering.

## 开发 / Development

```bash
npm install
npx svelte-kit sync
npm run dev
```

常用验证：

```bash
npm run lint
npm run build
npm run rust:check
npm run i18n:check
npm test -- src/lib/stores/memo-store.test.ts
cargo test --manifest-path src-tauri/Cargo.toml storage::memos::tests:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml commands::memos::tests:: -- --nocapture
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
