# Changelog / 更新日志

## Phase 10.a (2026-05-13)

### Managed Config Injection — Hooks/Plugins CRUD + Settings Refactor

**Bug Fixes:**
- MCP 持久化修复：`update_user_settings()` 新增 `mcp_servers` patch handler，修复托管 MCP 服务器添加后不持久化的问题
- Plugins marketplace 安装状态：已安装插件显示 "Installed" 禁用按钮，不再始终显示 "Install"
- Hooks 数据丢失修复：hooks 从 `~/.claude/settings.json` 迁移到 Claw GO managed settings，避免被 CLI 更新覆盖

**Settings 重构:**
- `update_user_settings()` 从 190 行 `if let Some` 单体函数重构为 19 个 per-field `apply_*` 函数
- 新增泛型 `apply_hashmap_field<V>` 和 `apply_deser_vec_field<T>` 消除重复代码
- 语义等价重构，无行为变更

**数据模型扩展:**
- `UserSettings` 新增 `hooks: HashMap<String, Value>` 和 `enabled_plugins: HashMap<String, bool>` 字段
- `#[serde(default)]` 保证旧 settings.json 向后兼容

**Session JSON 注入:**
- `ManagedConfig` 结构体捆绑 mcp_servers/hooks/enabled_plugins，避免参数膨胀
- `provider_config_json_from_env()` 三层合并：MCP additive、hooks per-event overwrite、plugins overlay
- superpowers 强制注入在 managed overlay 之后执行（`insert` 而非 `or_insert`，确保不可被覆盖）
- `write_mcp_only_settings` 重命名为 `write_managed_settings`

**IPC 命令:**
- 6 个新 Tauri 命令：`list_managed_hooks`、`add_managed_hook`、`remove_managed_hook`、`list_managed_plugins`、`set_managed_plugin`、`remove_managed_plugin`
- 遵循现有 MCP 命令的 read-modify-write 模式

**前端迁移:**
- `HookManager.svelte` 从 `getCliConfig`/`updateCliConfig` 迁移到 managed hooks API
- `api.ts` 新增 6 个前端 API wrapper
- `handleSaveEditor` 空数组时调用 `deleteEventHooks` 而非静默跳过
- `deleteEventHooks` 使用专用 `hooks_deleted` i18n key

**审查修复 (4 providers: Claude, DeepSeek, MiMo Plan, Packy CX2CC):**
- superpowers `or_insert` → `insert` — 确保强制覆盖不可被 managed config 绕过
- 测试编译错误修复：`&HashMap::new()` → `&empty_managed()`
- `write_mcp_only_settings` doc comment 更新
- 新增 `hooks_deleted` i18n key（en + zh-CN）

**Native Hooks Migration:**
- `hooks/setup.rs` 新增 `migrate_native_hooks()`：首次启动时自动将 `~/.claude/settings.json` 中的 hooks 导入 Claw GO managed settings
- `UserSettings` 新增 `native_hooks_migrated: bool` 标记，`#[serde(default)]` 保证向后兼容
- 仅在 managed hooks 为空时导入，避免覆盖用户已配置的 managed hooks
- 导入后从 native settings 移除 hooks key，避免重复注入
- `lib.rs` 启动时在 `cleanup_hook_bridge()` 之后调用

**Review Round 2 修复:**
- `migrate_native_hooks` save() 错误处理：`let _ = save()` → `if let Err(e)` + `log::warn!`，save 失败时跳过 native 移除（自愈：下次启动重试）
- `write_mcp_only_settings` 重命名为 `write_managed_settings`
- startup 顺序注释补充 cleanup→migration 依赖说明

## Phase 9.z (2026-05-12)

### Custom Provider 支持 + Native Config Merge + Managed MCP

**Custom Provider:**
- 后端 `provider_claude_config.rs` 新增 `custom-*` 平台路由：`is_custom_platform()`、`leak_custom_id()`（带 Mutex 缓存避免重复 leak）、`platform_to_provider_id()` 返回自身、`requires_explicit_base_url/model()` 要求必填、`provider_env_from_credential()` 委托 `build_parameterized_env`
- 前端 Settings → Connection 新增 Custom Providers 卡片：表单（Name / Base URL / API Key / Model / Effort Level）、CRUD 操作、已有 custom provider 列表展示
- 碰撞防护：`Date.now()` + `Math.random()` 随机后缀；URL 格式校验（仅允许 http/https）
- Custom form API key 可见性独立于全局 `showApiKey` 状态（`customShowApiKey`）
- 4 个新测试：`custom_platform_maps_to_self`、`custom_platform_requires_base_url_and_model`、`custom_platform_valid_with_all_fields`、`custom_platform_builds_parameterized_env`

