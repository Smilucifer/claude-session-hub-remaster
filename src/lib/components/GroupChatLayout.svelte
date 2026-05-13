<script lang="ts">
  import * as api from "$lib/api";
  import type {
    GroupChatDetail,
    GroupChatParticipantDetail,
    AiCharacter,
    RunStatus,
  } from "$lib/types";
  import { getPhase7Provider, providerIdForRun } from "$lib/utils/provider-catalog";
  import { t } from "$lib/i18n/index.svelte";
  import { dbg, dbgWarn } from "$lib/utils/debug";

  let { groupChat }: { groupChat: api.GroupChatRunIndexEntry | null } = $props();

  // ── State ──
  let detail = $state<GroupChatDetail | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let panelOpen = $state(true);

  // ── Character picker ──
  let characters = $state<AiCharacter[]>([]);
  let pickerOpen = $state(false);
  let pickerLoading = $state(false);
  let quickLabel = $state("");
  let creating = $state(false);

  // ── Composer ──
  let composerText = $state("");
  let mentionOpen = $state(false);
  let mentionQuery = $state("");
  let mentionIndex = $state(0);

  let textareaEl: HTMLTextAreaElement | undefined = $state();

  // ── Derived ──
  let participants = $derived(detail?.participants ?? []);

  let filteredMentionParticipants = $derived.by(() => {
    if (!mentionOpen) return [];
    const q = mentionQuery.toLowerCase();
    return participants.filter(
      (p) =>
        p.participant.label.toLowerCase().includes(q) ||
        p.participant.agent.toLowerCase().includes(q),
    );
  });

  // ── Load detail ──
  async function loadDetail() {
    if (!groupChat) return;
    loading = true;
    error = null;
    try {
      detail = await api.getGroupChat(groupChat.room_id);
    } catch (e) {
      dbgWarn("GroupChatLayout", "loadDetail failed", e);
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  // Load on mount and when room changes
  $effect(() => {
    const roomId = groupChat?.room_id;
    if (roomId) loadDetail();
  });

  // ── Character picker ──
  async function openPicker() {
    pickerOpen = true;
    if (characters.length === 0) {
      pickerLoading = true;
      try {
        characters = await api.listCharacters();
      } catch (e) {
        dbgWarn("GroupChatLayout", "listCharacters failed", e);
        error = "Failed to load characters";
      } finally {
        pickerLoading = false;
      }
    }
  }

  function closePicker() {
    pickerOpen = false;
    quickLabel = "";
  }

  async function addCharacterParticipant(char: AiCharacter) {
    if (!detail || creating) return;
    creating = true;
    try {
      const cwd = detail.cwd ?? "/";
      const provider = getPhase7Provider(char.default_provider);
      const updated = await api.createGroupChatClaudeParticipant(
        detail.id,
        char.role_instruction ?? `You are ${char.label}.`,
        cwd,
        char.default_model ?? provider.defaultModel,
        provider.platformId,
        undefined, // connectionProfileId
        char.label,
        char.role_type,
      );
      detail = updated;
      closePicker();
    } catch (e) {
      dbgWarn("GroupChatLayout", "addCharacterParticipant failed", e);
      error = "Failed to add participant";
    } finally {
      creating = false;
    }
  }

  async function addQuickParticipant() {
    if (!detail || creating || !quickLabel.trim()) return;
    creating = true;
    try {
      const cwd = detail.cwd ?? "/";
      const updated = await api.createGroupChatClaudeParticipant(
        detail.id,
        `You are ${quickLabel.trim()}.`,
        cwd,
        undefined,
        undefined,
        undefined,
        quickLabel.trim(),
        "custom",
      );
      detail = updated;
      closePicker();
    } catch (e) {
      dbgWarn("GroupChatLayout", "addQuickParticipant failed", e);
      error = "Failed to add participant";
    } finally {
      creating = false;
    }
  }

  // ── Composer: @mention ──
  function handleComposerInput() {
    const pos = textareaEl?.selectionStart ?? composerText.length;
    // Scan backwards for @
    let atPos = -1;
    for (let i = pos - 1; i >= 0; i--) {
      const ch = composerText[i];
      if (ch === "@") {
        if (i === 0 || /\s/.test(composerText[i - 1])) {
          atPos = i;
        }
        break;
      }
      if (/\s/.test(ch)) break;
    }
    if (atPos >= 0) {
      mentionOpen = true;
      mentionQuery = composerText.slice(atPos + 1, pos);
      mentionIndex = 0;
    } else {
      mentionOpen = false;
    }
  }

  function selectMention(p: GroupChatParticipantDetail) {
    if (!textareaEl) return;
    const pos = textareaEl.selectionStart ?? composerText.length;
    // Find the @ position
    let atPos = -1;
    for (let i = pos - 1; i >= 0; i--) {
      if (composerText[i] === "@") {
        if (i === 0 || /\s/.test(composerText[i - 1])) {
          atPos = i;
        }
        break;
      }
      if (/\s/.test(composerText[i])) break;
    }
    if (atPos < 0) return;
    const prefix = composerText.slice(0, atPos + 1);
    const suffix = composerText.slice(pos);
    composerText = prefix + p.participant.label + " " + suffix;
    mentionOpen = false;
    requestAnimationFrame(() => {
      if (textareaEl) {
        const newPos = atPos + 1 + p.participant.label.length + 1;
        textareaEl.selectionStart = textareaEl.selectionEnd = newPos;
        textareaEl.focus();
      }
    });
  }

  function handleComposerKeydown(e: KeyboardEvent) {
    if (mentionOpen && filteredMentionParticipants.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        mentionIndex = Math.min(
          mentionIndex + 1,
          filteredMentionParticipants.length - 1,
        );
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        mentionIndex = Math.max(mentionIndex - 1, 0);
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        selectMention(filteredMentionParticipants[mentionIndex]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        mentionOpen = false;
        return;
      }
    }
    // Enter without shift → send
    if (e.key === "Enter" && !e.shiftKey && !mentionOpen && composerText.trim()) {
      e.preventDefault();
      handleSend();
    }
  }

  async function handleSend() {
    const text = composerText.trim();
    if (!text || !detail) return;
    dbg("GroupChatLayout", "send", { len: text.length });
    try {
      const updated = await api.sendGroupChatMessage(detail.id, text);
      detail = updated;
      composerText = "";
      if (textareaEl) textareaEl.style.height = "auto";
    } catch (e) {
      dbgWarn("GroupChatLayout", "send failed", e);
    }
  }

  function insertSummaryCommand() {
    if (!textareaEl) return;
    composerText = "@summary " + composerText;
    requestAnimationFrame(() => {
      if (textareaEl) {
        textareaEl.selectionStart = textareaEl.selectionEnd = composerText.length;
        textareaEl.focus();
      }
    });
  }

  function autoResize() {
    if (!textareaEl) return;
    textareaEl.style.height = "auto";
    const max = 4 * 24;
    textareaEl.style.height = Math.min(textareaEl.scrollHeight, max) + "px";
  }

  // ── Status helpers ──
  function statusColor(status: RunStatus | undefined): string {
    switch (status) {
      case "running":
        return "bg-green-500";
      case "completed":
        return "bg-blue-500";
      case "failed":
        return "bg-red-500";
      case "stopped":
        return "bg-yellow-500";
      case "pending":
        return "bg-orange-400";
      default:
        return "bg-muted-foreground/40";
    }
  }

  function statusLabel(status: RunStatus | undefined): string {
    switch (status) {
      case "running":
        return t("groupChat_statusLabelRunning");
      case "completed":
        return t("groupChat_statusLabelCompleted");
      case "failed":
        return t("groupChat_statusLabelFailed");
      case "stopped":
        return t("groupChat_statusLabelStopped");
      case "pending":
        return t("groupChat_statusLabelStarting");
      default:
        return t("groupChat_statusLabelIdle");
    }
  }

  function providerLabel(agent: string, platformId?: string | null): string {
    const id = providerIdForRun(agent, platformId);
    return getPhase7Provider(id).label;
  }

  function roleBadgeColor(role: string): string {
    switch (role) {
      case "planner":
        return "bg-purple-500/15 text-purple-400";
      case "executor":
        return "bg-blue-500/15 text-blue-400";
      default:
        return "bg-muted text-muted-foreground";
    }
  }
</script>

<div class="flex h-full overflow-hidden bg-background">
  <!-- Main content area (placeholder for timeline) -->
  <div class="flex-1 flex flex-col min-w-0">
    <!-- Header bar -->
    <div class="flex items-center justify-between border-b border-border px-4 py-2.5 shrink-0">
      <div class="flex items-center gap-2 min-w-0">
        <h2 class="text-sm font-semibold truncate">{detail?.name ?? groupChat?.room_name ?? t("groupChat_defaultName")}</h2>
        {#if detail}
          <span class="text-[11px] text-muted-foreground shrink-0">
            {t("groupChat_participantsCount", { count: String(participants.length) })}
          </span>
        {/if}
      </div>
      <button
        class="flex h-7 w-7 items-center justify-center rounded-lg text-muted-foreground/60 hover:text-foreground hover:bg-accent transition-colors shrink-0"
        onclick={() => (panelOpen = !panelOpen)}
        title={panelOpen ? t("groupChat_hidePanel") : t("groupChat_showPanel")}
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
          <circle cx="9" cy="7" r="4" />
          <path d="M22 21v-2a4 4 0 0 0-3-3.87" />
          <path d="M16 3.13a4 4 0 0 1 0 7.75" />
        </svg>
      </button>
    </div>

    <!-- Loading / error states -->
    {#if loading}
      <div class="flex-1 flex items-center justify-center">
        <div class="h-5 w-5 rounded-full border-2 border-border border-t-muted-foreground animate-spin"></div>
      </div>
    {:else if error}
      <div class="flex-1 flex items-center justify-center">
        <p class="text-sm text-destructive">{error}</p>
      </div>
    {:else if detail}
      <!-- Placeholder: future timeline goes here -->
      <div class="flex-1 flex items-center justify-center p-8">
        <div class="text-center text-muted-foreground max-w-sm">
          <p class="text-sm">{t("groupChat_noTurns")}</p>
          <p class="text-xs mt-1 text-muted-foreground/60">{t("groupChat_roundtablePlaceholder")}</p>
        </div>
      </div>

      <!-- Composer area -->
      <div class="border-t border-border bg-muted/30 px-4 py-3 shrink-0 relative">
        <!-- Toolbar -->
        <div class="flex items-center gap-1 mb-2">
          <button
            class="shrink-0 rounded-md border border-border/50 px-2 py-0.5 text-[11px] text-muted-foreground/70 hover:text-foreground hover:bg-accent hover:border-border transition-colors"
            onclick={insertSummaryCommand}
            title={t("groupChat_summaryTitle")}
          >
            @{t("groupChat_summary")}
          </button>
          <button
            class="shrink-0 rounded-md border border-border/50 px-2 py-0.5 text-[11px] text-muted-foreground/70 hover:text-foreground hover:bg-accent hover:border-border transition-colors"
            onclick={() => {
              composerText = "@debate " + composerText;
              requestAnimationFrame(() => {
                if (textareaEl) {
                  textareaEl.selectionStart = textareaEl.selectionEnd = composerText.length;
                  textareaEl.focus();
                }
              });
            }}
            title={t("groupChat_debateTitle")}
          >
            @{t("groupChat_debate")}
          </button>
        </div>

        <!-- Textarea -->
        <textarea
          bind:this={textareaEl}
          bind:value={composerText}
          onkeydown={handleComposerKeydown}
          oninput={() => { autoResize(); handleComposerInput(); }}
          placeholder={t("groupChat_roundtablePlaceholder")}
          rows={1}
          class="w-full resize-none bg-transparent px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none border border-border rounded-lg"
          style="min-height: 36px;"
        ></textarea>

        <!-- @mention dropdown -->
        {#if mentionOpen && filteredMentionParticipants.length > 0}
          <div class="absolute bottom-full left-4 right-4 mb-1 z-50 rounded-lg border border-border bg-background shadow-lg max-h-[200px] overflow-y-auto">
            {#each filteredMentionParticipants as p, i (p.participant.id)}
              <button
                class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm transition-colors {i === mentionIndex ? 'bg-accent text-foreground' : 'text-muted-foreground hover:bg-accent/50'}"
                onmouseenter={() => (mentionIndex = i)}
                onclick={() => selectMention(p)}
              >
                <span class="w-1.5 h-1.5 rounded-full {statusColor(p.run?.status)} shrink-0"></span>
                <span class="truncate text-xs font-medium">{p.participant.label}</span>
                <span class="text-[10px] text-muted-foreground/60 shrink-0">{p.participant.agent}</span>
              </button>
            {/each}
          </div>
        {/if}

        <!-- Send button -->
        <div class="flex justify-end mt-2">
          <button
            class="flex h-7 items-center gap-1.5 rounded-lg px-3 text-xs font-medium transition-colors {composerText.trim()
              ? 'bg-primary text-primary-foreground hover:bg-primary/90'
              : 'text-muted-foreground/40'}"
            onclick={handleSend}
            disabled={!composerText.trim()}
          >
            <span>{t("groupChat_send")}</span>
            <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M5 12h14" /><path d="m12 5 7 7-7 7" />
            </svg>
          </button>
        </div>
      </div>
    {/if}
  </div>

  <!-- Collapsible participant side panel -->
  {#if panelOpen}
    <div class="w-64 border-l border-border bg-background flex flex-col shrink-0">
      <div class="flex items-center justify-between px-3 py-2.5 border-b border-border shrink-0">
        <span class="text-xs font-semibold text-muted-foreground">{t("groupChat_participants")}</span>
        <button
          class="flex h-6 items-center gap-1 rounded px-1.5 text-[11px] font-medium text-primary hover:bg-accent transition-colors"
          onclick={openPicker}
        >
          <svg class="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 5v14" /><path d="M5 12h14" />
          </svg>
          {t("groupChat_addParticipant")}
        </button>
      </div>

      <div class="flex-1 overflow-y-auto p-2 space-y-1">
        {#if participants.length === 0}
          <p class="text-[11px] text-muted-foreground/60 text-center py-4">{t("groupChat_noParticipants")}</p>
        {/if}
        {#each participants as p (p.participant.id)}
          {@const status = p.run?.status}
          {@const provider = providerLabel(p.participant.agent, p.run?.platform_id)}
          <div class="rounded-lg border border-border/50 px-3 py-2 hover:bg-accent/30 transition-colors">
            <div class="flex items-center gap-2">
              <span class="w-2 h-2 rounded-full {statusColor(status)} shrink-0" title={statusLabel(status)}></span>
              <span class="text-xs font-medium truncate flex-1">{p.participant.label}</span>
              <span class="text-[10px] rounded px-1 py-0.5 shrink-0 {roleBadgeColor(p.participant.role)}">
                {p.participant.role}
              </span>
            </div>
            <div class="flex items-center gap-1.5 mt-1 ml-4">
              <span class="text-[10px] text-muted-foreground/60 truncate">{provider}</span>
              {#if p.run?.model}
                <span class="text-[10px] text-muted-foreground/40 truncate">/ {p.run.model}</span>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<!-- Character picker overlay -->
{#if pickerOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center" role="dialog" aria-modal="true">
    <div class="fixed inset-0 bg-black/60 backdrop-blur-sm" onclick={closePicker} role="presentation"></div>
    <div class="relative z-50 w-full max-w-md rounded-lg border bg-background p-4 shadow-lg max-h-[80vh] flex flex-col">
      <h3 class="text-sm font-semibold mb-3">{t("groupChat_addParticipant")}</h3>

      <!-- Quick create -->
      <div class="flex items-center gap-2 mb-3">
        <input
          type="text"
          bind:value={quickLabel}
          placeholder={t("groupChat_quickCreatePlaceholder")}
          class="flex-1 h-8 rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-ring"
          onkeydown={(e) => { if (e.key === "Enter" && quickLabel.trim()) addQuickParticipant(); }}
        />
        <button
          class="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50"
          onclick={addQuickParticipant}
          disabled={creating || !quickLabel.trim()}
        >
          {t("groupChat_quickCreate")}
        </button>
      </div>

      <div class="text-[11px] text-muted-foreground/60 mb-2">{t("groupChat_orSelectCharacter")}</div>

      <!-- Character list -->
      <div class="flex-1 overflow-y-auto space-y-1 min-h-0">
        {#if pickerLoading}
          <div class="flex items-center justify-center py-6">
            <div class="h-4 w-4 rounded-full border-2 border-border border-t-muted-foreground animate-spin"></div>
          </div>
        {:else if characters.length === 0}
          <p class="text-[11px] text-muted-foreground/60 text-center py-4">{t("groupChat_noCharacters")}</p>
        {:else}
          {#each characters as char (char.id)}
            <button
              class="flex w-full items-center gap-2.5 rounded-lg border border-border/50 px-3 py-2 text-left hover:bg-accent/50 transition-colors disabled:opacity-50"
              onclick={() => addCharacterParticipant(char)}
              disabled={creating}
            >
              <span class="text-sm shrink-0">{char.icon ?? "?"}</span>
              <div class="flex-1 min-w-0">
                <div class="text-xs font-medium truncate">{char.label}</div>
                <div class="text-[10px] text-muted-foreground/60 truncate">
                  {char.role_type} &middot; {getPhase7Provider(char.default_provider).label}
                  {#if char.default_model}
                    / {char.default_model}
                  {/if}
                </div>
              </div>
              <span class="text-[10px] rounded px-1 py-0.5 shrink-0 {roleBadgeColor(char.role_type)}">
                {char.role_type}
              </span>
            </button>
          {/each}
        {/if}
      </div>

      <div class="flex justify-end mt-3 pt-2 border-t border-border">
        <button
          class="h-8 rounded-md border border-border px-3 text-xs font-medium text-muted-foreground hover:bg-accent transition-colors"
          onclick={closePicker}
        >
          {t("groupChat_confirmNo")}
        </button>
      </div>
    </div>
  </div>
{/if}
