<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { getTransport } from "$lib/transport/index";
  import type { CliSessionSummary, DiscoverResult, ImportResult } from "$lib/types";
  import { t } from "$lib/i18n/index.svelte";
  import { fmtRelative } from "$lib/i18n/format";
  import { cwdDisplayLabel } from "$lib/utils/format";
  import { dbg, dbgWarn } from "$lib/utils/debug";
  import { startSession } from "$lib/api";

  // State
  let sessions = $state<CliSessionSummary[]>([]);
  let loading = $state(true);
  let error = $state("");
  let searchInput = $state("");
  let selectedProject = $state<string>("");
  let importingSessionId = $state<string | null>(null);
  let truncated = $state(false);

  // Helper to invoke Tauri commands
  function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    return getTransport().invoke<T>(cmd, args);
  }

  // Derived: unique projects from sessions
  let projects = $derived.by(() => {
    const projectSet = new Set(sessions.map((s) => s.cwd));
    return Array.from(projectSet).sort();
  });

  // Derived: filtered sessions
  let filteredSessions = $derived.by(() => {
    let result = sessions;

    // Filter by project
    if (selectedProject) {
      result = result.filter((s) => s.cwd === selectedProject);
    }

    // Filter by search text
    if (searchInput) {
      const query = searchInput.toLowerCase();
      result = result.filter(
        (s) =>
          s.firstPrompt.toLowerCase().includes(query) ||
          s.cwd.toLowerCase().includes(query) ||
          (s.model && s.model.toLowerCase().includes(query))
      );
    }

    return result;
  });

  // Truncate text
  function truncate(text: string, maxLen: number): string {
    if (text.length <= maxLen) return text;
    return text.slice(0, maxLen) + "...";
  }

  // Load sessions from CC native history
  async function loadSessions() {
    loading = true;
    error = "";

    try {
      dbg("history", "loadSessions: discovering CLI sessions");
      const result = await invoke<DiscoverResult>("discover_cli_sessions", { cwd: "/" });
      // Filter out subagent sessions
      sessions = result.sessions.filter((s) => !s.hasSubagents);
      truncated = result.truncated;
      dbg("history", "loadSessions: found", sessions.length, "sessions, truncated:", truncated);
    } catch (e) {
      error = t("history_cc_loadError");
      dbgWarn("history", "loadSessions error", e);
    } finally {
      loading = false;
    }
  }

  // Continue a session: import then resume (or just resume if already imported)
  async function continueSession(session: CliSessionSummary) {
    importingSessionId = session.sessionId;

    try {
      let runId: string;

      if (session.alreadyImported && session.existingRunId) {
        // Already imported - use existing run ID
        runId = session.existingRunId;
        dbg("history", "continueSession: already imported, using runId", runId);
      } else {
        // Not imported yet - import first
        dbg("history", "continueSession: importing session", session.sessionId);
        const importResult = await invoke<ImportResult>("import_cli_session", {
          sessionId: session.sessionId,
          cwd: session.cwd,
        });
        runId = importResult.runId;
      }

      // Resume the session via startSession
      await startSession(runId, "resume", session.sessionId);

      // Navigate to chat page
      goto(`/chat?run=${runId}`);
    } catch (e) {
      error = t("history_cc_continueError");
      dbgWarn("history", "continueSession error", e);
    } finally {
      importingSessionId = null;
    }
  }

  onMount(() => {
    loadSessions();
  });
</script>

