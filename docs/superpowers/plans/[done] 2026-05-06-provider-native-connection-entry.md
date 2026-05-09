# Provider-Native Connection Entry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add first-class DeepSeek, GLM, QWEN, and KIMI provider entries with provider-native launch-config generation and provider-card settings UI while preserving Claude as the execution engine.

**Architecture:** Keep provider identity separate from execution identity. Implement a Rust provider-native launch-config builder boundary that intercepts DeepSeek and parameterized providers before generic provider-default env injection, then update the Svelte provider catalog and settings/chat/room surfaces to expose the new providers and required fields. Persist dynamic provider fields in existing `platform_credentials` records instead of introducing a new storage system.

**Tech Stack:** SvelteKit, Svelte 5 runes, Tauri commands, Rust serde models, existing settings storage/migration helpers, Vitest, Rust unit tests.

**Execution status (2026-05-06):**
- Task 1 completed: expanded provider metadata/defaults for DeepSeek, GLM, QWEN, and KIMI; updated DeepSeek presets to v4 models; added legacy `deepseek-chat` migration logic.
- Task 2 completed at source level: added provider-native launch-config builder wiring in `src-tauri/src/commands/session.rs` and extended CLI-mode platform preservation for `bailian` and `kimi`.
- Task 3 completed: settings-page provider cards now treat DeepSeek as fixed-model API config and GLM/QWEN/KIMI as parameterized `api_key + base_url + model` providers.
- Task 4 completed: room UI labels, continuable-run lookup, and room participant creation cover QWEN/KIMI.
- Task 5 completed as far as environment allows: targeted frontend tests pass; targeted Rust tests compile but executable launch remains blocked on this machine by `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)`.

**Follow-up status (2026-05-09):**
- Added a second-stage implementation focused on third-party session-provider validation readiness.
- `src-tauri/src/agent/provider_claude_config.rs` now exposes structured provider validation results and reuses the same rule source for settings-page validation and provider env generation.
- `src-tauri/src/commands/settings.rs` + `src-tauri/src/lib.rs` now expose a `validate_platform_credentials` IPC for the connection page.
- `src/routes/settings/+page.svelte` now includes an “应用并校验配置” flow with provider-level result rendering.
- Xiaomi connection UX was tightened: `mimo-plan` and `mimo-api` keep separate key / base_url storage but share one model-configuration surface whose values are dual-written into both credentials.
- DeepSeek and Packy follow the same validation system without adding new connection fields; their page copy now explicitly states that complete explicit model configuration is required.

---

## File Structure

- Modify: `src/lib/utils/provider-catalog.ts`
  - Add QWEN and KIMI as first-class providers, update DeepSeek and GLM defaults, and formalize which providers are fixed-template vs parameterized-template.
- Modify: `src/lib/utils/provider-catalog.test.ts`
  - Cover provider list, defaults, and provider-id mapping.
- Modify: `src/lib/utils/platform-presets.ts`
  - Update DeepSeek models and ensure QWEN/KIMI defaults are usable as parameterized provider defaults.
- Modify: `src/routes/settings/+page.svelte`
  - Expand the provider-card UI to cover DeepSeek, GLM, QWEN, and KIMI with per-provider field rules.
- Modify: `src/routes/chat/+page.svelte`
  - Teach the chat entry/status logic to recognize the two new parameterized providers and read their credentials.
- Modify: `src/routes/rooms/+page.svelte`
  - Ensure room participant provider selection includes QWEN and KIMI.
- Modify: `src/lib/components/AgentSelector.svelte`
  - Ensure shared provider pickers render the expanded provider list correctly.
- Modify: `src/lib/stores/room-store.svelte.ts`
  - Preserve provider identity when creating room participants for QWEN/KIMI.
- Modify: `src/lib/stores/room-store.test.ts`
  - Cover room participant creation for new providers if needed.
- Modify: `src/lib/utils/room-ui.ts`
  - Render labels for QWEN/KIMI provider-backed Claude runs.
