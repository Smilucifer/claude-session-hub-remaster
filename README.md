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

## Phase 1 已完成内容 / Phase 1 Completed Work

Phase 1 聚焦 Memo 与工作台基线，目标是在不触碰 session 编排的情况下先交付可见价值。

Phase 1 focuses on Memo and the workbench baseline, delivering visible value without changing session orchestration.

### Global / Project Memo

已实现：

- 全局备忘录：`data_dir()/memos/global.json`
- 项目备忘录：`data_dir()/projects/{project-hash}/memo.json`
- Tauri commands：`list_memos`、`add_memo`、`update_memo`、`delete_memo`、`clear_memos`
- 前端 Memo 面板：通过 Command Palette 打开
- Global / Project scope 切换
- 添加、编辑、复制、删除、清空
- 空文本拒绝
- 损坏 JSON 文件不会让应用崩溃
- i18n 文案：英文与简体中文
- 前端 stale response 防护
- Rust 同 scope 并发写锁，避免 read-modify-write 丢更新

Implemented:

- Global memo: `data_dir()/memos/global.json`
- Project memo: `data_dir()/projects/{project-hash}/memo.json`
- Tauri commands: `list_memos`, `add_memo`, `update_memo`, `delete_memo`, `clear_memos`
- Frontend Memo panel opened from the Command Palette
- Global / Project scope switching
- Add, edit, copy, delete, and clear actions
- Empty text rejection
- Corrupt JSON files do not crash the app
- i18n copy for English and Simplified Chinese
- Frontend stale response protection
- Rust per-scope write locking to avoid read-modify-write data loss

### 文档与决策 / Docs and Decisions

已补充：

- `thinking.md`
- `docs/ADR-001-room-over-run.md`
- `docs/migration-decisions.md`
- `docs/implementation-roadmap.md`
- `docs/phase-1-memo-implementation-plan.md`

Added:

- `thinking.md`
- `docs/ADR-001-room-over-run.md`
- `docs/migration-decisions.md`
- `docs/implementation-roadmap.md`
- `docs/phase-1-memo-implementation-plan.md`

## 验证状态 / Verification Status

已通过：

- `npm run lint`
- `npm run build`
- `npm run rust:check`
- `npm run i18n:check`
- `npm test -- src/lib/stores/memo-store.test.ts`
- `cargo test --manifest-path src-tauri/Cargo.toml storage::memos::tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml commands::memos::tests:: -- --nocapture`

Passed:

- `npm run lint`
- `npm run build`
- `npm run rust:check`
- `npm run i18n:check`
- `npm test -- src/lib/stores/memo-store.test.ts`
- `cargo test --manifest-path src-tauri/Cargo.toml storage::memos::tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml commands::memos::tests:: -- --nocapture`

已知基线问题：

- `npm run check` 在 OpenCovibe `v0.1.55` 干净基线上已有 `105 errors / 74 warnings`。
- `npm test` 全量测试中，`src/lib/i18n/__tests__/i18n.test.ts` 仍有默认 locale 断言与当前 `zh-CN` 默认行为不一致。
- 上述问题不是 Phase 1 Memo 引入，但会影响后续把 `npm run verify` 作为强门禁。

Known baseline issues:

- `npm run check` already reports `105 errors / 74 warnings` on a clean OpenCovibe `v0.1.55` baseline.
- The full `npm test` suite still has default-locale assertion failures in `src/lib/i18n/__tests__/i18n.test.ts`, because the tests expect English while the current default behavior is `zh-CN`.
- These issues were not introduced by Phase 1 Memo, but they weaken `npm run verify` as a future hard gate.

## 后续计划 / Roadmap

### Phase 1.x: Baseline Cleanup

建议在进入 Room 前先处理：

- 修复 `svelte-check` 基线错误。
- 修复 i18n 默认 locale 测试。
- 将 `npm run verify` 恢复为可信门禁。
- 补 Windows Doctor 对 MSVC 工具链的检测。

Before Room work, the project should clean up the baseline:

- Fix existing `svelte-check` errors.
- Fix i18n default-locale tests.
- Restore `npm run verify` as a trustworthy gate.
- Add Windows Doctor checks for the MSVC toolchain.

### Phase 2: Room Foundation

计划：

- 新增 Room 数据模型。
- Room 建立在现有 `Run` 之上，不替代 `SessionActor` / `turn_engine`。
- 增加 room-local memo。
- 提供最小 Room UI 与事件流。

Plan:

- Add the Room data model.
- Build Room above existing `Run`, without replacing `SessionActor` or `turn_engine`.
- Add room-local memo.
- Provide a minimal Room UI and event stream.

### Phase 3: Collaboration Protocols

计划：

- Roundtable fanout / debate / summary
- Driver / Copilot
- Private side conversations
- Multi-CLI participant abstraction

Plan:

- Roundtable fanout / debate / summary
- Driver / Copilot
- Private side conversations
- Multi-CLI participant abstraction

### Phase 4: Research and Arena Memory

计划：

- Research Room
- Arena Memory：项目事实、决策、经验沉淀
- 从 Room / Run 中提取可复用上下文

Plan:

- Research Room
- Arena Memory for project facts, decisions, and lessons
- Reusable context extraction from Room / Run artifacts

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
