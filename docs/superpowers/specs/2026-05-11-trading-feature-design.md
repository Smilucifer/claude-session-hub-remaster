# A股交易分析功能设计文档

> 设计日期：2026-05-11 | 状态：[wip] | 版本：1.2.0

## 概述

为 Claw GO 桌面应用新增可选启用的 A 股交易分析模块。用户收盘后粘贴同花顺截图 → 自动提取交易数据 → 展示盈亏/胜率/持仓 → AI 引导复盘对话 → 生成交易笔记 → 积累知识图谱 → Room 多 agent 协作生成 AI 研报。

### 核心工作流

```
同花顺截图 → 解析流水线 → FIFO盈亏引擎 → 仪表盘
                                        → 复盘对话 → 交易笔记
                                        → 知识图谱
                                        → Room AI研报
```

---

## 整体架构

### 文件分布

| 层级 | 新增内容 |
|------|---------|
| 前端路由 | `src/routes/trading/` — dashboard、graph、notes |
| 前端 stores | `src/lib/stores/trade-store.svelte.ts` |
| 前端 utils | `src/lib/utils/trade-engine.ts` — FIFO 引擎辅助 |
| 前端组件 | `src/lib/components/trading/` — DashboardCards、PnLChart、PositionTable 等 |
| Rust 命令 | `src-tauri/src/commands/trading.rs` |
| Rust 存储 | `src-tauri/src/storage/trading.rs` |
| Rust 引擎 | `src-tauri/src/trading/fifo.rs` — FIFO 盈亏计算核心 |
| Rust HTTP | `src-tauri/src/trading/market_data.rs` — 东方财富行情抓取 |
| i18n | `messages/en.json` + `messages/zh-CN.json` — 交易相关 key |

### 复用现有基础设施

| 现有设施 | 用途 |
|---------|------|
| Chat session | 复盘对话载体 |
| Room orchestrator | 多 agent AI 研报编排 |
| Memo store | 笔记存储参考模式 |
| Explorer | `notes/trading/` 文件浏览 |
| Transport (Tauri IPC) | 前后端通信 |
| 设置页 | A 股分析开关 |
| 侧边栏 | `/trading` 入口（条件显隐） |

---

## 审查修正（多路审查共识）

四路 AI 审查（DeepSeek、GLM、MiMo、Packy）形成的关键修正：

| # | 修正项 | 原方案 | 修正后 |
|---|--------|--------|--------|
| 1 | **mx-data/mx-search 不可直接复用** | 复用 mx-data skill | Rust 侧自建东方财富 HTTP 客户端（`reqwest`），通过 Tauri command 暴露行情数据 |
| 2 | **FIFO 引擎放 Rust** | TypeScript 前端 | `src-tauri/src/trading/fifo.rs`，用 `rust_decimal` 保证金融精度，前端通过 IPC 查询 |
| 3 | **Recharts 不兼容 Svelte 5** | 用 Recharts 做图表 | 用纯 SVG/Canvas 实现图表（Svelte 5 原生） |
| 4 | **Phase 1 大幅缩减** | 含截图OCR+仪表盘图表+实时行情 | 只做：交易记录CRUD + FIFO引擎 + CSV/手动录入 + 盈亏卡片 + `/trade-review` 命令 |
| 5 | **`/review` 命令名冲突** | `/review` | `/trade-review`（`slash-commands.ts` 第 39 行已有 `/review` 做 PR 审查） |
| 6 | **OCR 推迟到 Phase 2** | Phase 1 含截图OCR | Phase 2 先做原型验证（20张同花顺截图基准测试），再决定 Tesseract vs WinRT OCR 方案 |

---

## Phase 1：MVP — 能记录、能复盘（2-3 周）

### 目标
用户能通过 CSV 或手动录入交易数据，系统计算 FIFO 盈亏，展示基础盈亏卡片。可在 chat 中用 `/trade-review` 触发 AI 复盘对话。