- Modify: `src/lib/utils/room-ui.test.ts`
  - Add label assertions for QWEN/KIMI.
- Modify: `src/lib/utils/continuable-run.test.ts`
  - Cover provider-id restoration for QWEN/KIMI and updated DeepSeek defaults.
- Modify: `src-tauri/src/storage/settings.rs`
  - Update provider defaults, add migration for DeepSeek model rename, and expose QWEN/KIMI defaults through existing settings helpers.
- Modify: `src-tauri/src/models.rs`
  - Extend any typed provider-id or settings metadata needed by the new providers without changing execution-agent identity.
- Modify: `src-tauri/src/commands/session.rs`
  - Add provider-native launch-config builders and route DeepSeek/GLM/QWEN/KIMI through them.
- Modify: `src-tauri/src/agent/claude_stream.rs`
  - Apply the generated full launch config when spawning Claude-native sessions if the current path only consumes loose auth/env fragments.
- Modify: `src-tauri/src/room/adapter.rs`
  - Ensure room-backed Claude launches preserve provider-native config semantics.
- Modify: `src-tauri/src/storage/runs.rs`
  - Preserve explicit provider/platform identity for resume/continue and inherited launches when needed.
- Modify: `src-tauri/src/commands/onboarding.rs`
  - Render readable provider names for QWEN/KIMI if onboarding/status surfaces expose these labels.
- Modify: `messages/en.json`
- Modify: `messages/zh-CN.json`
  - Add any provider-card copy or validation text needed for the two new providers and revised DeepSeek wording.

---

### Task 1: Expand provider metadata and defaults

**Files:**
- Modify: `src/lib/utils/provider-catalog.ts`
- Modify: `src/lib/utils/provider-catalog.test.ts`
- Modify: `src/lib/utils/platform-presets.ts`
- Modify: `src-tauri/src/storage/settings.rs`
- Test: `src/lib/utils/provider-catalog.test.ts`

- [ ] **Step 1: Write the failing frontend provider-catalog tests**

```ts
it("includes DeepSeek, GLM, QWEN, and KIMI as phase 7 providers", () => {
  expect(PHASE7_PROVIDERS.map((provider) => provider.id)).toEqual([
    "claude",
    "codex",
    "gemini",
    "deepseek",
    "glm",
    "qwen",
    "kimi",
  ]);
});

it("keeps DeepSeek constrained to v4 models", () => {
  expect(getPhase7Provider("deepseek")).toMatchObject({
    defaultModel: "deepseek-v4-pro",
    requiredConfig: ["api_key"],
  });
});

it("treats QWEN and KIMI as parameterized Claude-compatible providers", () => {
  expect(getPhase7Provider("qwen")).toMatchObject({
    mode: "claude_compatible_api",
    executionAgent: "claude",
    requiredConfig: ["api_key", "base_url", "model"],
  });
  expect(getPhase7Provider("kimi")).toMatchObject({
    mode: "claude_compatible_api",
    executionAgent: "claude",
    requiredConfig: ["api_key", "base_url", "model"],
  });
});

it("maps provider-backed Claude runs back to provider ids", () => {
  expect(providerIdForRun("claude", "deepseek")).toBe("deepseek");
  expect(providerIdForRun("claude", "zhipu")).toBe("glm");
  expect(providerIdForRun("claude", "bailian")).toBe("qwen");
  expect(providerIdForRun("claude", "kimi")).toBe("kimi");
});
```

- [ ] **Step 2: Run the provider-catalog tests and verify they fail**

Run: `npm test -- src/lib/utils/provider-catalog.test.ts`
Expected: FAIL because `qwen`/`kimi` entries and updated DeepSeek defaults are not defined yet.

- [ ] **Step 3: Update the provider catalog and preset defaults**

