# Provider-Native Connection Entry and Settings Design

**Date:** 2026-05-06  
**Status:** Partially implemented  
**Scope:** DeepSeek, GLM, QWEN, KIMI provider entry design, settings design, and native launch-config generation

**Implementation status (2026-05-06):**
- Completed: provider catalog expansion for QWEN and KIMI, DeepSeek model narrowing to `deepseek-v4-pro` / `deepseek-v4-flash`, settings-page provider-card field rules, room/chat-facing provider identity coverage, and provider-native launch-config builder wiring in `src-tauri/src/commands/session.rs`.
- Completed verification: targeted frontend tests for provider catalog, room UI labels, continuable-run lookup, and room-store participant creation all pass.
- Environment blocker: targeted Rust test binaries compile successfully but still fail to start on this Windows machine with the pre-existing runtime error `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)`, so full Rust green verification remains blocked by environment rather than current source-level compile errors.

---

## Goal

Unify DeepSeek, GLM, QWEN, and KIMI as first-class providers in the product surface while keeping the existing execution model intact: the UI presents provider identity, but the execution path still reuses the Claude-native launch/session machinery.

The main change is not introducing four new low-level execution agents. The main change is introducing provider-native launch config generation so these providers no longer depend on scattered `base_url` / `model` / `extra_env` defaults alone.

## What This Design Changes

1. DeepSeek, GLM, QWEN, and KIMI all become explicit provider entries in the user-facing provider surfaces.
2. Settings are redesigned around provider cards instead of generic connection-profile editing.
3. Launch behavior for these providers is generated from provider-native config builders.
4. DeepSeek uses a fixed full launch-config template with only API key injected from settings.
5. GLM, QWEN, and KIMI use a shared parameterized launch-config template where API key, base URL, and model come from settings.
6. Existing provider/execution identity separation remains unchanged.

## What This Design Does Not Change

1. Claude remains the underlying execution agent for these provider flows.
2. Claude, Codex, and Gemini official CLI provider behavior is not redesigned here.
3. This design does not add a free-form JSON editor in settings.
4. This design does not introduce new persisted execution-agent types for DeepSeek, GLM, QWEN, or KIMI.

---

## Provider Model

### Shared product model

Each of these providers is a first-class UI/provider identity:

- DeepSeek
- GLM
- QWEN
- KIMI

Each still executes through the Claude-native session/startup path under the hood.

The important architectural rule stays the same:

- **Provider identity** controls what the user selected, how the UI labels the run, what settings are required, and how startup config is generated.
- **Execution identity** controls which backend launcher/session machinery actually runs the work.

This means the current separation between displayed provider and execution agent is preserved, not removed.

---

## Provider Types

This design introduces two provider configuration modes.

### Type A: Fixed-template provider

#### DeepSeek

DeepSeek uses a full native launch-config template with fixed non-secret values.

Dynamic setting input:

- `api_key`

Fixed behavior:

- The full startup config is generated from a code-defined DeepSeek template.
- `ANTHROPIC_AUTH_TOKEN` is filled from the configured DeepSeek API key.
- The rest of the template remains provider-defined.

DeepSeek model policy is narrowed to:

- `deepseek-v4-pro`
- `deepseek-v4-flash`

`deepseek-chat` is removed from all visible and runtime-facing defaults. Existing persisted values using `deepseek-chat` are migrated to `deepseek-v4-pro`.

### Type B: Parameterized-template providers

#### GLM
#### QWEN
#### KIMI

These three providers share the same high-level configuration pattern.

Dynamic setting inputs:

- `api_key`
- `base_url`
- `model`

Fixed behavior:

- Startup config is still generated as a full structured launch config, not as loose env fragments.
- The config shell is code-defined.
- The provider-specific settings inject `base_url`, `auth_token`, and `model` into that shell.

For this batch, GLM is the reference design for parameterized providers. QWEN and KIMI follow the same settings and entry structure. Their default values come from existing repo presets where available. If a preset is incomplete, the provider still follows the GLM-style field structure and validation requirements.

---

## Launch Config Design

### DeepSeek launch config

DeepSeek startup must use the full provider-native config template supplied by the product owner.

The template includes:

- `env`
- `includeCoAuthoredBy`
- `thinking`
- `hooks`
- `skipDangerousModePermissionPrompt`
- `permissions`
- `enabledPlugins`
- `autoUpdatesChannel`
- `language`

Runtime injection rule:

- `env.ANTHROPIC_AUTH_TOKEN` comes from the saved DeepSeek API key.

DeepSeek base URL and model defaults are not user-editable in normal settings flow beyond the approved model choices. This keeps DeepSeek as a strongly constrained provider path.

### GLM / QWEN / KIMI launch config

GLM, QWEN, and KIMI use a shared structured launch-config shell.

The shell includes at least:

- `env`
- `permissions`
- `enabledPlugins`
- `autoUpdatesChannel`
- `language`

Runtime injection rules:

- `env.ANTHROPIC_BASE_URL` comes from settings
- `env.ANTHROPIC_AUTH_TOKEN` comes from settings
- `env.ANTHROPIC_MODEL` comes from settings
- model alias env such as `ANTHROPIC_DEFAULT_HAIKU_MODEL`, `ANTHROPIC_DEFAULT_SONNET_MODEL`, and `ANTHROPIC_DEFAULT_OPUS_MODEL` follow the configured model

