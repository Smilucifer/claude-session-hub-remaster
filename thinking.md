# OpenCovibe + Claude Session Hub 融合思考

日期：2026-04-30
范围：阅读 `D:\ClaudeWorkspace\Code\OpenCovibe` 与 `D:\ClaudeWorkspace\Code\claude-session-hub` 后，结合当前讨论收敛出的融合结论。

更新记录：
- 2026-04-30 初版
- 2026-04-30 补充：Research Mode 漏判修正、RoomTurn 与 turn_engine.rs 关系、AgentAdapter trait 前置到 Phase 2 末尾、Phase 1 Memo 切两步、Hub 数据迁移策略、Hub 能力迁移决策表、feature flag 硬要求
- 2026-04-30 补充：Windows 开发者环境支持（Doctor MSVC 检测 + Phase 1.c + session spawn env 注入），对应 Hub 反复踩到的 node-pty / cl.exe / link.exe 工具链坑
- 2026-04-30 拆分正式文档：`docs/ADR-001-room-over-run.md`、`docs/migration-decisions.md`、`docs/implementation-roadmap.md`
- 2026-04-30 收敛：清空 migration Pending，ADR-001 推进为 Accepted，新增 Phase 1 Memo 实施计划
- 2026-04-30 进度更新：Phase 1 Memo foundation 已合并；Phase 2 Room as Run Group + AgentAdapter boundary 已合并并通过 release 包冒烟；下一步进入 Phase 2.x Windows MSVC Environment Injection。
- 2026-05-05 进度更新：Phase 5.5 Native CLI Chat Parity 完成。Codex / Gemini pipe-exec 输出进入正常 chat timeline，历史回放保留多轮 user/assistant 顺序，Settings 连接配置与 CC 的 CLI Auth / App API Key 模式对齐，Windows npm `.cmd` shim 改为静默 `node.exe + CLI js` 启动。

拆分文档：
- [ADR-001: Room as an Orchestration Layer over Run](docs/ADR-001-room-over-run.md)
- [Hub Capability Migration Decisions](docs/migration-decisions.md)
- [OpenCovibe Remaster Implementation Roadmap](docs/implementation-roadmap.md)
- [Phase 1 Memo Implementation Plan](docs/phase-1-memo-implementation-plan.md)
- [Phase 5.5 Native CLI Chat Parity Implementation Plan](docs/phase-5.5-native-cli-chat-parity-implementation-plan.md)

## 一句话结论

建议以 OpenCovibe 为主架构，不迁移 claude-session-hub 的 Electron / node-pty 壳；把 Hub 的独特价值抽象成 OpenCovibe 里的新协作层，并明确把它定义为 **高于 Run 的编排工作区**：

> `Room / Participant / Turn / Memo / Arena Memory`，挂在现有 `Run / BusEvent / SessionActor / Storage / WebServer` 之上。

OpenCovibe 已经有更好的长期底座：Tauri v2、Rust actor、事件总线、本地 run 存储、Web 远程访问、Svelte 组件体系、CLI 配置和历史导入。Hub 的优势不在底层技术，而在几套非常有产品感的协作协议：会议室、圆桌、主驾/副驾、备忘录、跨 CLI 参与者、项目级记忆。

进一步收敛后的判断是：

- **架构基线采用 Room 协作层方案**，而不是先做零散 UI 拼装。
- **产品节奏上优先落地 memo 与轻量工作台增强**，让迁移第一阶段就有可感知收益。
- **会议室能力要以 OpenCovibe 原生模型重做**，而不是直接移植 `core/meeting-room.js` 一类 JS 模块。

## 我看到的现状

### OpenCovibe 的优点

- 架构层次清楚：`src-tauri/src/lib.rs` 注册 command、actor map、event writer、web server；`commands/*` 负责 IPC；`agent/*` 管 CLI 生命周期；`storage/*` 管本地持久化。
- Claude Code 已经是长连接 session actor：`agent/session_actor.rs` 单 actor 拥有 stdin/stdout、协议状态、turn queue、permission/hook/elicitation、Ralph loop。
- 数据模型已经够接协作层：`models.rs` 有 `RunMeta`、`TaskRun`、`BusEvent`、`RunStatus`、`ExecutionPath`、`ConversationRef`、team 类型、usage 类型。
- 前端已经从“终端管理器”升级成“AI 编程工作台”：chat timeline、inline tool cards、permission panel、MCP、history、usage、memory、plugins、teams。
- 远程访问做得比 Hub 更现代：Rust `web_server` 用 axum + websocket + cookie/token；比 Hub 的 Express mobile server 更适合作为统一远程入口。
- 本地优先路径清晰：`~/.opencovibe/runs/{run-id}/meta.json/events.jsonl/artifacts.json`，无数据库依赖，便于新增 room 文件夹。