**Native Config Merge:**
- `provider_config_json_from_env` 重构：以 native `~/.claude/settings.json` 为基底，strip 敏感 key（`apiKey`/`primaryApiKey`），叠加 provider env/permissions/MCP，保留 hooks/plugins/enabledMcpjsonServers 等用户配置
- `SENSITIVE_KEYS` 从 `cli_config.rs` 提取为 `pub const`，`session.rs` 和 `provider_claude_config.rs` 共享引用，消除重复常量
- 6 个新测试覆盖：native hooks 保留、API key 剥离、MCP 合并、env 覆盖、native env 保留、superpowers 插件强制启用

**Managed MCP Injection:**
- `mcp_registry.rs` 新增第 5 来源：Claw GO 托管服务器（`UserSettings.mcp_servers`），scope="managed"
- 托管服务器替换同名 `scope="user"` 条目，保留 `local`/`project` scope
- Extensions 页面配置列表正确显示托管 MCP 服务器

**其他:**
- `provider_config_json_from_env` 硬覆盖字段（thinking/includeCoAuthoredBy/language 等）补充设计意图注释

## Phase 9.y (2026-05-09)

### v1.1.7 — 第三方 session provider 显式配置校验与 Xiaomi 共用模型配置

- 第三方 session provider 新增统一显式配置校验结果结构：`ProviderIssue`、`ProviderValidationResult`、`ValidatePlatformCredentialsResponse`
- 后端在 `src-tauri/src/agent/provider_claude_config.rs` 中新增统一校验入口 `validate_provider_credential` / `validate_platform_credentials`，覆盖 DeepSeek、GLM、QWEN、KIMI、Xiaomi（`mimo-plan` / `mimo-api`）、Packy
- `build_deepseek_env` / `build_parameterized_env` 在生成临时 session JSON 前先执行统一校验；配置不完整时直接阻止 provider config 生成
- 新增 settings IPC：`validate_platform_credentials`，并在 `src-tauri/src/lib.rs` 注册
- Settings → Connection 页新增“应用并校验配置”按钮：保存当前 `platform_credentials` 后立即调用后端统一校验，并在 provider 卡片内联展示字段级问题列表
- DeepSeek / Packy 卡片补充提示语义：明确要求显式填写完整模型配置；Packy 不再使用默认模型兜底
- Xiaomi 双 provider 卡片收口：`mimo-plan` 与 `mimo-api` 共享 6 个模型配置输入（`ANTHROPIC_MODEL`、三档 tier、`CLAUDE_CODE_SUBAGENT_MODEL`、`CLAUDE_CODE_EFFORT_LEVEL`），输入变更双写到两份 `extra_env`；`api_key` 与 `base_url` 仍分别保存在各自 credential 中
- Xiaomi / provider 校验成功文案从“配置完整，可启动”收窄为“配置校验通过”，避免对运行态做过度承诺
- Rust 测试代码补充：新增 `kimi` / `deepseek` / `mimo-api` / `packy` 的显式校验覆盖；本机仍受既有 `0xc0000139` 环境问题影响，验证以 `cargo check` 为主
- Xiaomi 共用模型配置一致性修复：Settings 页共用模型面板改为共享视图（优先 `mimo-plan.extra_env`，缺失时回退 `mimo-api.extra_env`），后端 `migrate_platform_credentials` 新增共享字段补齐逻辑，自动修复历史上 `mimo-plan` / `mimo-api` 模型字段分叉导致的 `mimo-api` 校验缺项问题

### v1.1.6 — 旧 ID 彻底清理

- 移除所有旧 provider ID 支持：`mimo-pro`、`xiaomi`、`mimo` 从前端 `providerIdForRun` + 后端 `platform_to_provider_id`/`provider_env_from_credential`/`default_base_url`/`is_phase7_claude_compatible_api_platform`/`known_provider_defaults`/`auth_fixes` 同步删除
- 移除旧 ID 迁移逻辑（`migrate_platform_credentials` 中 mimo-pro→mimo-plan、mimo/xiaomi→mimo-api 的迁移代码）
- `mimo-plan` provider label 从 `"Xiaomi"` 改为 `"Xiaomi (Plan)"`，与 `"Xiaomi (API)"` 明确区分
- `session-store.test.ts` 新增 `preserves raw multi-question AskUserQuestion options on tool_start` 测试
- 全局 `rustfmt` 格式化统一：多行断言、函数签名、match arm 缩进

### v1.1.5 — Provider 预设清理与白名单机制