This keeps the provider behavior stable while still allowing user-controlled endpoint and model selection.

---

## Scope of Application

This behavior applies everywhere the effective provider/platform is one of the above providers.

For DeepSeek specifically, the owner requirement is broad and strict:

- as long as the effective provider path resolves to DeepSeek, the DeepSeek native launch config must be used

That same consistency rule is applied to GLM, QWEN, and KIMI.

This includes:

- new chat starts
- room participant starts
- resume / continue flows
- side-question or derivative Claude session flows if they preserve provider identity
- subagent launches that inherit the parent provider context

The design goal is to avoid entry-specific drift where one entry path uses the provider-native config and another silently falls back to generic provider defaults.

---

## Settings Page Design

### Surface model

The settings page should move from a generic connection-profile mindset to explicit provider cards.

Each of the following appears as its own provider card:

- DeepSeek
- GLM
- QWEN
- KIMI

The page should not expose a raw JSON editor.

### Card fields

#### DeepSeek card

Visible fields:

- API key
- model selector

Model selector options:

- `deepseek-v4-pro`
- `deepseek-v4-flash`

Base URL handling:

- hidden, or shown as read-only fixed provider info

#### GLM / QWEN / KIMI cards

Visible fields:

- API key
- base URL
- model

Default values:

- initialize from existing repo presets when available
- if a preset lacks one of these values, keep the same field structure and require completion before use

### Interaction rules

1. Each provider card saves independently.
2. Missing required fields are shown inline in the card.
3. The settings page remains operational and compact, not a configuration-file editor.
4. Provider cards describe the connection requirements, but the backend remains responsible for generating the final launch config.

---

## Provider Entry Design

These providers should be selectable as first-class product providers, not hidden as preset-only transport details.

The selection surfaces that should reflect this are:

- chat provider picker
- room participant provider picker
- any resume/continue UI that reflects run provider identity

The user selects a provider identity. The system then resolves that provider to the correct launch config and execution path.

This preserves the existing architecture and makes the user-facing mental model cleaner.

---

## Backend Design

### New builder boundary

Introduce a provider-native launch-config builder boundary.

Expected builder shape:

- `build_deepseek_launch_config(...)`
- `build_parameterized_provider_launch_config(provider, settings)`

Where appropriate, GLM / QWEN / KIMI should reuse the same parameterized builder instead of duplicating nearly identical code.

### Resolution flow

At startup time:

1. resolve the effective provider/platform identity
2. if it is DeepSeek, build the fixed DeepSeek launch config
3. if it is GLM, QWEN, or KIMI, build the parameterized launch config from settings
4. otherwise continue using the existing generic path

### Priority model

The effective priority should be:

1. provider-native launch-config rules
2. saved provider settings for that provider
3. generic provider defaults/presets
4. fallback app defaults

This prevents generic defaults from silently overriding explicit provider-native behavior.

---

## Migration Rules

### DeepSeek

- migrate `deepseek-chat` to `deepseek-v4-pro`
- remove `deepseek-chat` from provider defaults, visible model lists, and any runtime fallback lists

### GLM / QWEN / KIMI

- preserve existing saved settings where possible
- if saved settings are missing required `base_url` or `model`, fill from known defaults when possible
- if not safely fillable, surface the provider as incomplete and require user correction

The design should favor correctness over silent fallback to unrelated providers.

---

## Validation Requirements

The implementation is only complete if it is verified across the main entry paths.

Required checks:

1. DeepSeek settings show only the approved model options.
2. DeepSeek launch config is used for DeepSeek starts across all supported entry paths.
3. GLM launch config is built from settings-derived API key, base URL, and model.
4. QWEN launch config is built from settings-derived API key, base URL, and model.
5. KIMI launch config is built from settings-derived API key, base URL, and model.
6. resume / continue does not drop provider-specific config semantics.
7. room participant starts do not drop provider-specific config semantics.
8. subagent behavior preserves the provider-native model/config intent where inherited.
9. Claude, Codex, Gemini, and unrelated provider presets are not regressed.

Frontend verification is required because provider cards and provider pickers are user-visible behavior.

---

## Recommended Implementation Order

1. add the provider-native launch-config builder boundary in Rust
2. implement DeepSeek fixed-template config generation
3. implement GLM/QWEN/KIMI parameterized config generation
4. update provider catalog and settings card metadata
5. migrate DeepSeek model defaults and persisted `deepseek-chat` values
6. verify chat, room, resume, continue, and inherited/subagent paths

---

## Risks and Mitigations

### Risk: generic provider-default logic still leaks into these providers

Mitigation:

- introduce a clear startup builder boundary with provider-first precedence

### Risk: one entry path keeps using the old generic path

Mitigation:

- verify all entry points that resolve provider/platform identity, especially resume, room starts, and inherited runs

### Risk: settings UI becomes too configurable and drifts from the intended product model

Mitigation:

- use provider cards with required fields only
- do not expose raw JSON editing in this batch

---

## Final Recommendation

Proceed with a two-track provider strategy:

- DeepSeek as a fixed-template provider with constrained model choices
- GLM / QWEN / KIMI as parameterized-template providers sharing one settings shape and one launch-config generation pattern

This meets the owner requirement without overhauling the execution architecture, and it provides a clean extension path for future Claude-compatible providers.