```ts
export type Phase7ProviderId =
  | "claude"
  | "codex"
  | "gemini"
  | "deepseek"
  | "glm"
  | "qwen"
  | "kimi";

{
  id: "deepseek",
  label: "DeepSeek",
  mode: "claude_compatible_api",
  executionAgent: "claude",
  platformId: "deepseek",
  defaultModel: "deepseek-v4-pro",
  defaultBaseUrl: "https://api.deepseek.com/anthropic",
  requiredConfig: ["api_key"],
  defaultPermissionMode: "bypass",
},
{
  id: "qwen",
  label: "QWEN",
  mode: "claude_compatible_api",
  executionAgent: "claude",
  platformId: "bailian",
  defaultModel: "qwen3.5-plus",
  defaultBaseUrl: "https://coding.dashscope.aliyuncs.com/apps/anthropic",
  requiredConfig: ["api_key", "base_url", "model"],
  defaultPermissionMode: "bypass",
},
{
  id: "kimi",
  label: "KIMI",
  mode: "claude_compatible_api",
  executionAgent: "claude",
  platformId: "kimi",
  defaultModel: "kimi-k2.5",
  defaultBaseUrl: "https://api.moonshot.cn/anthropic",
  requiredConfig: ["api_key", "base_url", "model"],
  defaultPermissionMode: "bypass",
},

export function providerIdForRun(agent: string, platformId?: string | null): Phase7ProviderId {
  if (platformId === "deepseek") return "deepseek";
  if (platformId === "zhipu" || platformId === "zhipu-intl") return "glm";
  if (platformId === "bailian") return "qwen";
  if (platformId === "kimi") return "kimi";
  if (agent === "codex" || agent === "gemini") return agent;
  return "claude";
}
```

```ts
{
  id: "deepseek",
  name: "DeepSeek",
  base_url: "https://api.deepseek.com/anthropic",
  auth_env_var: "ANTHROPIC_AUTH_TOKEN",
  description: "DeepSeek API",
  key_placeholder: "your-deepseek-key",
  category: "provider",
  models: ["deepseek-v4-pro", "deepseek-v4-flash"],
  extra_env: { API_TIMEOUT_MS: "600000" },
}
```

```rust
"deepseek" => Some(ProviderDefaults {
    base_url: Some("https://api.deepseek.com/anthropic"),
    models: Some(vec![
        "deepseek-v4-pro".to_string(),
        "deepseek-v4-flash".to_string(),
    ]),
    extra_env: Some(HashMap::from([(
        "API_TIMEOUT_MS".to_string(),
        "600000".to_string(),
    )])),
    key_optional: false,
    auth_env_var: None,
}),
"kimi" => Some(ProviderDefaults {
    base_url: Some("https://api.moonshot.cn/anthropic"),
    models: Some(vec!["kimi-k2.5".to_string(), "kimi-k2".to_string()]),
    extra_env: None,
    key_optional: false,
    auth_env_var: None,
}),
"bailian" => Some(ProviderDefaults {
    base_url: Some("https://coding.dashscope.aliyuncs.com/apps/anthropic"),
    models: Some(vec![
        "qwen3-max".to_string(),
        "qwen3.5-plus".to_string(),
        "qwen-plus".to_string(),
        "qwen-flash".to_string(),
    ]),
    extra_env: None,
    key_optional: false,
    auth_env_var: None,
}),
```

- [ ] **Step 4: Add the DeepSeek model migration test**

```rust
#[test]
fn migrate_platform_credentials_rewrites_legacy_deepseek_chat_model() {
    let mut settings = AllSettings::default();
    settings.user.platform_credentials.push(PlatformCredential {
        platform_id: "deepseek".to_string(),
        api_key: Some("sk-test".to_string()),
        model: Some("deepseek-chat".to_string()),
        ..PlatformCredential::default()
    });

    let changed = migrate_platform_credentials(&mut settings);

    assert!(changed);
    assert_eq!(
        settings.user.platform_credentials[0].model.as_deref(),
        Some("deepseek-v4-pro")
    );
}
```