- 新增 Packy CX2CC API provider（base URL: https://www.packyapi.com），模型从设置页读取
- 移除 5 个无后端支持的 provider 预设：kimi-coding、doubao、minimax、minimax-cn、mimo（前端 platform-presets.ts + 后端 onboarding.rs/settings.rs 同步清理）
- `PlatformCredential.extra_env` 白名单机制：`ALLOWED_EXTRA_ENV_KEYS` 限制用户可覆盖的环境变量（模型 tier + effort level），防止误覆盖稳定性变量
- `merge_extra_env` 合并函数：stability_env_vars → extra_env 覆盖顺序，空值过滤，6 个单元测试覆盖
- Settings 页 CC Session provider 卡片重设计：API Key 始终可见 + 可折叠高级配置面板（6 个 env var 字段：5 文本框 + 1 effort level 下拉框）
- Chat 页模型下拉菜单显示 tier 标签（Opus/Sonnet/Haiku），使用 `expandModelsToTiers` 展开，支持 extra_env 覆盖
- 第三方 provider 模型热切换：移除 `!isThirdParty` 限制，`set_model` control protocol 经 DeepSeek 和 MiMo Pro 实测有效
- extra_env 输入框统一为 `onblur` 持久化，与 API Key 字段行为一致
- EFFORT_LEVEL 下拉框改用 Svelte 受控 `value` 绑定
- placeholder 使用 tier 展开结果，修复 2 模型配置下 sonnet/haiku 显示错误的问题

## Phase 9.x (2026-05-09)

- Room adapter timeout 重构：固定 5 分钟 `max_polls` 改为活动感知双层超时（10 分钟不活跃 + 30 分钟硬截止）
- `RunMeta.active_at` 字段：EventWriter 节流写入（1s 间隔），用于检测 run 是否仍在活跃
- `events.rs` lock scoping 改进：per-run 锁在调用 `update_active_at_throttled` 前释放，避免潜在死锁
- `cancel_room_turn` Tauri 命令：遍历 room participants 停止活跃 run，过滤非 Running 状态
- 前端 Cancel 按钮：turn 进行中时替换 Send 按钮，`cancelGeneration` 防止竞态
- 前端长时间运行警告：运行超过 5 分钟显示 amber 标签
- 前端最近活动显示：使用 `active_at` 优先于 event-derived `last_activity_at`
- `get_run()` 修复：SessionActor 运行的 `last_activity_at` 不再为 `undefined`
- Adapter 测试补充：`with_deadlines()` + 硬截止超时测试 + 不活跃超时测试
- Adapter I/O 优化：每次循环只读一次 `meta.json`，移除死代码 `read_outcome`

## Phase 9 (2026-05-08)

- History 页面重写：从 Claw GO runs 切换为直接读取 CC 原生会话（`~/.claude/projects/`）
- 过滤掉 `hasSubagents: true` 的子代理会话
- 简化 UI：仅显示 prompt、时间、项目路径、模型 badge、继续/导入按钮
- 支持文本搜索 + 项目 pill 过滤
- 已导入会话跳过重复导入，直接跳转
- 清理 ~30 条无用 i18n keys，新增 10 条 CC 历史相关 keys
- AskUserQuestion / elicitation 交互按钮显式设为 `type="button"`，避免多问题权限卡片重复提交

## Phase 8.x (2026-05-08)

- 聊天侧边栏预览修复：`summarize_events()` 改为反向扫描，显示最新消息而非最早消息
- 版本更新检查 GitHub 地址修正为 fork 仓库
- Provider 切换时自动更新默认模型，修复旧值残留导致的错误
- 新建圆桌会议室后显示可关闭的命令速查横幅（@debate、@summary、/dm、@Name）

## Phase 8 (2026-05-08)

- Gemini 彻底移除（~54 文件，前后端 + 测试 + 文档）
- Stepper mini-map 替换 History strip，支持逐轮回放与快照加载
- `@DisplayName message` SingleTarget 公开点名（仅被点名者回答）
- `/dm @Name message` 保留私有回合
- Room sidebar 虚拟"会议室"文件夹分组
- Roundtable seat prompt 英文证据约束
- Context events 跨 session 类型验证
- Code review 修复：snapshot 渲染、activeSnapshot 复位、Private handler 歧义检测、i18n、guard

## Phase 7.y (2026-05-07)

- Room 删除时停止 participant 并软删除 runs
- Roundtable 增量回合推送（JSONL 去重 + 1500ms 前端轮询）
- 右键"移除会话"上下文菜单（含 force-stop）
- Participant 状态本地化（pending→Starting..., running→Thinking...）
- Seat label 修改自动同步 prompt

## Phase 7.x (2026-05-07)

- Provider 配置完全动态化（从设置页读取而非硬编码模型/URL）
- Per-session 临时配置 JSON（`--settings session-{run_id}.json`）
- MiMo Pro provider 新增
- MiMo 余额/用量检查器（cookie 认证，双 API，琥珀色主题卡片）

## Phase 7 (2026-05-06)

- Codex PTY 原生 CLI 适配器
- Provider 设置页动态化
- Roundtable 三栏布局重设计
- 全局备忘面板重构

## Phase 6

- Driver MCP

## Phase 5.5

- Native CLI chat parity

## Phase 5

- Capability matrix

## Phase 4.5

- Research follow-up

## Phase 4

- Driver/Copilot

## Phase 3

- Roundtable implementation

## Phase 1

- Memo implementation