### 任务清单

#### T1.1 Rust 后端：交易存储模块
**文件：** `src-tauri/src/storage/trading.rs`

- 数据目录：`~/.claw-go/trading/`
- `trades.jsonl` — 每行一个 TradeRecord（JSONL 格式）
- `positions.json` — 当前持仓快照
- 复用现有 `storage/` 模块的模式（参考 `runs.rs`、`settings.rs`）

#### T1.2 Rust 后端：FIFO 盈亏引擎
**文件：** `src-tauri/src/trading/fifo.rs`

```
fn calculate_pnl(trades: Vec<TradeRecord>) -> PnlSummary
```

- 使用 `rust_decimal` crate 保证金融精度
- 买入 → push 批次到持仓队列
- 卖出 → 从队列头部 FIFO pop，计算已实现盈亏
- 公式：收入 - 成本基础 - 佣金 - 印花税(仅卖出) - 过户费
- 输出：`Vec<RealizedPnL>`、`Vec<OpenPosition>`、`AggregatedStats`
- A 股特殊规则注释：
  - T+1 制度
  - 印花税单边征收（卖出 0.1%）
  - 过户费双向
  - 佣金最低标准

**单元测试覆盖：**
- 标准单笔买入→卖出
- 多批次不同价格买入→部分/全部卖出
- 同只股票多批次交叉
- 空持仓卖出报错
- 精度测试（涉及小数分/厘）

**Cargo.toml 新增依赖：** `rust_decimal = "1"`

#### T1.3 Tauri 命令
**文件：** `src-tauri/src/commands/trading.rs`

| 命令 | 功能 |
|------|------|
| `import_trades` | 批量导入交易记录（CSV 解析结果或手动录入） |
| `list_trades` | 按日期范围查询交易记录 |
| `get_pnl_summary` | 获取 FIFO 盈亏聚合数据 |
| `get_positions` | 获取当前持仓列表 |
| `delete_trade` | 删除单条交易记录 |

#### T1.4 前端：交易 Store
**文件：** `src/lib/stores/trade-store.svelte.ts`

- 封装 Tauri IPC invoke 调用
- 响应式状态：`trades`、`pnlSummary`、`positions`
- 方法：`loadTrades()`、`importTrades()`、`refreshPnl()`

#### T1.5 前端：`/trading` 页面
**文件：** `src/routes/trading/+page.svelte`

- Tab 切换（不采用子路由，避免布局重挂载）：仪表盘 / 笔记 / （图谱 Phase 3）
- 交易记录表格：时间、代码、名称、方向、数量、价格、金额、费用
- CSV 导入按钮 + 手动录入表单
- 盈亏摘要卡片：总盈亏、胜率、交易笔数
- 简易 SVG 柱状图（月度盈亏，纯手写 < 200 行）
- 当前持仓表格

#### T1.6 前端：`/trade-review` 斜杠命令
**文件：** `src/lib/utils/slash-commands.ts` 新增条目

- 注册 `VIRTUAL_COMMANDS` 条目：`/trade-review`
- 支持参数：`/trade-review [日期]`、`/trade-review week`、`/trade-review month`
- 上下文自动注入：当日交易记录 + FIFO 盈亏 + 持仓 + 个人策略文件（`~/.claw-go/A-ShareStrategy.md`）
- AI 引导五步复盘：操作回顾 → 市场环境 → 心态/纪律 → 核心反思 → 明日计划
- 对话中可随时说"生成复盘笔记" → 保存为 `notes/trading/日复盘/YYYY-MM-DD-复盘.md`

#### T1.7 个人股市策略文件
**文件：** `~/.claw-go/A-ShareStrategy.md`

- 首次启动时自动生成空模板
- 模板内容示例：

```markdown
# 我的股市策略

## 投资风格
（待填写：如趋势跟踪、价值投资、波段操作等）

## 交易规则
（待填写：买入条件、卖出条件、仓位管理等）

## 风险控制
（待填写：止损线、单票仓位上限等）

## 经验教训
（待填写：过往总结的重要教训）
```

