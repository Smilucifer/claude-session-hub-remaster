# Claude Session Hub Remaster / Claude Session Hub 重制版

> 本文件是中文入口；英文说明见 [README.md](README.md)。

## 致谢

本项目基于 [AnyiWang/OpenCovibe](https://github.com/AnyiWang/OpenCovibe) 的本地优先桌面架构与 UI 基础继续演进，并吸收了 [TianLin0509/claude-session-hub](https://github.com/TianLin0509/claude-session-hub) 在会议室、备忘录、协作协议和多 CLI 工作流上的产品启发。感谢这两个项目对本 remaster 方向的帮助。

## 项目定位

Claude Session Hub Remaster 的目标不是重写 OpenCovibe，也不是直接搬运 Claude Session Hub。它以 OpenCovibe 当前的 Tauri + Svelte + Rust 架构为主体，逐步叠加 Claude Session Hub 中有差异化价值的协作能力。

当前架构原则：

- `Run` 继续作为最小执行单元。
- `Room` 作为协作编排层，建立在 `Run` 之上。
- Memo、Room、Roundtable、Driver/Copilot、Research、Arena Memory 等能力分阶段落地。
- 不在早期破坏现有 `/chat` 路径。

## 当前功能

### Memo

你可以在应用内维护全局备忘录和项目备忘录，用来保存长期偏好、项目约定、临时上下文和后续待办。项目备忘录会随当前项目目录切换，避免不同项目的上下文混在一起。

支持：

- 添加、编辑、复制、删除、清空 memo。
- Global / Project scope 切换。
- 重启后保留 memo 内容。
- 通过 Command Palette 打开 Memo 面板。

### Rooms、Roundtable 和 Driver/Copilot

Rooms 是多智能体协作的入口。你可以创建 Room、添加 Claude / Codex participant，并把 participant 关联到已有或新建的 Run。Room 删除不会删除对应 Run。Research Room 作为 Room kind 在同一入口中创建。Room participant 会话在侧边栏中以虚拟"会议室"文件夹单独分组，与项目会话分离。

当前适合用来：

- 把相关 Run 聚合到一个 Room 中查看。
- 为 Room 保存局部 memo。
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

### Codex 和接入方式

普通聊天入口可以从底部 agent selector 或 Command Palette 切换到 Claude、Codex。Claude 使用 stream session；Codex 走原生 PTY pipe mode。Codex 启动页与 Claude 保持一致，只提供继续可用会话和选择接入方式的入口，不再展示示例 prompt。Codex 的输出默认按聊天时间线渲染，历史回放会保留多轮 `user -> assistant` 顺序，而不是显示为终端 dump。

Settings 为 CC / Codex 提供独立连接配置页。Codex 使用与 CC 一致的 CLI Auth / App API Key 卡片模式，同时保留各自的原生命令设置和 No-review mode。保存的接入方式可以使用 CLI 认证或 App 管理的 API key；普通聊天和 Room 创建都可以从已保存接入方式启动。Room 项目路径通过文件夹选择器选择，并由三个固定 Roundtable seat 共享。

会议室可以混用 Claude Code stream session 参与者，以及原生 Codex pipe-exec 参与者。Claude Code profile 仍可通过 `platform_id` 接不同 API / 不同模型。把 profile 写到 `~/.opencovibe/settings.json` 的 `user.cc_agent_profiles`，不要覆盖文件里的其他字段。`agent` 可为 `claude` 或 `codex`，省略时默认 `claude`。`connection_profile_id` 可绑定一个保存的接入方式；未指定时使用对应 agent 的默认连接设置。

### Windows Native Toolchain Support

在 Windows 上，如果你从普通桌面窗口启动应用，Claude / Codex 子进程通常拿不到 Visual Studio Developer Prompt 里的 `cl`、`link`、Windows SDK 等环境。当前版本会在明确需要 native toolchain 的项目中，自动为本地 CLI 子进程补充 MSVC developer environment。Codex 如果通过 npm `.cmd` shim 安装，应用会在 Windows 上直接以 `node.exe + CLI js` 方式启动，避免对话时闪出临时 `cmd` 窗口。

模式：

- `auto`：默认，仅在保守 native project 信号下启用。
- `always`：强制为本地子进程启用。
- `off`：关闭自动注入。

你可以在 Settings 中切换模式，并查看当前项目的 MSVC 环境状态。状态提示会说明是否已注入、无需注入、已关闭或需要安装 Visual Studio C++ build tools；不会展示完整环境变量。

## 当前限制

- Claude Code Room 参与者仍依赖活跃 stream session；Codex 参与者以单轮 native CLI pipe-exec 方式运行，不是长驻 stream actor。
- 继续上次会话目前只展示后端已支持继续的 agent / run；Codex thread 记录已保存，但尚未接入可恢复的交互式 stream actor。
- Driver/Copilot 目前是 MVP：Copilot 只读行为通过 review prompt 约束，危险操作审批和硬权限限制仍在后续阶段。
- Research Room 支持研究分发、artifact 历史归档和标记式 Arena Memory 候选抽取；候选提升为永久项目 Arena Memory 仍在后续阶段。
- 仍有部分上游基线检查需要后续清理。

## 后续计划

已完成 Phase 8：Gemini 彻底移除、Stepper mini-map 逐轮回放、`@DisplayName` SingleTarget 公开点名、Room sidebar 虚拟分组、seat prompt 英文约束、context events 验证。1214 前端测试通过，cargo check 干净。

- Arena Memory 候选提升：项目事实、决策、经验沉淀。
- 强化 Codex：继续优化 Codex PTY 执行路径、resume-last 语义。
- Roundtable Debate/Summary 交互完善。

## 开发

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

## 许可证

本 remaster 仍应遵守上游项目及其依赖的许可证要求。合并上游代码或迁移 Claude Session Hub 设计时，应保留必要的 attribution 与 license 说明。