### claude-session-hub 的独特价值

- 会议室不是简单分屏。`core/meeting-room.js` 有 room 元数据、最多 3 个 subSessions、layout/focus/sendTarget、timeline、per-session cursor、惰性加载和增量上下文。
- 圆桌是一套完整协议。`core/roundtable-orchestrator.js` 支持 `fanout`、`debate`、`summary` 三种轮次，并把每轮持久化到 `arena-prompts/<meetingId>-turn-N.json`。
- 通用圆桌有清晰交互语法。`core/general-roundtable-mode.js` 定义默认提问、`@debate`、`@summary @who`、`@who` 私聊，且通过 prompt 注入让不同 CLI 理解同一协议。
- 私聊被单独持久化。`core/general-roundtable-private-store.js` 把 `@claude/@gemini/@codex` 私聊从圆桌公开历史里分离出来，这个设计很有用。
- 主驾/副驾模式有明确工程协作分工。`core/driver-mode.js` 让 Claude 做 Driver，Gemini/Codex 做 Copilot，带 `request_review`、`request_danger_review`、`.arena/context.md`、`.arena/state.md` 和记忆协议。
- Arena Memory 很值得保留。`core/arena-memory/store.js` 把项目级 facts 写到 `.arena/memory/shared/facts.md`，把 lesson/decision 进 `episodes.jsonl`，避免污染各 CLI 的个人长期记忆。
- 备忘录虽小但产品感强。`renderer/renderer.js` 的 memo 是全局轻量 scratchpad，独立于会话，适合临时想法、待问问题、复制片段。
- 多 CLI 参与者理念明确。`core/session-manager.js` 支持 Claude、Gemini、Codex、DeepSeek、PowerShell，且在会议室里按参与者注入不同 prompt / env / resume 参数。
- Research Mode 是与会议室、圆桌、Driver 同级的协作模式。`core/research-mode.js` + `core/research-mcp-server.js` 把多 CLI 协作搜资料、归档结论的过程结构化，初版 thinking.md 漏判，必须纳入协议迁移清单。

## 当前收敛后的核心判断

### 1. OpenCovibe 继续做主干，Hub 只迁移“产品协议”

需要保留 OpenCovibe 现有的：

- Tauri + Rust backend
- SessionActor / Run / BusEvent 主模型
- 本地 run storage
- Web server 远程访问
- Svelte UI 与现有工作台能力

不建议迁移的内容：

- Electron 主壳
- node-pty 多终端作为默认执行路径
- Hub 的 renderer 直接控制会话生命周期的方式

迁移对象应该是：会议室协议、圆桌轮次、Driver/Copilot 协作、Memo、Arena Memory、多 CLI 参与者理念。

### 2. Room 必须是 Run 之上的编排层

这是当前最关键的架构结论。

- `Run` 仍然是最小执行单元。
- `Room` 是组织多个 run 的协作工作区。
- `RoomParticipant` 引用 `run_id`，而不是复制会话状态。
- `RoomTurn` 是协作轮次，不等于原始聊天消息。

这样做的好处：

- 普通单会话 chat 不被破坏。
- Room 删除、归档、恢复可以独立于 run 生命周期。
- Room timeline 可以保存协作摘要与引用，不需要和 run events 混成一锅。

另一个必须明确的关系：OpenCovibe 已经有 `agent/turn_engine.rs`，是 single-run 的轮次状态机。Room **不要替换**它，而是把每个参与者的回合内部仍然交给 turn_engine 跑；RoomTurn 只负责跨 run 的编排（fanout / debate / summary / private 派发、收集、超时、完成判定）。这两层在 ADR 里必须显式区分，避免后续返工。

### 3. 会议室能力应当重写为 OpenCovibe 原生模型

`claude-session-hub/core/meeting-room.js`、`roundtable-orchestrator.js`、`driver-mode.js` 这些文件很有参考价值，但不应原样迁移。

正确方向是：