- [ ] **Step 5: Implement the migration**

```rust
if cred.platform_id == "deepseek" && cred.model.as_deref() == Some("deepseek-chat") {
    log::info!(
        "[storage/settings] migrating deepseek model from deepseek-chat to deepseek-v4-pro"
    );
    cred.model = Some("deepseek-v4-pro".to_string());
    changed = true;
}
```

- [ ] **Step 6: Run the focused tests and verify they pass**

Run: `npm test -- src/lib/utils/provider-catalog.test.ts && cargo test --manifest-path src-tauri/Cargo.toml storage::settings::tests:: -- --nocapture`
Expected: PASS for the updated provider-catalog tests and the new migration coverage.

- [ ] **Step 7: Commit**

```bash
git add src/lib/utils/provider-catalog.ts src/lib/utils/provider-catalog.test.ts src/lib/utils/platform-presets.ts src-tauri/src/storage/settings.rs
git commit -m "feat: add provider metadata for qwen and kimi"
```

### Task 2: Build provider-native launch-config generation

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands/session.rs`
- Modify: `src-tauri/src/agent/claude_stream.rs`
- Modify: `src-tauri/src/room/adapter.rs`
- Test: `src-tauri/src/commands/session.rs`

- [ ] **Step 1: Write the failing Rust tests for provider-native launch config**

```rust
#[test]
fn build_deepseek_launch_config_injects_saved_api_key_and_fixed_models() {
    let settings = make_settings_with_cred(
        "deepseek",
        Some("sk-deepseek"),
        Some("https://api.deepseek.com/anthropic"),
        Some("deepseek-v4-pro"),
    );

    let config = build_provider_launch_config(&settings, "deepseek").unwrap();

    assert_eq!(
        config.env.get("ANTHROPIC_AUTH_TOKEN").map(String::as_str),
        Some("sk-deepseek")
    );
    assert_eq!(
        config.env.get("ANTHROPIC_MODEL").map(String::as_str),
        Some("deepseek-v4-pro")
    );
    assert_eq!(
        config.env.get("ANTHROPIC_DEFAULT_HAIKU_MODEL").map(String::as_str),
        Some("deepseek-v4-flash")
    );
    assert_eq!(config.language.as_deref(), Some("简体中文"));
}

#[test]
fn build_parameterized_launch_config_uses_saved_base_url_and_model() {
    let settings = make_settings_with_cred(
        "bailian",
        Some("sk-qwen"),
        Some("https://custom.qwen.example/anthropic"),
        Some("qwen3.5-plus"),
    );

    let config = build_provider_launch_config(&settings, "bailian").unwrap();

    assert_eq!(
        config.env.get("ANTHROPIC_BASE_URL").map(String::as_str),
        Some("https://custom.qwen.example/anthropic")
    );
    assert_eq!(
        config.env.get("ANTHROPIC_MODEL").map(String::as_str),
        Some("qwen3.5-plus")
    );
    assert_eq!(
        config.env.get("ANTHROPIC_DEFAULT_OPUS_MODEL").map(String::as_str),
        Some("qwen3.5-plus")
    );
}
```

- [ ] **Step 2: Run the Rust session tests and verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::session::tests:: -- --nocapture`
Expected: FAIL because the provider launch-config builder types/functions do not exist yet.

