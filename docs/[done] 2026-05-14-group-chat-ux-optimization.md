# 群聊体验优化 & Bug 修复计划

> 状态: ✅ 已完成 (v2.2.0) | 日期: 2026-05-14 | 版本: v1

---

## Context

基于群聊功能的实际使用反馈，当前存在 9 个问题，覆盖：
- **Bug 修复**（Avatar 上传、角色人设载入、Embedding Key 回显、多 @mention 解析）
- **群聊核心体验**（输出排版、Executor 路由规则、上下文互相可见）
- **性能优化**（cx2cc 群聊启动慢）
- **功能补全**（Character Memory System 剩余项）

目标：修复阻塞体验的 bug，升级群聊可读性和交互逻辑，优化启动速度。

---

## 问题全景

| 编号 | 问题 | 类型 | 优先级 | 影响范围 |
|------|------|------|--------|----------|
| A | Avatar 上传选择后无反馈，用户不知道是否生效 | Bug | P0 | 角色编辑页 |
| B | 角色人设 prompt 保存后未正确载入到群聊 | Bug | P0 | 角色存储读取 + 群聊编排 |
| G | 无法同时 @mention 两个及以上角色 | Bug | P0 | @mention 解析器 |
| I | Embedding API Key 设置页切回后显示为空 | Bug | P0 | 设置页回显 |
| C | 群聊输出平铺无排版换行，多参与者难以扫读 | 体验 | P1 | 消息渲染组件 |
| D | Executor 无 @mention 时不应主动回答 | 逻辑 | P1 | orchestrator + Stepper |
| H | 群聊参与者应能看见彼此上下文（至少当前轮） | 功能 | P1 | orchestrator 上下文注入 |
| E | cx2cc 群聊启动时 superpowers 强制通读项目文件夹，启动慢 | 性能 | P2 | 群聊启动链路 |
| F | Character Memory System 剩余功能补全 | 功能 | P3 | memory 模块 |

---

## 执行阶段

### 第 1 步 — P0 Bug 修复（阻塞基础体验）

#### A. Avatar 上传反馈链路修复

**问题**：选文件后无即时预览、无上传状态、失败无回滚，用户无法判断是否生效。

**方向**：
1. 文件选择后立即显示本地预览（`URL.createObjectURL`），不等后端返回
2. 预览区域覆盖 mini spinner 表示上传中
3. 后端保存失败时回滚旧图 + toast 提示

**涉及文件**：角色编辑页前端组件（`src/routes/settings/characters/`）

---

#### B. 角色人设 prompt 未正确载入

**问题**：用户编辑了角色人设但在群聊对话中未体现。

**方向**：
1. 排查 prompt 读取链路：保存时的序列化 → 存储格式 → 群聊编排层注入时的读取
2. 检查是否存在"读取了旧缓存"或"字段名不匹配（如 `system_prompt` vs `role_instruction`）"的问题
3. 角色编辑页加一个 mini 预览区：输入人设后旁边显示示例回复，让用户建立"写了就会被用上"的信心

**涉及文件**：
- `src-tauri/src/storage/settings.rs`（角色存储读取）
- `src-tauri/src/group_chat/orchestrator.rs`（群聊 prompt 注入点）
- 角色编辑页前端组件

---

#### G. 多 @mention 解析修复

**问题**：同时 @mention 两个或以上角色时，第二个及之后的标记被无视。

**方向**：
1. @mention 解析器从"匹配第一个"改为"匹配所有"（全局扫描而非首次匹配）
2. 注意边界过滤：`@@name`、`@ name`（中间空格）为无效格式
3. UI 增强：
   - 输入框中每个 @mention 都有视觉标记（颜色 chip），发送前用户可确认
   - 解析失败的 mention 给红色下划线提示
   - GroupChatStepper 显示"已分发至: @Planner, @Executor"而非仅第一个

**涉及文件**：
- 前端 @mention 解析逻辑（composer / input 组件）
- `src-tauri/src/group_chat/orchestrator.rs`（多目标分发）
- `src/lib/components/GroupChatStepper.svelte`（状态显示）

---

#### I. Embedding API Key 回显修复

**问题**：设置页填写 API Key 测试成功，切到其他页再回来显示为空。

**方向**：
1. 排查字段序列化一致性：保存时的 key name → 后端 JSON key → 前端反序列化时的 key name 是否一致
2. 确认设置页 onMount / 初始化时是否从持久化存储重新加载（而非仅依赖内存状态）
3. 加保存状态指示器：保存按钮旁显示"已保存 HH:MM"绿色确认

**涉及文件**：
- `src-tauri/src/storage/settings.rs`（embedding 配置持久化）
- 设置页前端组件（`src/routes/settings/`）
- 前端 settings store / API 调用层

---

### 第 2 步 — P1 群聊核心体验

#### C. 群聊输出排版 & 可读性

**问题**：长文本平铺无换行，多参与者场景下扫读困难。