- Rust backend 负责 room lifecycle、participant membership、turn orchestration、memo persistence、arena memory。
- commands 层暴露 room IPC。
- BusEvent 扩展 room 级事件。
- Svelte 前端消费统一事件流渲染 UI。

换句话说，迁移的是能力定义和交互协议，不是原始实现代码。

### 4. `/rooms` 与 `/teams` 必须分离

- `/teams` 保持 Claude Code 原生 team 的只读观察定位。
- `/rooms` 是 OpenCovibe 主动编排的多 CLI 协作空间。

这两者可以共享某些 UI 密度和事件可视化经验，但不应是同一对象模型。

### 5. Memo 与 Arena Memory 需要分层，而不是只做一个便签

建议明确区分：

- `global memo`：跨项目临时想法
- `project memo`：项目上下文与待办碎片
- `room memo`：某次会议/圆桌过程中的问题、结论、摘录
- `arena memory`：项目级事实、决策、lesson learned，与个人长期记忆分离

这样既能保留 Hub 的“顺手记一下”体验，也能让 OpenCovibe 继续保持本地优先和结构化。

## 建议的新架构

### 新增后端模块

建议在 OpenCovibe 后端加：

- `src-tauri/src/room/mod.rs`
- `src-tauri/src/room/models.rs`
- `src-tauri/src/room/orchestrator.rs`
- `src-tauri/src/storage/rooms.rs`
- `src-tauri/src/commands/rooms.rs`
- `src-tauri/src/storage/memos.rs`
- `src-tauri/src/commands/memos.rs`
- 可选：`src-tauri/src/room/arena_memory.rs`

核心类型大致是：

```rust
Room {
  id,
  title,
  kind: Free | Roundtable | Driver | Research,
  project_cwd,
  participants: Vec<RoomParticipant>,
  active_round,
  created_at,
  updated_at,
}

RoomParticipant {
  id,
  run_id,
  agent: "claude" | "codex" | "gemini" | "...",
  role: Driver | Copilot | Peer,
  label,
  status,
  capabilities,
}

RoomTurn {
  idx,
  mode: Fanout | Debate | Summary | Private,
  user_input,
  targets,
  responses,
  started_at,
  completed_at,
}
```

### 存储建议

```text
~/.opencovibe/
├── runs/
├── memos/
│   └── global.json
├── projects/
│   └── {project-hash}/
│       └── memo.json
└── rooms/
    └── {room-id}/
        ├── meta.json
        ├── timeline.jsonl
        ├── private.json
        ├── memo.json
        ├── prompts/
        └── arena/
            ├── state.md
            ├── context.md
            └── memory/
```

### 新增事件

在 `BusEvent` 里加 room 级事件，让桌面和 Web 端一致刷新：

- `RoomCreated`
- `RoomUpdated`
- `RoomParticipantAdded`
- `RoomTurnStarted`
- `RoomParticipantPartial`
- `RoomParticipantCompleted`
- `RoomTurnCompleted`
- `RoomPrivateMessage`
- `RoomMemoUpdated`
- `ArenaMemoryUpdated`

这些事件不应该重复写聊天消息正文的完整历史；正文落在 `rooms/{id}/timeline.jsonl`，事件主要驱动实时 UI。

### 新增前端页面

建议新建 `/rooms`，不要把功能塞进现有 `/teams`。

UI 第一版建议：

- 顶部：成员条、房间状态、快捷操作
- 左侧：轮次列表 / 任务列表 / room memo 快览
- 中央：讨论时间线 / 当前响应 / 私聊提示
- 右侧：上下文、Arena Memory、project memo

不要回到 Hub 的“三终端墙”默认态。raw terminal 只作为按需展开视图，而不是主视图。

## 功能分期

### Phase 1：Memo + 轻量工作台增强

这是当前最适合先落地的阶段，因为收益高、风险低，而且能让迁移从第一步就可见。

#### Memo

按"先 global、再 project 维度"切两小步，避免 project hash + 跟随 cwd 的复杂度拖累首日节奏：

**Phase 1.a — global memo（D1-D2）**

- 后端加 `storage/memos.rs`（仅 `~/.opencovibe/memos/global.json`）和 `commands/memos.rs`（list / add / update / delete / clear）
- 前端入口先做 command palette + 浮动面板，**不做侧栏**
- 验收：D2 末尾能写、能读、重启不丢

**Phase 1.b — project memo（D3-D4）**