<div class="flex h-full flex-col overflow-hidden">
  <!-- Header -->
  <div class="shrink-0 border-b border-border px-6 py-4">
    <h1 class="text-xl font-semibold text-foreground">{t("history_title")}</h1>
    <p class="mt-1 text-sm text-muted-foreground">{t("history_subtitle")}</p>
  </div>

  <div class="flex-1 overflow-y-auto px-6 py-4">
    <!-- Search + Stats -->
    <div class="mb-4 flex items-center gap-3">
      <div class="relative flex-1">
        <svg
          class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" />
        </svg>
        <input
          type="text"
          bind:value={searchInput}
          placeholder={t("history_searchPlaceholder")}
          class="w-full rounded-lg border border-border bg-background py-2 pl-10 pr-4 text-sm text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
        />
      </div>
      <div class="text-sm text-muted-foreground">
        {filteredSessions.length} / {sessions.length} {t("history_cc_sessions")}
      </div>
    </div>

    <!-- Truncation notice -->
    {#if truncated}
      <div class="mb-4 rounded-lg border border-yellow-500/20 bg-yellow-500/10 p-3 text-sm text-yellow-400">
        {t("history_cc_truncated")}
      </div>
    {/if}

    <!-- Project filter pills -->
    {#if projects.length > 1}
      <div class="mb-4 flex flex-wrap gap-2">
        <button
          onclick={() => (selectedProject = "")}
          class="rounded-full px-3 py-1 text-xs font-medium transition-colors {!selectedProject
            ? 'bg-primary text-primary-foreground'
            : 'bg-muted text-muted-foreground hover:bg-muted/80'}"
        >
          {t("history_allProjects")}
        </button>
        {#each projects as project}
          <button
            onclick={() => (selectedProject = project)}
            class="rounded-full px-3 py-1 text-xs font-medium transition-colors {selectedProject ===
            project
              ? 'bg-primary text-primary-foreground'
              : 'bg-muted text-muted-foreground hover:bg-muted/80'}"
          >
            {cwdDisplayLabel(project)}
          </button>
        {/each}
      </div>
    {/if}

    <!-- Loading state -->
    {#if loading}
      <div class="flex items-center justify-center py-20">
        <div
          class="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent"
        ></div>
      </div>
    {:else if error}
      <div class="rounded-lg border border-red-500/20 bg-red-500/10 p-4 text-sm text-red-400">
        {error}
      </div>
    {:else if filteredSessions.length === 0}
      <div class="flex flex-col items-center justify-center py-20 text-muted-foreground">
        <svg
          class="mb-3 h-12 w-12 opacity-30"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
        >
          <circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" />
        </svg>
        <p class="text-sm">{t("history_noResults")}</p>
      </div>
    {:else}
      <!-- Session cards -->
      <div class="space-y-2">
        {#each filteredSessions as session (session.sessionId)}
          <div
            class="rounded-lg border border-border bg-card p-4 transition-colors hover:bg-muted/30"
          >
            <div class="flex items-start justify-between gap-3">
              <!-- Left side: prompt + metadata -->
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  {#if session.alreadyImported}
                    <span class="h-2 w-2 shrink-0 rounded-full bg-green-500" title={t("history_cc_alreadyImported")}></span>
                  {:else}
                    <span class="h-2 w-2 shrink-0 rounded-full bg-blue-500"></span>
                  {/if}
                  <p class="text-sm font-medium text-foreground">
                    {truncate(session.firstPrompt || t("history_cc_emptyPrompt"), 80)}
                  </p>
                </div>
                <div class="mt-1.5 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                  <span>{cwdDisplayLabel(session.cwd)}</span>
                  <span>·</span>
                  <span>{fmtRelative(session.lastActivityAt)}</span>
                  {#if session.model}
                    <span>·</span>
                    <span
                      class="rounded bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary"
                    >
                      {session.model}
                    </span>
                  {/if}
                  <span>·</span>
                  <span>{t("history_cc_messageCount", { count: String(session.messageCount) })}</span>
                </div>
              </div>

              <!-- Right side: continue button -->
              <div class="shrink-0">
                <button
                  onclick={() => continueSession(session)}
                  disabled={importingSessionId === session.sessionId}
                  class="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50"
                >
                  {#if importingSessionId === session.sessionId}
                    <span class="flex items-center gap-1.5">
                      <span
                        class="inline-block h-3 w-3 animate-spin rounded-full border-2 border-primary-foreground border-t-transparent"
                      ></span>
                      {t("history_cc_importing")}
                    </span>
                  {:else}
                    {session.alreadyImported ? t("history_cc_open") : t("history_cc_continue")}
                  {/if}
                </button>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
