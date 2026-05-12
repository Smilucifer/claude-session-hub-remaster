# Managed Config Injection Design

**Status:** [done]
**Date:** 2026-05-13
**Phase:** 10.a

## Problem Statement

The Plugins page has three confirmed bugs:

1. **MCP persistence bug (P0)**: `update_user_settings()` in `storage/settings.rs` ignores the `mcp_servers` patch field. Managed MCP servers never persist to disk. UI shows "success" but data is silently lost.
2. **Plugins marketplace install state**: Marketplace cards don't cross-reference `installedPlugins` array — always shows "Install" even for already-installed plugins.
3. **Hooks data fragility**: `HookManager.svelte` writes hooks directly to `~/.claude/settings.json` via `updateCliConfig()`. This file can be overwritten by CLI updates or external tools, causing hooks to vanish.

## Design: Three-Layer Config Model

Session JSON generation (`provider_config_json_from_env()`) already follows an implicit three-layer model. This design makes it explicit:

```
┌─────────────────────────────────────────────────┐
│           Session JSON (--settings)              │
│  ┌───────────────────────────────────────────┐  │
│  │  Layer 3: Provider overrides (forced)     │  │
│  │  permissions, language, thinking, etc.    │  │
│  └───────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────┐  │
│  │  Layer 2: Claw GO Managed (additive)      │  │
│  │  UserSettings.mcp_servers → mcpServers    │  │
│  │  UserSettings.hooks       → hooks         │  │
│  │  UserSettings.enabled_plugins → plugins   │  │
│  └───────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────┐  │
│  │  Layer 1: Native CLI base                 │  │
│  │  ~/.claude/settings.json (read-only base) │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

### Merge Strategy

| Config type | Native structure | Managed structure | Merge rule |
|-------------|-----------------|-------------------|------------|
| **mcpServers** | `{name: config}` in `.claude.json` | `{name: config}` in `UserSettings.mcp_servers` | additive: managed overwrites same-name |
| **hooks** | `{event: [groups]}` in `settings.json` | `{event: [groups]}` in `UserSettings.hooks` | **managed overwrites native per-event** |
| **enabledPlugins** | `{name: bool}` in `settings.json` | `{name: bool}` in `UserSettings.enabled_plugins` | overlay: managed overwrites same-name. Provider forced fields (superpowers) execute last. |

**Hooks merge decision**: Per-event overwrite (NOT concat). If managed defines hooks for `PreToolUse`, the native `PreToolUse` hooks are ignored entirely. This avoids double-execution during migration and matches the MCP "managed overwrites same-name" semantics.

## Implementation Plan

### Phase 1: Fix Confirmed Bugs (MVP)

#### Step 1.1: Fix MCP persistence

**File**: `src-tauri/src/storage/settings.rs`

In `update_user_settings()` (line ~725), add `mcp_servers` patch handler:

```rust
if let Some(v) = patch.get("mcp_servers") {
    if v.is_null() {
        all.user.mcp_servers = std::collections::HashMap::new();
    } else {
        all.user.mcp_servers = serde_json::from_value(v.clone())
            .map_err(|e| format!("Invalid mcp_servers: {}", e))?;
    }
}
```

**Verification**: `add_managed_mcp_server` -> `get_user_settings().mcp_servers` contains new server -> persists after reload.

#### Step 1.2: Fix Plugins marketplace install state

**File**: `src/routes/plugins/+page.svelte`

In the marketplace card renderer (~line 1748), add cross-reference:

```svelte
{@const isInstalled = installedPlugins.some(p => p.name === plugin.name)}
```

Conditionally render "Installed" (disabled) vs "Install" button.

**Verification**: Installed plugins show "Installed" in marketplace view.

### Phase 2: Refactor `update_user_settings()` + Extend Data Model

#### Step 2.1: Refactor `update_user_settings()` into field dispatcher

**File**: `src-tauri/src/storage/settings.rs`

Current function is 190 lines of `if let Some(v) = patch.get("field")` blocks. Refactor into per-field handler functions called from a central dispatcher. Each handler is a small, testable function.

Pattern:
```rust
pub fn update_user_settings(patch: serde_json::Value) -> Result<UserSettings, String> {
    let mut all = load();
    apply_default_agent(&mut all.user, &patch)?;
    apply_platform_credentials(&mut all.user, &patch)?;
    apply_mcp_servers(&mut all.user, &patch)?;
    apply_hooks(&mut all.user, &patch)?;
    apply_enabled_plugins(&mut all.user, &patch)?;
    // ... all other fields ...
    all.user.updated_at = crate::models::now_iso();
    save(&all)?;
    Ok(all.user)
}
```

Each `apply_*` function handles one field, following the existing pattern:
```rust
fn apply_mcp_servers(user: &mut UserSettings, patch: &serde_json::Value) -> Result<(), String> {
    if let Some(v) = patch.get("mcp_servers") {
        if v.is_null() {
            user.mcp_servers = HashMap::new();
        } else {
            user.mcp_servers = serde_json::from_value(v.clone())
                .map_err(|e| format!("Invalid mcp_servers: {}", e))?;
        }
    }
    Ok(())
}
```

**Verification**: All existing tests pass. No behavior change — pure refactor.

#### Step 2.2: Add `hooks` and `enabled_plugins` to `UserSettings`

**File**: `src-tauri/src/models.rs`

```rust
/// Managed hook configs (event name → array of hook groups).
/// Overwrites native hooks per-event in generated session JSON.
#[serde(default, skip_serializing_if = "HashMap::is_empty")]
pub hooks: HashMap<String, serde_json::Value>,