- 复用 OpenCovibe 现有的 project scoping 拿到 project hash
- 数据放 `~/.opencovibe/projects/{hash}/memo.json`
- 切换项目时 memo 跟随刷新
- 验收：D4 末尾两层 memo 互不串扰

room memo（Phase 2 配套）和 arena memory（Phase 4 配套）不在 Phase 1 范围内。

**Phase 1.c — Doctor 增强：MSVC 工具链检测（D5）**

- 扩展 `commands/diagnostics.rs` 加 MSVC env 检查（`VSCMD_VER` / `cl.exe` 在 PATH / `vswhere` 能否定位 VS2022）
- Doctor 报告新增"Windows 编译环境"卡片，未达标时给出"在 x64 Native Tools Command Prompt 里重启"的指引
- 验收：普通 PowerShell 启动 Doctor 应明确告警；在 VS2022 开发者命令行启动应通过

Session spawn 层的 env 自动注入独立成 Phase 2.x「Spawn Environment Resolver」通用层，不绑死 Room——普通 `/chat` 启动也能受益。详细设计见 `docs/implementation-roadmap.md` Phase 2.x 与下方"Windows 开发者环境支持"一节。

#### 轻量工作台增强

在不改变主架构的前提下，可以顺手吸收少量 session-hub 的“顺手感”：

- 会话列表的最后消息预览
- 更明显的未读提示
- pinned session / 快速回到最近活跃会话
- 恢复体验优化

这一阶段的原则是：**增强现有单人工作台体验，但不让 UI 小修小补抢走 Room 主线。**

### Phase 2：Room 作为 Run 分组

先不做自动编排，只做 room 元数据和参与者：

- 创建 room
- 从 room 里启动 Claude run
- room participant 引用 `run_id`
- UI 展示成员状态、最近输出、room memo

第一版允许只支持 Claude 参与者，这样可以最大程度复用现有 `start_session`、`send_session_message`、`get_run_events` 和 actor 生命周期。

**Phase 2 末尾的硬性收口：AgentAdapter trait**

虽然 Phase 2 只跑 Claude participant，但必须在结束前把现有 `claude_protocol / codex_parser / pipe_parser` 收口到统一 trait：

```rust
trait AgentAdapter {
    async fn wait_turn_complete(&mut self) -> Result<TurnOutcome>;
    async fn stream_message(&mut self, msg: &str) -> Result<()>;
    fn inject_prompt(&mut self, scope: PromptScope, body: &str) -> Result<()>;
    fn capabilities(&self) -> AgentCapabilities;
}
```

理由：Phase 3 通用圆桌的 fanout / debate 需要稳定的 `wait_turn_complete` 抽象——没有它就退回到 PTY buffer/marker 那条路，正是本文档不想踩的坑。把这一步前置到 Phase 2 末尾而不是 Phase 5，是因为 Phase 3 第一行代码就需要它。

`AgentCapabilities` 字段表仍按 Phase 5 计划展开，但 trait 本身必须 Phase 2 完成。

### Phase 3：通用圆桌

**前提**：Phase 2 末尾的 `AgentAdapter` trait 已落地。圆桌的 fanout / debate / summary 全部基于 trait 抽象，禁止直接调用 CLI-specific 协议。

移植 Hub 的通用圆桌协议，但建立在 OpenCovibe 的 turn / state / event 之上：

- 普通输入 -> fanout 给所有 peer
- `@debate` -> 取上一轮其他人观点，定向发给每个参与者
- `@summary @claude` -> 取 room 历史，发给指定 summarizer
- `@codex ...` -> private turn，不入公开轮次

OpenCovibe 不应该依赖 PTY 输出 buffer 来判断完成。优先复用 `RunState`、`message_complete`、actor turn 状态；没有结构化协议的 CLI 则通过 adapter 定义统一的 `wait_turn_complete`。

### Phase 3.5：Research Room

Hub 的 `core/research-mode.js` + `core/research-mcp-server.js` 是与 Roundtable / Driver 同级的协作模式：研究主题 + 多 CLI 协作搜资料 + 结构化归档。落到 OpenCovibe：

- Room kind 增加 `Research`
- 复用 Phase 3 的 fanout 把研究子任务派给不同 participant
- 复用 Phase 4 的 review 让 Claude 做归档与 fact 提取
- 输出落到 `rooms/{id}/research/` 下，沉淀为 arena memory candidate