- 每次 `/trade-review` 和 `/research` 对话时，自动读取此文件内容注入到 context
- 用户可在 Explorer 中直接编辑此文件

#### T1.8 设置开关 & 侧边栏集成
**文件：** `src/routes/settings/+page.svelte` + `src/routes/+layout.svelte`

- `UserSettings` 新增字段 `trading_enabled: Option<bool>`（Rust models.rs + 前端 types.ts）
- 设置页新增 "A 股分析" 开关区域
- 侧边栏 `navItems` 条件渲染：`trading_enabled ? { path: "/trading", icon: "trending-up" } : null`
- 路由守卫：`/trading` 禁用时显示"功能未启用"提示
- macOS 编译时此功能不编译（Rust `#[cfg(target_os = "windows")]` 条件编译 + 前端 `trading_enabled` 默认 `None`）

#### T1.9 Windows 专属屏蔽

**Rust 侧：**
- `src-tauri/src/commands/trading.rs` 所有命令加 `#[cfg(target_os = "windows")]`
- `src-tauri/src/storage/trading.rs` 条件编译
- `src-tauri/src/trading/` 模块条件编译

**前端侧：**
- macOS 上 `trading_enabled` 默认为 `None`（即不显示）
- 设置页开关仅在 Windows 上可见
- `/trading` 路由仅在功能启用时可访问

#### T1.10 i18n
- 新增 ~20 个 message key（en.json + zh-CN.json）
- 关键 key：`nav_trading`、`trading_dashboard`、`trading_total_pnl`、`trading_win_rate`、`trading_positions`、`trading_review_start` 等

### Phase 1 验收标准

- [ ] FIFO 盈亏计算结果与 Excel 手工计算一致（差 < 0.01 元）
- [ ] CSV 导入 + 手动录入均正常工作
- [ ] `/trade-review` 斜杠命令不冲突
- [ ] `/trade-review` 注入的交易 context 包含个人策略文件内容
- [ ] 个人策略文件首次启动自动生成空模板
- [ ] 盈亏摘要卡片数据准确
- [ ] 设置开关关闭后侧边栏入口消失
- [ ] Rust `cargo check` 通过、前端 `npm run build` 通过
- [ ] i18n en/zh-CN 覆盖所有新增 key
- [ ] macOS 编译不包含交易功能代码

---

## Phase 2：完善 — 截图解析 + 行情 + 图表（2-3 周）

### T2.1 截图解析流水线
- 先做原型验证：20 张同花顺截图基准测试
- 根据测试结果选择 OCR 方案：
  - WinRT OCR（`Windows.Media.Ocr`，需 `windows` crate WinRT 投影）
  - Tesseract OCR（通过 subprocess 调用）
  - Vision API fallback（用当前对话 provider）
- 实现 `parse_trade_screenshot` Tauri 命令
- 前端独立按钮触发（非自动拦截 chat 粘贴）
- 解析结果可编辑卡片 → 确认导入

### T2.2 行情数据集成
**文件：** `src-tauri/src/trading/market_data.rs`

- 用 `reqwest` 抓取东方财富公开 API（push2ex.eastmoney.com 等端点）
- 实时价格查询 → 持仓浮动盈亏计算
- 大盘行情摘要（上证/深证/创业板指数）
- 5 分钟可配刷新间隔
- Tauri events 推送到前端

### T2.3 仪表盘图表增强
- 月度盈亏走势：SVG 柱状图 + 折线（手写 < 300 行）
- 个股盈亏排行：表格 + 颜色条
- 持仓占比饼图
- 收益分布直方图

### T2.4 复盘对话增强
- `/trade-review` 自动注入大盘行情（Phase 2 新增）
- 复盘笔记模板增强（五段式 markdown 模板）