/// Managed plugin enable/disable states (plugin name → enabled).
/// Overwrites native enabledPlugins in generated session JSON.
#[serde(default, skip_serializing_if = "HashMap::is_empty")]
pub enabled_plugins: HashMap<String, bool>,
```

Both fields use `#[serde(default)]` for backward compatibility with existing settings files.

**Verification**: `cargo check` passes. Old settings.json deserializes correctly.

#### Step 2.3: Add patch handlers for new fields

**File**: `src-tauri/src/storage/settings.rs`

```rust
fn apply_hooks(user: &mut UserSettings, patch: &serde_json::Value) -> Result<(), String> {
    if let Some(v) = patch.get("hooks") {
        if v.is_null() {
            user.hooks = HashMap::new();
        } else {
            user.hooks = serde_json::from_value(v.clone())
                .map_err(|e| format!("Invalid hooks: {}", e))?;
        }
    }
    Ok(())
}

fn apply_enabled_plugins(user: &mut UserSettings, patch: &serde_json::Value) -> Result<(), String> {
    if let Some(v) = patch.get("enabled_plugins") {
        if v.is_null() {
            user.enabled_plugins = HashMap::new();
        } else {
            user.enabled_plugins = serde_json::from_value(v.clone())
                .map_err(|e| format!("Invalid enabled_plugins: {}", e))?;
        }
    }
    Ok(())
}
```

**Verification**: `cargo check` passes.

#### Step 2.4: Add CRUD commands for hooks and plugins

**File**: `src-tauri/src/commands/settings.rs`

Mirror the MCP commands pattern:

```rust
#[tauri::command]
pub fn list_managed_hooks() -> HashMap<String, serde_json::Value> { ... }

#[tauri::command]
pub fn add_managed_hook(event: String, groups_json: String) -> Result<PluginOperationResult, String> { ... }

#[tauri::command]
pub fn remove_managed_hook(event: String) -> Result<PluginOperationResult, String> { ... }

#[tauri::command]
pub fn list_managed_plugins() -> HashMap<String, bool> { ... }

#[tauri::command]
pub fn set_managed_plugin(name: String, enabled: bool) -> Result<PluginOperationResult, String> { ... }

#[tauri::command]
pub fn remove_managed_plugin(name: String) -> Result<PluginOperationResult, String> { ... }
```

