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

### Rooms and Roundtable

Rooms 是多智能体协作的入口。你可以创建 Room、添加 Claude participant，并把 participant 关联到已有或新建的 Run。Room 删除不会删除对应 Run。

Rooms are the entry point for multi-agent collaboration. You can create a Room, add a Claude participant, and link that participant to an existing or newly created Run. Deleting a Room does not delete the linked Run.

当前适合用来：

- 把相关 Run 聚合到一个 Room 中查看。
- 为 Room 保存局部 memo。
- 在 Roundtable 时间线里向多个活跃 participant 分发同一个问题。
- 使用 `@debate` 让 participant 基于上一轮公开回复互相比较观点。
- 使用 `@summary @name` 指定一个 participant 总结公开 Room 历史。
- 使用 `@name message` 发送私有回合，私有内容不会出现在公开时间线中。

Currently useful for:

- Grouping related Runs in a Room.
- Keeping Room-local memos.
- Sending one prompt to multiple active participants in a Roundtable timeline.
- Using `@debate` to ask participants to compare positions based on previous public replies.
- Using `@summary @name` to ask one participant to summarize the public Room history.
- Using `@name message` for private turns that do not appear in the public timeline.

### Windows Native Toolchain Support

在 Windows 上，如果你从普通桌面窗口启动应用，Claude / Codex 子进程通常拿不到 Visual Studio Developer Prompt 里的 `cl`、`link`、Windows SDK 等环境。当前版本会在明确需要 native toolchain 的项目中，自动为本地 CLI 子进程补充 MSVC developer environment。

On Windows, when the app is launched from the normal desktop environment, Claude / Codex child processes usually do not inherit the `cl`, `link`, Windows SDK, and related variables from a Visual Studio Developer Prompt. This version can automatically add an MSVC developer environment for local CLI child processes when the current project clearly needs native tooling.

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

- Roundtable 当前依赖活跃的本地 Claude participant；更完整的 Codex / Gemini / 多 CLI 能力矩阵仍在后续阶段。
- Driver / Copilot 和 Research Room 尚未完成。
- 仍有部分上游基线检查需要后续清理。

Current limitations:

- Roundtable currently depends on active local Claude participants; the fuller Codex / Gemini / multi-CLI capability matrix is still planned for a later phase.
- Driver / Copilot and Research Room are not implemented yet.
- Some upstream baseline checks still need cleanup.

## 后续计划 / Roadmap

计划：

- Driver / Copilot Room。
- Research Room。
- Arena Memory：项目事实、决策、经验沉淀。
- Multi-CLI capability matrix。

Plan:

- Driver / Copilot Room.
- Research Room.
- Arena Memory for project facts, decisions, and lessons.
- Multi-CLI capability matrix.

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