- [ ] **Step 3: Add a typed launch-config model**

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaudeNativeLaunchConfig {
    pub env: HashMap<String, String>,
    pub include_co_authored_by: Option<bool>,
    pub thinking: Option<bool>,
    pub permissions_default_mode: Option<String>,
    pub skip_dangerous_mode_permission_prompt: Option<bool>,
    pub enabled_plugins: Option<HashMap<String, bool>>,
    pub auto_updates_channel: Option<String>,
    pub language: Option<String>,
}
```

- [ ] **Step 4: Implement the DeepSeek and parameterized builders**

```rust
fn build_deepseek_launch_config(cred: &PlatformCredential) -> Result<ClaudeNativeLaunchConfig, String> {
    let api_key = cred
        .api_key
        .clone()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| "DeepSeek API key is not configured".to_string())?;

    Ok(ClaudeNativeLaunchConfig {
        env: HashMap::from([
            ("ANTHROPIC_BASE_URL".to_string(), "https://api.deepseek.com/anthropic".to_string()),
            ("ANTHROPIC_AUTH_TOKEN".to_string(), api_key),
            ("ANTHROPIC_MODEL".to_string(), "deepseek-v4-pro".to_string()),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), "deepseek-v4-pro".to_string()),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), "deepseek-v4-pro".to_string()),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), "deepseek-v4-flash".to_string()),
            ("CLAUDE_CODE_SUBAGENT_MODEL".to_string(), "deepseek-v4-pro".to_string()),
            ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(), "1".to_string()),
            ("CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK".to_string(), "1".to_string()),
            ("CLAUDE_CODE_EFFORT_LEVEL".to_string(), "max".to_string()),
            ("CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS".to_string(), "true".to_string()),
            ("CLAUDE_CODE_AUTO_COMPACT_WINDOW".to_string(), "400000".to_string()),
        ]),
        include_co_authored_by: Some(false),
        thinking: Some(false),
        permissions_default_mode: Some("bypassPermissions".to_string()),
        skip_dangerous_mode_permission_prompt: Some(true),
        enabled_plugins: Some(HashMap::from([(
            "superpowers@claude-plugins-official".to_string(),
            true,
        )])),
        auto_updates_channel: Some("latest".to_string()),
        language: Some("简体中文".to_string()),
    })
}

fn build_parameterized_provider_launch_config(
    cred: &PlatformCredential,
) -> Result<ClaudeNativeLaunchConfig, String> {
    let api_key = cred
        .api_key
        .clone()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| format!("{} API key is not configured", cred.platform_id))?;
    let base_url = cred
        .base_url
        .clone()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| format!("{} base URL is not configured", cred.platform_id))?;
    let model = cred
        .model
        .clone()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| format!("{} model is not configured", cred.platform_id))?;

    Ok(ClaudeNativeLaunchConfig {
        env: HashMap::from([
            ("ANTHROPIC_BASE_URL".to_string(), base_url),
            ("ANTHROPIC_AUTH_TOKEN".to_string(), api_key),
            ("ANTHROPIC_MODEL".to_string(), model.clone()),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), model.clone()),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), model.clone()),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), model),
        ]),
        permissions_default_mode: Some("bypassPermissions".to_string()),
        enabled_plugins: Some(HashMap::from([(
            "superpowers@claude-plugins-official".to_string(),
            true,
        )])),
        auto_updates_channel: Some("latest".to_string()),
        language: Some("简体中文".to_string()),
        ..ClaudeNativeLaunchConfig::default()
    })
}
```

- [ ] **Step 5: Route supported providers through the builder boundary**

```rust
fn build_provider_launch_config(
    settings: &AllSettings,
    platform_id: &str,
) -> Result<Option<ClaudeNativeLaunchConfig>, String> {
    let cred = settings
        .user
        .platform_credentials
        .iter()
        .find(|cred| cred.platform_id == platform_id)
        .ok_or_else(|| format!("No credential found for platform '{}'", platform_id))?;

    match platform_id {
        "deepseek" => Ok(Some(build_deepseek_launch_config(cred)?)),
        "zhipu" | "zhipu-intl" | "bailian" | "kimi" => {
            Ok(Some(build_parameterized_provider_launch_config(cred)?))
        }
        _ => Ok(None),
    }
}
```

```rust
let provider_launch_config = effective_pid
    .map(|pid| build_provider_launch_config(&user_settings, pid))
    .transpose()?;