Register all new commands in `src-tauri/src/lib.rs`.

**Verification**: `cargo check` passes. Commands are callable from frontend.

### Phase 3: Injection + Frontend Migration

#### Step 3.1: Introduce `ManagedConfig` struct

**File**: `src-tauri/src/agent/provider_claude_config.rs`

```rust
pub struct ManagedConfig<'a> {
    pub mcp_servers: &'a HashMap<String, serde_json::Value>,
    pub hooks: &'a HashMap<String, serde_json::Value>,
    pub enabled_plugins: &'a HashMap<String, bool>,
}
```

Replace the current `mcp_servers` parameter in `provider_config_json_from_env()`:

```rust
fn provider_config_json_from_env(
    env: &HashMap<String, String>,
    managed: &ManagedConfig,
) -> Value { ... }
```

Update all call sites:
- `write_provider_claude_config()` — build `ManagedConfig` from user_settings
- `write_mcp_only_settings()` — build `ManagedConfig` with empty hooks/plugins

**Verification**: `cargo check` passes. Existing MCP merge test still passes.

#### Step 3.2: Add hooks and plugins merge logic

**File**: `src-tauri/src/agent/provider_claude_config.rs`

In `provider_config_json_from_env()`, after the existing MCP merge block:

```rust
// hooks: managed overwrites native per-event (not concat).
if !managed.hooks.is_empty() {
    let existing = obj
        .get("hooks")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mut merged = existing;
    for (event, groups) in managed.hooks {
        merged.insert(event.clone(), groups.clone());
    }
    obj.insert("hooks".to_string(), Value::Object(merged));
}

// enabledPlugins: managed overlay, but provider forced fields execute last.
if !managed.enabled_plugins.is_empty() {
    let existing = obj
        .get("enabledPlugins")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mut merged = existing;
    for (name, enabled) in managed.enabled_plugins {
        merged.insert(name.clone(), Value::Bool(*enabled));
    }
    obj.insert("enabledPlugins".to_string(), Value::Object(merged));
}

// Force superpowers plugin (AFTER managed overlay — cannot be disabled).
let plugins = obj
    .entry("enabledPlugins".to_string())
    .or_insert_with(|| Value::Object(Map::new()));
if let Some(plugins_obj) = plugins.as_object_mut() {
    plugins_obj.insert("superpowers@claude-plugins-official".to_string(), Value::Bool(true));
}
```

**Verification**: New tests — `provider_config_merges_managed_hooks`, `provider_config_merges_managed_plugins`, `provider_config_managed_hooks_overwrite_native_per_event`.

#### Step 3.3: Migrate HookManager to Claw GO API

**File**: `src/lib/components/HookManager.svelte`

Replace `getCliConfig()`/`updateCliConfig()` with new Claw GO hooks API:
- `loadConfig()` → `listManagedHooks()`
- `saveHooks()` → iterate events, call `addManagedHook()` / `removeManagedHook()`

Add i18n keys for new API calls.

**File**: `src/lib/api.ts`

Add frontend API wrappers:
```typescript
export async function listManagedHooks(): Promise<Record<string, unknown[]>> { ... }
export async function addManagedHook(event: string, groupsJson: string): Promise<PluginOperationResult> { ... }
export async function removeManagedHook(event: string): Promise<PluginOperationResult> { ... }
export async function listManagedPlugins(): Promise<Record<string, boolean>> { ... }
export async function setManagedPlugin(name: string, enabled: boolean): Promise<PluginOperationResult> { ... }
export async function removeManagedPlugin(name: string): Promise<PluginOperationResult> { ... }
```

**Verification**: HookManager UI loads managed hooks. Add/edit/remove works.