为什么列为 3.5 而不是新增 Phase 6：它是 Phase 3 + Phase 4 能力的复用组合，不是新机制。等 Driver Mode 上线后顺手实现，避免拉高 Phase 编号造成"功能未完"错觉。

### Phase 4：Driver / Copilot Room

把 Hub 的 Driver Mode 产品化：

- Driver 是主要执行参与者
- Copilot 默认只读审查
- review request 先做成 OpenCovibe 内部 command 或 slash command
- 危险操作审查等后续再接 permission panel / hook / tool cards
- `.arena/context.md`、`.arena/state.md`、`.arena/memory` 延续 Hub 的项目级记忆思想

### Phase 5：多 CLI 能力表

多 CLI 不要一开始就铺满，而是等 Room 主模型稳定后，再把差异显式化：

```ts
AgentCapabilities = {
  streamSession: boolean,
  pipeExec: boolean,
  interactivePty: boolean,
  resume: "session_id" | "latest" | "none",
  promptInjection: "system_prompt" | "append_file" | "instruction_file" | "env",
  mcpConfig: boolean,
  contextUsage: boolean,
  permissionProtocol: boolean,
}
```

然后 room orchestrator 只问能力，不知道具体命令行参数。

## 关键取舍

### 不建议直接搬 Hub 的 node-pty 多终端

Hub 的 PTY 方案适合 Windows 本地终端管理，但 OpenCovibe 已经有更强的 stream-json 事件体验。搬 node-pty 会引入：

- 跨平台 PTY 兼容成本
- raw terminal UI 与 inline tool card 双轨并存
- 完成状态识别困难，仍要靠 buffer/tap/marker

更好的方式是：Claude 继续 session actor；Codex/Gemini 尽量走 JSON/pipe/adapter；仅在某些 CLI 没有结构化协议时，提供 fallback terminal participant。

### 会议室不要等同于 Teams

OpenCovibe 的 `teams` 读取 `~/.claude/teams/` 和 `~/.claude/tasks/`，是 Claude Code 原生 team 的观察窗。Hub 的会议室是 OpenCovibe 主动编排的多 CLI 协作空间。两者可以互相引用，但不应合并成一个概念。

### Memo 不要只做全局

Hub 的 memo 是全局临时纸条；OpenCovibe 更适合做三层：

- global memo
- project memo
- room memo

后续 room summary 可以把 room memo 自动带入 summarizer prompt。

### Arena Memory 应该区分个人记忆和项目记忆

个人偏好交给 Claude/Gemini/Codex 自己的 memory；项目事实、决策、教训进入 `.arena/memory`。OpenCovibe 可以把它做成可视化 memory lane，并支持从 review 输出里提取 `[fact] [lesson] [decision]`。

### Hub 数据不做反向兼容

老 Hub 用户的会议历史、arena memory、memo 在新 OpenCovibe 不做自动导入。理由：

- Hub 数据散落在 `~/.claude-session-hub/`、`<project>/.arena/`、`<project>/.arena-prompts/` 多处，schema 与 OpenCovibe 不同
- 自动迁移脚本要兼容多版本 Hub `state.json`，长期维护成本高
- 用户基数小，更适合一次性 markdown 导出 + 手动重建

折中方案：提供 `opencovibe migrate-hub --export-only` 子命令，把 Hub 数据 dump 成 markdown，让用户自行 paste 回新工作台。这条要写进 ADR 的"非目标"部分，避免 Phase 4 时被人当 bug 提。

## Hub 能力迁移决策表

为避免 thinking.md 漏判 Research Mode 那种情况再次发生，在动手前必须把 Hub 的 `core/` 全部条目过一遍。先按四象限初判，落到 `claude-session-hub-remaster/docs/migration-decisions.md` 维护：

| 类别 | 含义 | 当前已识别条目 |
|------|------|----------------|
| **迁移** | 协议/产品体验值钱，按 OpenCovibe 原生模型重写 | meeting-room.js、roundtable-orchestrator.js、driver-mode.js、arena-memory/、general-roundtable-mode.js、general-roundtable-private-store.js、research-mode.js、data-dir.js |
| **保留参考** | 实现思路有价值但不直接迁，写进 ADR 或 backlog | mobile-protocol.js（PWA + Tailscale 设计）、deep-summary-service.js、summary-providers/、meeting-store.js、deep-summary-config.js、summary-engine.js、summary-parser.js、summary-prompt.js、usage-filter.js |
| **抛弃** | OpenCovibe 已有更现代实现或无需保留 | mobile-server.js（用 web_server）、mobile-routes.js、mobile-auth.js、transcript-tap.js（不走 PTY）、ansi-utils.js（用 xterm.js）、state-store.js（用 storage/）、session-archive.js、lindang-bridge.js |