```

- [ ] **Step 6: Apply provider launch config to Claude-native spawn env**

```rust
if let Some(config) = provider_launch_config.as_ref() {
    for (key, value) in &config.env {
        resolved.extra_env
            .get_or_insert_with(HashMap::new)
            .insert(key.clone(), value.clone());
    }
}
```

```rust
if let Some(config) = provider_launch_config {
    merge_extra_env_into_spawn_env_plan(&mut plan, &config.env);
}
```

- [ ] **Step 7: Run the focused Rust tests and verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::session::tests:: -- --nocapture`
Expected: PASS for the new provider launch-config tests and existing session routing tests.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/commands/session.rs src-tauri/src/agent/claude_stream.rs src-tauri/src/room/adapter.rs
git commit -m "feat: build native launch config for api providers"
```

### Task 3: Redesign the settings provider cards

**Files:**
- Modify: `src/routes/settings/+page.svelte`
- Modify: `messages/en.json`
- Modify: `messages/zh-CN.json`
- Test: `src/routes/settings/+page.svelte` related tests if present, otherwise `npm run check`

- [ ] **Step 1: Add a failing settings-surface assertion if a UI test exists; otherwise capture the expected provider-card data shape in code**

```ts
const providerFields = $derived.by(() =>
  PHASE7_PROVIDERS.map((provider) => ({
    id: provider.id,
    requiredConfig: provider.requiredConfig,
  }))
);