### Phase 2 验收标准
- [ ] OCR 对 20 张同花顺截图字段级准确率 >= 85%
- [ ] 行情刷新不卡 UI
- [ ] 持仓浮动盈亏计算正确
- [ ] 图表数据与 FIFO 引擎输出一致

---

## Phase 3：扩展 — 知识图谱 + AI 研报（按需启动）

### T3.1 交易笔记
**文件：** `src/routes/trading/+page.svelte` 的笔记 tab

- 目录结构：`notes/trading/日复盘/`、`股票/`、`概念/`、`策略/`
- 左侧列表 + 右侧 Markdown 编辑器 + 实时预览
- 模板选择器（五段复盘、个股研究、策略笔记）
- 复用 Explorer 文件浏览

### T3.2 知识图谱
**文件：** `src/routes/trading/+page.svelte` 的图谱 tab

- Canvas 力导向布局（< 100 节点用纯 Canvas，> 100 考虑 D3-force）
- 节点：股票、概念、策略、错误、复盘、预测
- 关系：属于、涉及、发生于、关联、验证于、引用
- 洞察面板：惊喜连接、知识缺口、模式发现
- 从复盘笔记和交易记录中自动提取实体和关系

### T3.3 AI 研报 Room 集成
- 手动触发（选股票 → "生成研报"）
- Room 三角色：数据采集员（东方财富数据）→ 研报撰写员 → 审核员
- 新增 `orchestrator.rs` 中的 "research-report" 编排路径
- 研报模板输出：基本面、行业竞争、技术面、资金面、催化剂、风险、评级
- 保存至 `notes/trading/股票/研报-{股票名}-{日期}.md`

### Phase 3 验收标准
- [ ] 图谱 < 100 节点时 Canvas 渲染 60fps
- [ ] Room 三角色成功完成一次完整研报生命周期
- [ ] 研报内容充实可用

---

## 技术决策记录

| # | 决策 | 理由 |
|---|------|------|
| 1 | FIFO 引擎放 Rust 端 | 金融精度（`rust_decimal`）、跨组件一致性（仪表盘+Room+存储共享） |
| 2 | 前端图表用纯 SVG/Canvas | Recharts 是 React 库，Svelte 5 不兼容；手写 SVG < 300 行 |
| 3 | Phase 1 不做 OCR | WinRT OCR 依赖未知，需原型验证；先做 CSV/手动录入 MVP |
| 4 | 截图解析独立按钮触发 | 避免侵入式修改 Chat 消息流水线 |
| 5 | `/trading` 用 tab 切换 | 避免子路由导致的布局重挂载，留存实时数据 |
| 6 | Windows 专属（Phase 1） | `#[cfg(target_os = "windows")]` 条件编译；macOS 不显示 |
| 7 | 自建东方财富 HTTP 客户端 | mx-data/mx-search 是 Claude Code IDE skill，不可在应用内复用 |
| 8 | 笔记独立目录 | `notes/trading/` 独立于 Memo 系统，避免混淆 |
| 9 | 个人策略文件 `~/.claw-go/A-ShareStrategy.md` | 用户级全局策略，所有交易对话自动注入 |

---

## 集成影响评估

| 现有系统 | 影响 | 说明 |
|---------|------|------|
| Chat session | 中 | `/trade-review` 命令 + 复盘 context 注入 |
| Room orchestrator | 低(Phase 1) / 中(Phase 3) | 研报流程需要新编排路径 |
| Memo store | 无 | 交易笔记独立目录，不冲突 |
| Explorer | 低 | `notes/trading/` 可通过 Explorer 浏览 |
| Sidebar/Layout | 中 | 条件 navItem 渲染 |
| Settings | 中 | 新增 A 股分析开关 + `trading_enabled` 字段 |
| i18n | 低 | 新增 ~20 key（Phase 1），后续递增 |
| Build | 低 | 新增 `rust_decimal` crate；不引入前端图表库 |
| macOS 构建 | 无 | Windows 条件编译，macOS 不受影响 |