Pending 已在 2026-04-30 清零；正式分类以 [`docs/migration-decisions.md`](docs/migration-decisions.md) 为准。后续如果发现新的 Hub 文件，直接归入 Migrate / Reference / Drop，不再保留无 owner 的待定池。

## Windows 开发者环境支持（Doctor 扩展）

Hub 的 `claude-session-hub/CLAUDE.md` 反复出现的"node-pty 编译失败 / EBUSY / electron-builder rebuild 半坏"痛点，根因是 Windows 上的 native 模块和 Rust/Tauri 编译都依赖 MSVC 工具链：必须在 "x64 Native Tools Command Prompt for VS 2022" 或 "Developer PowerShell for VS 2022" 环境里才有 `cl.exe`、`link.exe`、Windows SDK 路径等。普通 PowerShell / cmd 启动的 Claude Code 子进程继承不到这些 env，npm install / cargo build / `npm run tauri dev` 必然失败。

OpenCovibe 当前问题：

- README 明确声明 Windows 未充分测试
- 现有 `commands/diagnostics.rs`（Doctor）只检查 CLI / 平台 / SSH / 代理，不检查编译工具链
- 启动 Claude / Codex / Gemini session 时直接继承当前 shell 的 env，普通 PowerShell 没有 VS env，编译类命令必失

建议新增能力（落到 Doctor + session spawn 两层）：

**Doctor 检测层（Phase 1.c）**

- 检查环境变量 `VSCMD_VER` / `VCINSTALLDIR` / `VSINSTALLDIR` 是否已设置
- 检查 `cl.exe` / `link.exe` 是否在 PATH
- 通过 `vswhere.exe`（标准位置 `%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe`）定位已安装的 VS 2022 实例
- 不满足时给出"未在 VS2022 开发者命令行启动"警告 + 修复指引

**Session spawn 层（Phase 2 配套）**

- 启动 Claude / Codex / Gemini session 前，如果检测到当前项目需要编译（存在 `Cargo.toml` / `src-tauri/` / 依赖 native 模块的 `package.json`），自动通过 `vswhere` + `vsdevcmd.bat` 派生 MSVC env 注入到子进程
- 提供 settings 选项 `windows.force_msvc_env: true | false | "auto"`，"auto" 模式按项目类型决策
- 给一个"重新在 VS2022 开发者命令行启动 OpenCovibe"的快捷方式生成器，作为兜底（生成 `.lnk` 指向 `vsdevcmd.bat && opencovibe.exe`）

**最短人工操作步骤**（作为 Doctor 警告里的 fallback 文案，也写进 README Windows 一节）：

1. 开始菜单 → Visual Studio 2022 → "x64 Native Tools Command Prompt for VS 2022"（或 "Developer PowerShell for VS 2022"）
2. `cd D:\ClaudeWorkspace\Code\OpenCovibe`（或对应项目目录）
3. 在该窗口里启动 Claude / OpenCovibe / `npm run tauri dev` / `cargo build` 等需要 MSVC 的命令

这条能力对 Hub 迁移用户尤其重要——他们最熟悉的就是 node-pty 编译坑，OpenCovibe 把这层兜底做掉，迁移摩擦会小很多。同时 Doctor 里"VS2022 缺失"的卡片本身就是产品差异化卖点：macOS 工具不会主动告诉你"你环境不对"。

## 当前推荐的近期执行顺序

1. 先写 `memos` 数据模型和 storage，快速交付可见收益。
2. 同步规划 `rooms` 数据模型和 storage，但第一步不急着铺满 UI。
3. 在现有 `/chat` 不动的前提下，新建 `/rooms` 入口。
4. Room 第一版先只支持 Claude participant。
5. 通用圆桌先支持 Claude + Codex 两类参与者；Gemini 等 adapter 稳定后再加。
6. Driver 模式先做“人工触发 review”，不要第一版就做 MCP 工具链和危险操作拦截。
7. 每个阶段都保持 run 原子性：room 删除不能删除 run，除非用户明确选择。

## 最大风险