expect(providerFields).toContainEqual({
  id: "qwen",
  requiredConfig: ["api_key", "base_url", "model"],
});
```

- [ ] **Step 2: Run the current frontend validation and verify the provider-card surface is not ready**

Run: `npm run check`
Expected: FAIL or incomplete behavior for QWEN/KIMI because the settings page only knows the earlier provider set.

- [ ] **Step 3: Refactor provider-card field rules in the settings page**

```ts
function providerVisibleFields(provider: Phase7ProviderEntry) {
  if (provider.id === "deepseek") {
    return {
      showApiKey: true,
      showBaseUrl: false,
      showModel: true,
      modelOptions: ["deepseek-v4-pro", "deepseek-v4-flash"],
    };
  }

  if (provider.id === "glm" || provider.id === "qwen" || provider.id === "kimi") {
    return {
      showApiKey: true,
      showBaseUrl: true,
      showModel: true,
      modelOptions: null,
    };
  }

  return {
    showApiKey: false,
    showBaseUrl: false,
    showModel: false,
    modelOptions: null,
  };
}
```

```svelte
{#if fieldRules.showApiKey}
  <input bind:value={draft.api_key} placeholder={providerKeyPlaceholder(provider)} />
{/if}
{#if fieldRules.showBaseUrl}
  <input bind:value={draft.base_url} placeholder={provider.defaultBaseUrl ?? "https://..."} />
{/if}
{#if fieldRules.showModel}
  {#if fieldRules.modelOptions}
    <select bind:value={draft.model}>
      {#each fieldRules.modelOptions as model}
        <option value={model}>{model}</option>
      {/each}
    </select>
  {:else}
    <input bind:value={draft.model} placeholder={provider.defaultModel ?? "model-id"} />
  {/if}
{/if}
```

- [ ] **Step 4: Update localized copy for new provider cards and validation text**

```json
{
  "settings_provider_qwen": "QWEN",
  "settings_provider_kimi": "KIMI",
  "settings_provider_requires_api_key": "需要 API Key",
  "settings_provider_requires_base_url": "需要 Base URL",
  "settings_provider_requires_model": "需要模型"
}
```

- [ ] **Step 5: Run frontend validation and verify it passes**

Run: `npm run check`
Expected: PASS with QWEN/KIMI provider cards rendered from the shared settings logic.

- [ ] **Step 6: Commit**

```bash
git add src/routes/settings/+page.svelte messages/en.json messages/zh-CN.json
git commit -m "feat: add provider cards for qwen and kimi"
```

### Task 4: Extend chat, room, and provider-label surfaces

**Files:**
- Modify: `src/routes/chat/+page.svelte`
- Modify: `src/routes/rooms/+page.svelte`
- Modify: `src/lib/components/AgentSelector.svelte`
- Modify: `src/lib/stores/room-store.svelte.ts`
- Modify: `src/lib/utils/room-ui.ts`
- Modify: `src/lib/utils/room-ui.test.ts`
- Modify: `src/lib/utils/continuable-run.test.ts`
- Modify: `src/lib/stores/room-store.test.ts`
- Test: `src/lib/utils/room-ui.test.ts`
- Test: `src/lib/utils/continuable-run.test.ts`
- Test: `src/lib/stores/room-store.test.ts`

- [ ] **Step 1: Write the failing label and continuity tests**

```ts
it("renders QWEN and KIMI labels for Claude-backed room participants", () => {
  expect(roomParticipantProviderLabel("claude", "bailian")).toBe("QWEN");
  expect(roomParticipantProviderLabel("claude", "kimi")).toBe("KIMI");
});

it("restores the latest continuable run for QWEN and KIMI providers", () => {
  const runs = [
    run("qwen-run", "claude", {
      status: "running",
      platform_id: "bailian",
    }),
    run("kimi-run", "claude", {
      status: "running",
      platform_id: "kimi",
    }),
  ];

  expect(findLastContinuableRun(runs, "qwen")?.id).toBe("qwen-run");
  expect(findLastContinuableRun(runs, "kimi")?.id).toBe("kimi-run");
});
```

- [ ] **Step 2: Run the focused frontend tests and verify they fail**

Run: `npm test -- src/lib/utils/room-ui.test.ts src/lib/utils/continuable-run.test.ts src/lib/stores/room-store.test.ts`
Expected: FAIL because QWEN/KIMI are not yet treated as first-class provider ids.

- [ ] **Step 3: Update provider label and selection logic**

```ts
export function roomParticipantProviderLabel(agent: string, platformId?: string | null): string {
  const provider = getPhase7Provider(providerIdForRun(agent, platformId));
  return provider.label;
}
```

```ts
const API_PROVIDER_IDS = new Set(["deepseek", "glm", "qwen", "kimi"]);
```

```ts
if (provider.id === "qwen" || provider.id === "kimi") {
  platformId = provider.platformId;
  executionAgent = "claude";
}
```

- [ ] **Step 4: Ensure chat and room surfaces read the correct saved credential**

```ts
const cred = provider.platformId
  ? findCredential(settings?.platform_credentials ?? [], provider.platformId)
  : null;
```

```ts
const providerReady = provider.requiredConfig.every((field) => {
  if (field === "api_key") return Boolean(cred?.api_key?.trim());
  if (field === "base_url") return Boolean(cred?.base_url?.trim());
  if (field === "model") return Boolean(cred?.model?.trim());
  return true;
});
```

- [ ] **Step 5: Run the focused tests and verify they pass**

Run: `npm test -- src/lib/utils/room-ui.test.ts src/lib/utils/continuable-run.test.ts src/lib/stores/room-store.test.ts`
Expected: PASS with QWEN/KIMI recognized in room labels, continuable-run lookup, and room participant creation.

- [ ] **Step 6: Commit**

```bash
git add src/routes/chat/+page.svelte src/routes/rooms/+page.svelte src/lib/components/AgentSelector.svelte src/lib/stores/room-store.svelte.ts src/lib/utils/room-ui.ts src/lib/utils/room-ui.test.ts src/lib/utils/continuable-run.test.ts src/lib/stores/room-store.test.ts
git commit -m "feat: surface qwen and kimi in chat and rooms"
```

### Task 5: Verify resume, inherited provider config, and final integration

**Files:**
- Modify: `src-tauri/src/storage/runs.rs`
- Modify: `src-tauri/src/commands/onboarding.rs`
- Modify: `src-tauri/src/commands/session.rs`
- Test: `src-tauri/src/commands/session.rs`
- Test: `src-tauri/src/storage/settings.rs`

- [ ] **Step 1: Add failing resume/inheritance tests for parameterized providers**

```rust
#[test]
fn effective_platform_for_auth_mode_preserves_parameterized_provider_platforms() {
    assert_eq!(effective_platform_for_auth_mode("cli", Some("bailian")), Some("bailian"));
    assert_eq!(effective_platform_for_auth_mode("cli", Some("kimi")), Some("kimi"));
}

#[test]
fn build_provider_launch_config_errors_when_parameterized_provider_is_incomplete() {
    let settings = make_settings_with_cred("kimi", Some("sk-kimi"), None, Some("kimi-k2.5"));

    let result = build_provider_launch_config(&settings, "kimi");

    assert_eq!(result.unwrap_err(), "kimi base URL is not configured");
}
```

- [ ] **Step 2: Run the focused Rust tests and verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::session::tests:: -- --nocapture`
Expected: FAIL because `effective_platform_for_auth_mode` still only preserves DeepSeek/GLM-style platform ids.

- [ ] **Step 3: Extend provider-platform preservation for resume and inherited flows**

```rust
fn is_phase7_claude_compatible_api_platform(platform_id: &str) -> bool {
    matches!(platform_id, "deepseek" | "zhipu" | "zhipu-intl" | "bailian" | "kimi")
}
```

```rust
let resolved_pid = platform_id
    .clone()
    .or_else(|| connection_profile.as_ref().and_then(|p| p.platform_id.clone()))
    .or_else(|| settings.active_platform_id.clone());
```

- [ ] **Step 4: Update any provider label/status helpers that still special-case only the old provider set**

```rust
match platform_id {
    "deepseek" => "DeepSeek",
    "zhipu" | "zhipu-intl" => "GLM",
    "bailian" => "QWEN",
    "kimi" => "KIMI",
    _ => platform_id,
}
```

- [ ] **Step 5: Run targeted and broad verification**

Run: `npm run check && npm test -- src/lib/utils/provider-catalog.test.ts src/lib/utils/room-ui.test.ts src/lib/utils/continuable-run.test.ts src/lib/stores/room-store.test.ts && cargo test --manifest-path src-tauri/Cargo.toml commands::session::tests:: -- --nocapture && cargo test --manifest-path src-tauri/Cargo.toml storage::settings::tests:: -- --nocapture`
Expected: PASS, or on this Windows machine a known non-code Rust runtime blocker after compilation; if the runtime blocker appears before test execution, record it explicitly and keep the compiled assertions as the code-level confidence signal.

- [ ] **Step 6: Manual UI verification**

Run: `npm run tauri dev`
Expected: The desktop app starts, the settings page shows provider cards for DeepSeek/GLM/QWEN/KIMI, chat provider selection shows all seven providers, and room participant selection can create QWEN/KIMI seats with the expected labels.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/storage/runs.rs src-tauri/src/commands/onboarding.rs src-tauri/src/commands/session.rs
git commit -m "feat: preserve provider launch config across entry paths"
```

---

## Self-Review

- **Spec coverage:**
  - Provider entry expansion is covered by Tasks 1 and 4.
  - Settings-page provider-card redesign is covered by Task 3.
  - DeepSeek fixed-template config is covered by Task 2.
  - GLM/QWEN/KIMI parameterized-template config is covered by Task 2.
  - DeepSeek model migration is covered by Task 1.
  - Entry-path consistency across chat, room, resume, continue, and inherited flows is covered by Tasks 2, 4, and 5.
- **Placeholder scan:** No `TODO`, `TBD`, or “similar to task N” placeholders remain. Each code-changing step includes concrete snippets or commands.
- **Type consistency:** Provider ids are consistently `deepseek`, `glm`, `qwen`, `kimi`; platform ids are consistently `deepseek`, `zhipu|zhipu-intl`, `bailian`, `kimi`.

---

Plan complete and saved to `docs/superpowers/plans/2026-05-06-provider-native-connection-entry.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