#### Step 3.4: One-time migration from native hooks

**File**: `src-tauri/src/commands/settings.rs` or startup hook

On first launch after upgrade:
1. Read `~/.claude/settings.json` for existing `hooks` key
2. If `hooks` exists AND `UserSettings.hooks` is empty → import native hooks into managed storage
3. Remove `hooks` key from `~/.claude/settings.json` via `update_cli_config()`

This is a one-time operation. Use a flag in `UserSettings` (e.g., `hooks_migrated: bool`) to track.

**Verification**: Old user with native hooks → hooks appear in managed UI after upgrade. Native file no longer has hooks key.

## Key Files

| File | Changes |
|------|---------|
| `src-tauri/src/models.rs` | Add `hooks`, `enabled_plugins` fields to `UserSettings` |
| `src-tauri/src/storage/settings.rs` | Refactor `update_user_settings()` into field dispatcher; add patch handlers |
| `src-tauri/src/commands/settings.rs` | Add hooks/plugins CRUD commands; migration logic |
| `src-tauri/src/agent/provider_claude_config.rs` | `ManagedConfig` struct; hooks/plugins merge logic |
| `src-tauri/src/lib.rs` | Register new Tauri commands |
| `src/lib/components/HookManager.svelte` | Switch to Claw GO hooks API |
| `src/routes/plugins/+page.svelte` | Marketplace install state fix |
| `src/lib/api.ts` | New frontend API wrappers |
| `messages/en.json`, `messages/zh-CN.json` | New i18n keys |

## Verification Checklist

- [x] MCP: add → reload → server persists
- [x] Plugins: installed plugin shows "Installed" in marketplace
- [x] Hooks: managed hooks load, add, edit, remove via UI
- [x] Hooks: managed hooks overwrite native per-event in session JSON
- [x] Plugins: managed enabledPlugins overlay native in session JSON
- [x] superpowers: forced injection (insert, not or_insert) after managed overlay
- [x] Migration: native hooks auto-imported to managed on first launch (`migrate_native_hooks()` in `hooks/setup.rs`)
- [x] Backward compat: old settings.json (no hooks/enabled_plugins) deserializes
- [x] `cargo check` passes
- [x] `npm run build` passes
- [x] `npm run i18n:check` passes

## Code Review (2026-05-13)

4 providers: Claude, DeepSeek, MiMo Plan, Packy CX2CC.

**Fixes applied from review:**
1. Superpowers `or_insert` → `insert` — ensured forced override cannot be bypassed by managed config
2. Test compilation error: `&HashMap::new()` → `&empty_managed()` in `writes_temp_config_with_run_id_in_path`
3. `write_mcp_only_settings` doc comment updated to reflect actual behavior
4. `handleSaveEditor` empty array now calls `deleteEventHooks` instead of silent skip
5. `deleteEventHooks` toast uses dedicated `hooks_deleted` i18n key

**Deferred:**
- Backend event name validation in `add_managed_hook`
- Tests for hooks/plugins merge logic and IPC commands

**Post-review addition:**
- One-time native hooks migration (Step 3.4) — implemented in `hooks/setup.rs::migrate_native_hooks()`, called at startup in `lib.rs`. Uses `native_hooks_migrated: bool` flag in `UserSettings`.

## Code Review — Round 2 (2026-05-13)

4 providers: Claude, DeepSeek, MiMo Plan, Packy CX2CC. Focus: native hooks migration + full diff.

**Fixes applied:**
1. `migrate_native_hooks` save() errors: 3 处 `let _ = save()` 改为 `if let Err(e)` + `log::warn!`，save 失败时 `return` 不移除 native hooks（自愈：下次启动重试）
2. `write_mcp_only_settings` 重命名为 `write_managed_settings`，更新 `session.rs` 调用点
3. startup 顺序添加注释说明 `cleanup_hook_bridge` → `migrate_native_hooks` 依赖关系