- **Codex/Gemini 完成状态不统一**：Hub 用 PTY buffer 和 watchdog 绕过，OpenCovibe 需要更清晰的 adapter 状态接口。
- **Prompt 注入污染**：会议规则、公约、review prompt、memory 注入必须有作用域，不能污染普通 chat run。
- **历史体积增长**：room timeline、run events、context.md 三份历史容易重复。需要明确 canonical source，建议 run events 保存原始执行，room timeline 保存协作摘要和引用。
- **UI 复杂度**：会议室很容易变成“大屏幕塞满卡片”。第一版应聚焦：成员条、输入框、轮次卡、当前响应、memo。
- **权限边界**：Driver/Copilot 的”谁能写文件”在 prompt 层只是软约束。真正安全要结合 OpenCovibe 的 permission panel、tool allowlist 和 agent settings。
- **Phase 边界塌陷**：Phase 2 一旦开始动 chat 路由就回不去了。Memo / Room / AgentAdapter 三个新模块必须在 feature flag 下（建议 settings 开关 + Cargo `cfg(feature = “rooms”)` 双重防护），主 chat 路径在 flag off 时**字节级一致**。这是写进 PR template 的硬要求，不是软约束。
- **Windows 工具链断层**：OpenCovibe 当前以 macOS 为主，Windows 未充分测试；Hub 用户大多在 Windows，迁移过去如果遇到 node-pty / Rust / Tauri 编译失败而 Doctor 又没告警，体验是灾难性的。Phase 1.c 必须落地 MSVC 检测，否则 Phase 2 把 Hub 用户引过来时会撞墙。

## 推荐最终形态

OpenCovibe 可以从“AI CLI 桌面壳”升级成“本地优先的 AI 协作工作台”：

- 单人工作：现有 chat/run 体验
- 临时认知：Memo
- 多模型讨论：Room Roundtable
- 工程执行：Driver/Copilot Room
- 项目沉淀：Arena Memory
- 远程控制：复用 OpenCovibe Web server，而不是另起 mobile server

Hub 的巧思应该被保留为协议和产品体验；OpenCovibe 的架构应该承担长期维护和跨平台能力。

## 收敛检查

1. 否决理由 -> ADR？已落地 [`docs/ADR-001-room-over-run.md`](docs/ADR-001-room-over-run.md)（Status: Accepted），覆盖 Room/Run 关系、AgentAdapter 边界、Hub 数据迁移策略、Feature Flag Policy、Alternatives Considered。
2. 踩坑教训 -> lessons-learned？无。本文档为前瞻性融合方案，尚无实际实现教训；Phase 1 落地后再补。
3. 操作规则 -> 指引文件？已部分落地：[`docs/migration-decisions.md`](docs/migration-decisions.md) 维护 Hub 能力分类（Migrate / Reference / Drop），[`docs/implementation-roadmap.md`](docs/implementation-roadmap.md) 维护 Phase 边界、验收标准与 Testing Strategy，[`docs/phase-1-memo-implementation-plan.md`](docs/phase-1-memo-implementation-plan.md) 承接第一段实现计划。

本 thinking.md 自此保留为思考过程档案，决策权移交至 `docs/` 下的正式文档。后续争议以 ADR-001 修订为准。

## 2026-04-30 Phase 2.x 进度补记

MSVC 环境注入已从“Doctor 扩展”调整为 spawn resolver 优先：先让 OpenCovibe 普通窗口启动的本地 Claude / Codex 子进程能拿到 VS Developer env，再补 UI 状态面。当前 Phase 2.x backend 已在 `feat/phase2x-msvc-env` worktree 实现：

- `windows_msvc_env_mode = auto | always | off` 设置模型。
- 保守 native project 探测，不用 `Cargo.toml` 单独触发 auto。
- `vswhere` + `VsDevCmd.bat` 派生 MSVC env，成功结果按 VS 安装路径和 arch 缓存。
- 只保留 MSVC build allowlist 变量，不向日志或 UI 暴露完整 env。
- Claude session actor、Codex pipe-exec、fork/one-shot local spawn 共用同一 `SpawnEnvPlan`。
- `PATH` / `INCLUDE` / `LIB` / `LIBPATH` 对 provider/user extra env 走 merge 而不是替换。

仍待补：settings/status UI、真实 Windows 普通启动下的 `where cl` 手工验收，以及 reviewer 对同步执行 `VsDevCmd.bat` 首次 spawn 延迟的判断。