**方向**：
1. **基础层**：消息组件启用 Markdown 渲染（段落间距、标题层级、列表、代码块）
2. **体验层**：
   - 参与者之间用视觉分隔（间距 + 头像/颜色锚点）
   - 长文本自动折叠首段 + "展开阅读"
   - Planner 输出的任务列表用微高亮标记
3. **品牌层**：不同角色类型可配置输出风格指引（Planner 偏结构化、Executor 偏段落），在角色人设 prompt 中加入格式指引

**涉及文件**：
- 群聊消息渲染组件（`GroupChatLayout.svelte` 或消息气泡组件）
- 角色人设编辑页（可选：加输出格式字段）

---

#### D. Executor 无 @mention 不主动回答

**问题**：当前靠 prompt 软约束不可靠，需要 orchestrator 硬拦截。

**方向**：
1. orchestrator 层加入路由规则：
   ```
   消息到达 → 解析 @mention 目标
     ├── 无 @mention → 仅分发给 Planner
     ├── @Executor → 分发给 Executor
     └── @Planner → 正常分发
   ```
2. GroupChatStepper 视觉标记：未分发的参与者显示"未分发（未被 @mention）"
3. 这不是软建议，是硬约束——prompt 里也保留指引作为双重保障

**涉及文件**：
- `src-tauri/src/group_chat/orchestrator.rs`
- `src/lib/components/GroupChatStepper.svelte`

---

#### H. 群聊上下文互相可见

**问题**：参与者之间互相不知道对方说了什么，用户被迫当传声筒。

**方向**：
1. 上下文分层设计：
   - **公共层**：当前轮所有公开消息 → 注入给每个参与者
   - **私密层**：@dm 私密消息 → 仅注入给该参与者
   - **角色层**：参与者自身历史 → 保持记忆连续性
2. 注入时机：每轮开始前，将公共层内容追加到各参与者的上下文中
3. Planner 的系统 prompt 中加入指引："你可以看到本轮所有参与者的发言，主动参考，无需等待用户重复"

**涉及文件**：
- `src-tauri/src/group_chat/orchestrator.rs`（上下文组装 & 注入）
- 角色 system prompt 模板

---

### 第 3 步 — P2 性能优化

#### E. cx2cc 群聊启动优化

**问题**：superpowers 强制通读项目文件夹，群聊场景不需要，启动很慢。

**方向**：
1. **信号传递方案**（推荐）：群聊启动时传入场景标记（环境变量或 CLI flag），superpowers 检查标记后跳过项目扫描
2. 标记不在 prompt 里而在启动参数里，确保不被 prompt 覆盖
3. 单人开发场景不受任何影响
4. 保留"被明确要求时读取"的能力——如果用户在群聊中说"看看这个项目"，再去加载

**涉及文件**：
- 群聊启动链路（`src-tauri/src/group_chat/` 或 `src-tauri/src/agent/`）
- Superpowers skill 初始化逻辑

---

### 第 4 步 — P3 功能补全

#### F. Character Memory System 剩余功能

根据 Phase 10+ 计划，剩余项：
- sigma.js 知识图谱可视化
- LLM 自动提取
- Review queue
- Injection config UI

**建议优先做 Injection Config UI**——这是用户最能直接感知的入口，做了之后其他 memory 功能才有"被用户看到"的界面。

---

## 依赖关系

```
A (前端独立) ─┐
B (前后端)   ─┤
G (前后端)   ─┼─ 第1步：可并行排查/修复
I (前后端)   ─┘
                │
                ▼
C (前端独立) ─┐
D (后端为主) ─┼─ 第2步：C 独立，D+H 共享 orchestrator 改动
H (后端为主) ─┘
                │
                ▼
E (启动链路) ─── 第3步：依赖 D+H 的 orchestrator 改动稳定后
                │
                ▼
F (memory) ──── 第4步：独立于群聊改动
```

---

## 验证方式

每个步骤完成后：

1. **A**: 角色页 → 选择头像文件 → 确认预览立即出现 → 切页再回 → 确认头像保持
2. **B**: 编辑角色人设 → 保存 → 在群聊中使用该角色 → 确认回复体现人设
3. **G**: 输入 `@角色A @角色B 帮我分析` → 发送 → 确认两个角色都收到分发
4. **I**: 设置页 → 填入 API Key → 保存 → 切到其他页 → 切回设置 → 确认 Key 显示（掩码或圆点）
5. **C**: 群聊中让 Planner 输出多段落+列表 → 确认渲染有换行和排版
6. **D**: 群聊中不 @Executor 发消息 → 确认 Executor 不回复
7. **H**: @Planner 提问 → Planner 回复后 → @Executor 参考 Planner 的方案 → 确认 Executor 能看到 Planner 的回复
8. **E**: 群聊启动 → 确认不触发项目文件夹扫描 → 启动时间明显缩短
9. **F**: 按具体功能分别验证

运行 `npm run build` 和 `npm run verify` 确保无回归。
