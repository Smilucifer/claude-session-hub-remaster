# P0 Bug 排查报告 & 修复方案

> 日期: 2026-05-14 | 来源: 群聊体验反馈圆桌讨论 | 状态: ✅ 已完成 (v2.2.0)

---

## 问题总览

| # | 问题 | 类型 | 严重度 | 根因 | 涉及文件 |
|---|------|------|--------|------|----------|
| A | Avatar 上传选择后无反馈 | Bug | P0 | Tauri v2 移除 `file.path`，回退到 `file.name` 导致 Rust 端无法打开文件；新建角色时 `editingId=null` 阻断上传 | `src/routes/settings/characters/+page.svelte` |
| B | 角色人设 prompt 未载入到群聊 | Bug | P0 | `character_id` 在 participant 创建时永远为空字符串，`resolve_participant_system_prompt` 直接返回 `None` | `src-tauri/src/storage/group_chats.rs`、`src-tauri/src/group_chat/orchestrator.rs`、`src-tauri/src/commands/group_chat.rs`、`src/lib/api.ts` |
| G | 无法同时 @mention 多个角色 | Bug | P0 | `parse_group_chat_command` 用 `splitn(2, …)` 只取第一个 @mention；`GroupChatCommand` 无 `MultiTarget` 变体 | `src-tauri/src/group_chat/orchestrator.rs` |
| I | Embedding API Key 切页后显示为空 | Bug | P0 | 前端传 `apiKey`（camelCase）但 Rust struct 用 snake_case，serde 反序列化丢弃字段 → 写入 `None` → 回显空值 | `src/routes/settings/+page.svelte`（可能已在 6e755ab 修复） |

---

## 详细分析

### A. Avatar 上传反馈链路断裂

**问题链路：**

```
用户选择文件 → onchange
  ├── 新建角色: editingId=null → if (!editingId) return   ← 直接跳过
  └── 编辑角色: (file as any).path → undefined (Tauri v2)
        └── 回退 file.name = "photo.png"
              └── Rust: std::fs::open("photo.png") ← 找不到文件
                    └── catch { dbgWarn(...) } ← 静默失败，无 UI 提示
```

**三个根因：**

| 子问题 | 位置 | 说明 |
|--------|------|------|
| 新建角色无法上传 | `+page.svelte:425` | `if (!editingId) return` 阻断，需先保存角色才能上传 |
| 文件路径不可用 | `+page.svelte:427` | `(file as any).path` 在 Tauri v2 中为 undefined |
| 无用户反馈 | `+page.svelte:429-431` | catch 块只写 dbgWarn 日志，不显示 toast |

**修复方案：**
1. 将 `<input type="file">` 替换为 `@tauri-apps/plugin-dialog` 的 `open()` API（与项目其他文件选择一致，见 `chat/+page.svelte:1950`）
2. 新建角色也允许上传：保存临时路径，创建完成后调用 `uploadCharacterAvatar`
3. 加 toast 提示（成功/失败）

---

### B. 角色人设 prompt 链路断裂

**问题链路：**

```
前端 addCharacterParticipant(char)
  ├── 传入 char.role_instruction 作为 run.prompt (一次性快照)
  └── 不传 char.id
        └── Tauri: create_group_chat_claude_participant (无 character_id 参数)
              └── attach_group_chat_run: character_id = String::new()  ← 永远为空
                    └── orchestrator: resolve_participant_system_prompt()
                          └── if character_id.is_empty() → return None  ← 跳过注入
```

**两个断裂点：**

| 断裂点 | 位置 | 说明 |
|--------|------|------|
| `character_id` 不传递 | `group_chats.rs:192`、`group_chat.rs:228-258`、`api.ts:192-213` | 整个调用链都没有 `character_id` 参数 |
| 空 ID 跳过注入 | `orchestrator.rs:823` | `resolve_participant_system_prompt` 对空字符串返回 `None` |

**修复方案：**
1. 前端 `api.ts` 的 `createGroupChatClaudeParticipant` 加 `characterId` 参数
2. 前端 `GroupChatLayout.svelte` 的 `addCharacterParticipant` 传入 `char.id`
3. Tauri 命令 `create_group_chat_claude_participant` 加 `character_id` 参数
4. 后端 `attach_group_chat_run` 加 `character_id` 参数并写入 `GroupChatParticipant`
5. 以上完成后 `resolve_participant_system_prompt` 自动生效——它会动态读取 `AiCharacter.role_instruction`，用户编辑角色人设后下次轮次即刻生效

---

### G. 多 @mention 解析失败

**问题链路：**

```
输入: "@Alice @Bob check this"
  └── parse_group_chat_command()
        └── trimmed.strip_prefix('@') → "Alice @Bob check this"
              └── .splitn(2, char::is_whitespace) → ["Alice", "@Bob check this"]
                    target = "Alice"
                    message = "@Bob check this"  ← @Bob 变成消息文本
```

**根因：**
- `splitn(2, …)` 硬编码拆2段
- `GroupChatCommand` 枚举无 `MultiTarget` 变体
- `GroupChatTurnMode` 枚举无 `MultiTarget` 变体
- 执行路径只支持 `vec![target_ref]` 单个目标

**修复方案：**
1. `GroupChatCommand` 加 `MultiTarget { targets: Vec<String>, message: String }` 变体
2. `parse_group_chat_command` 中 @mention 解析改为：扫描所有 `@label` 前缀，提取全部匹配的 participant label，剩余部分为 message
3. 执行路径改为 `vec![target_refs…]`，对每个目标独立创建 turn
4. 前端 Stepper 显示 "已分发至: @Alice, @Bob"

---

### I. Embedding API Key 回显失效

**问题链路（修复前）：**

```
前端: updateEmbeddingConfig({ apiKey: "sk-..." })
  └── JSON: { "config": { "apiKey": "sk-..." } }
        └── Rust EmbeddingConfig { api_key: Option<String> }  ← 字段名不匹配
              └── serde 反序列化: api_key = None
                    └── settings.json 写入: "api_key": null
                          └── 重新加载: get_embedding_config() → api_key = None
                                └── 前端回显: embeddingApiKey = ""  ← 显示为空
```

**修复方案（已在 6e755ab 应用）：**
- 字段名从 `apiKey` 改为 `api_key`（匹配 Rust snake_case）
- 空值从 `null` 改为 `undefined`（匹配 `skip_serializing_if = "Option::is_none"`）
- 修复后测试成功可以保存并回显

**待验证：** 需在最新构建上确认是否已彻底修复。

---

## 修复优先级 & 依赖

```
A (前端独立, 无依赖) ─┐
                      ├─ 第 1 批（可并行）
B (全栈, 无依赖)     ─┤
                      │
G (后端为主)         ─┘  ← 和 B 共享 orchestrator.rs 文件
                      │
I (验证即可)           ─── 可能已修复，仅需确认
```

---

## 验证 Checklist

- [ ] **A**: 新建角色 → 选择头像 → 即时预览出现 → 保存 → 切页再回 → 头像保持
- [ ] **A**: 编辑已有角色 → 更换头像 → 即时预览 → 保存 → 头像更新
- [ ] **B**: 编辑角色人设 → 保存 → 在群聊中创建该角色 participant → 发送消息 → 回复体现人设
- [ ] **B**: 修改已有角色的人设 → 群聊下一轮 → 回复使用新人设
- [ ] **G**: 输入 `@Alice @Bob 帮我分析` → 发送 → Alice 和 Bob 都收到分发并回复
- [ ] **I**: 设置页 → 填写 Embedding API Key → 保存 → 测试成功 → 切到其他页 → 切回设置 → Key 回显（掩码）
