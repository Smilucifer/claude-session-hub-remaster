<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import * as api from "$lib/api";
  import type { UsageOverview, DailyAggregate, BalanceHelperSettings, UserSettings } from "$lib/types";
  import { formatCost, formatTokenCount } from "$lib/utils/format";
  import { dbg, dbgWarn } from "$lib/utils/debug";
  import Card from "$lib/components/Card.svelte";
  import Button from "$lib/components/Button.svelte";
  import Input from "$lib/components/Input.svelte";
  import HeatmapCalendar from "$lib/components/HeatmapCalendar.svelte";
  import StackedModelChart from "$lib/components/StackedModelChart.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { fmtDate, fmtNumber } from "$lib/i18n/format";

  let data = $state<UsageOverview | null>(null);
  let loading = $state(true);
  let error = $state("");
  let selectedDays = $state<number | undefined>(undefined); // undefined = all
  let heatmapDaily = $state<DailyAggregate[] | null>(null);
  let heatmapRequestId = 0;

  /** "app" = OpenCovibe runs only, "global" = all Claude Code sessions */
  let scope = $state<"app" | "global">("global");

  /** Monotonic counter to discard stale responses on rapid tab switching. */
  let requestId = 0;

  /** Whether to show the delayed first-load message (full scan taking > 500ms). */
  let showFullScanMessage = $state(false);

  /** Whether cache clear + rescan is in progress. */
  let refreshing = $state(false);

  // ── Balance state ──
  let balanceHelper = $state<BalanceHelperSettings | null>(null);
  let balanceRefreshing = $state(false);
  let balanceRefreshError = $state<string | null>(null);
  let packySessionInput = $state("");
  let packyTdcItokenInput = $state("");
  let packyUserIdInput = $state("");
  let showPackyCredentials = $state(false);
  let mimoServiceTokenInput = $state("");
  let mimoUserIdInput = $state("");
  let mimoSlhInput = $state("");
  let mimoPhInput = $state("");
  let showMimoCredentials = $state(false);
  let balanceSaving = $state(false);

  function balanceStatusText(source: string): {
    label: string;
    balance: string;
    sub: string;
    dotClass: string;
  } {
    const entry = balanceHelper?.cache?.[source];
    if (!entry)
      return {
        label: t("settings_balance_notChecked"),
        balance: "—",
        sub: "",
        dotClass: "bg-amber-400",
      };
    if (entry.status === "ok") {
      return {
        label: entry.balance_text ?? t("settings_balance_ok"),
        balance: entry.balance_text ?? "—",
        sub: entry.refreshed_at,
        dotClass: "bg-emerald-400",
      };
    }
    return {
      label: t("settings_balance_failed"),
      balance: "—",
      sub: entry.error ?? "",
      dotClass: "bg-red-400",
    };
  }

  async function refreshBalanceStatus(source: "all" | "deepseek" | "packy" | "mimo" = "all") {
    if (balanceRefreshing) return;
    balanceRefreshing = true;
    balanceRefreshError = null;
    try {
      balanceHelper = await api.refreshBalanceStatus(source);
    } catch (e) {
      balanceRefreshError = String(e);
      dbgWarn("usage", "refreshBalanceStatus error", e);
    } finally {
      balanceRefreshing = false;
    }
  }

  async function savePackyCredentials() {
    balanceSaving = true;
    try {
      const next = {
        ...(balanceHelper ?? { auto_refresh_secs: 120, cache: {} }),
        packy_session: packySessionInput.trim() || null,
        packy_tdc_itoken: packyTdcItokenInput.trim() || null,
        packy_user_id: packyUserIdInput.trim() || null,
      };
      const settings = await api.updateUserSettings({
        balance_helper: next,
      } as Partial<UserSettings>);
      balanceHelper = settings.balance_helper ?? null;
      packySessionInput = balanceHelper?.packy_session ?? "";
      packyTdcItokenInput = balanceHelper?.packy_tdc_itoken ?? "";
      packyUserIdInput = balanceHelper?.packy_user_id ?? "";
      void refreshBalanceStatus("packy");
    } catch (e) {
      dbgWarn("usage", "savePackyCredentials error", e);
    } finally {
      balanceSaving = false;
    }
  }

  async function clearPackyCredentials() {
    balanceSaving = true;
    try {
      const next = {
        ...(balanceHelper ?? { auto_refresh_secs: 120, cache: {} }),
        packy_session: null,
        packy_tdc_itoken: null,
        packy_user_id: null,
      };
      const settings = await api.updateUserSettings({
        balance_helper: next,
      } as Partial<UserSettings>);
      balanceHelper = settings.balance_helper ?? null;
      packySessionInput = "";
      packyTdcItokenInput = "";
      packyUserIdInput = "";
      balanceRefreshError = null;
    } catch (e) {
      dbgWarn("usage", "clearPackyCredentials error", e);
    } finally {
      balanceSaving = false;
    }
  }

  async function saveMimoCredentials() {
    balanceSaving = true;
    try {
      const next = {
        ...(balanceHelper ?? { auto_refresh_secs: 120, cache: {} }),
        mimo_service_token: mimoServiceTokenInput.trim() || null,
        mimo_user_id: mimoUserIdInput.trim() || null,
        mimo_slh: mimoSlhInput.trim() || null,
        mimo_ph: mimoPhInput.trim() || null,
      };
      const settings = await api.updateUserSettings({
        balance_helper: next,
      } as Partial<UserSettings>);
      balanceHelper = settings.balance_helper ?? null;
      mimoServiceTokenInput = balanceHelper?.mimo_service_token ?? "";
      mimoUserIdInput = balanceHelper?.mimo_user_id ?? "";
      mimoSlhInput = balanceHelper?.mimo_slh ?? "";
      mimoPhInput = balanceHelper?.mimo_ph ?? "";
      void refreshBalanceStatus("mimo");
    } catch (e) {
      dbgWarn("usage", "saveMimoCredentials error", e);
    } finally {
      balanceSaving = false;
    }
  }

  async function clearMimoCredentials() {
    balanceSaving = true;
    try {
      const next = {
        ...(balanceHelper ?? { auto_refresh_secs: 120, cache: {} }),
        mimo_service_token: null,
        mimo_user_id: null,
        mimo_slh: null,
        mimo_ph: null,
      };
      const settings = await api.updateUserSettings({
        balance_helper: next,
      } as Partial<UserSettings>);
      balanceHelper = settings.balance_helper ?? null;
      mimoServiceTokenInput = "";
      mimoUserIdInput = "";
      mimoSlhInput = "";
      mimoPhInput = "";
      balanceRefreshError = null;
    } catch (e) {
      dbgWarn("usage", "clearMimoCredentials error", e);
    } finally {
      balanceSaving = false;
    }
  }

  const DATE_RANGES = [
    { label: "1d", days: 1 },
    { label: "7d", days: 7 },
    { label: "30d", days: 30 },
    { label: "90d", days: 90 },
    { label: "All", days: undefined as number | undefined },
  ];

  // Default chart mode: "messages" for global, "cost" for app
  let chartMode = $state<"cost" | "tokens" | "messages" | "sessions">("messages");

  let maxDailyValue = $derived.by(() => {
    if (!data?.daily.length) return 1;
    if (chartMode === "cost") {
      return Math.max(...data.daily.map((d) => d.costUsd), 0.01);
    }
    if (chartMode === "messages") {
      return Math.max(...data.daily.map((d) => d.messageCount ?? 0), 1);
    }
    if (chartMode === "sessions") {
      return Math.max(...data.daily.map((d) => d.sessionCount ?? 0), 1);
    }
    return Math.max(...data.daily.map((d) => d.inputTokens + d.outputTokens), 1);
  });

  // Sort state for run history
  let sortCol = $state<"date" | "cost" | "tokens" | "turns">("date");
  let sortAsc = $state(false);

  let sortedRuns = $derived.by(() => {
    if (!data?.runs) return [];
    const runs = [...data.runs];
    runs.sort((a, b) => {
      let cmp = 0;
      switch (sortCol) {
        case "date":
          cmp = a.startedAt.localeCompare(b.startedAt);
          break;
        case "cost":
          cmp = a.totalCostUsd - b.totalCostUsd;
          break;
        case "tokens":
          cmp = a.inputTokens + a.outputTokens - (b.inputTokens + b.outputTokens);
          break;
        case "turns":
          cmp = a.numTurns - b.numTurns;
          break;
      }
      return sortAsc ? cmp : -cmp;
    });
    return runs;
  });

  function toggleSort(col: typeof sortCol) {
    if (sortCol === col) {
      sortAsc = !sortAsc;
    } else {
      sortCol = col;
      sortAsc = false;
    }
  }

  function sortIndicator(col: typeof sortCol): string {
    if (sortCol !== col) return "";
    return sortAsc ? " \u25B2" : " \u25BC";
  }

  async function loadData(days?: number) {
    const thisRequest = ++requestId;
    if (!data) loading = true; // Only show full spinner on initial load
    error = "";
    showFullScanMessage = false;

    // Delayed indicator: show message if full scan takes > 500ms
    const delayTimer = setTimeout(() => {
      if (thisRequest === requestId) {
        showFullScanMessage = true;
      }
    }, 500);

    try {
      let result: UsageOverview;
      if (scope === "global") {
        result = await api.getGlobalUsageOverview(days);
      } else {
        result = await api.getUsageOverview(days);
      }

      // Discard stale response if user switched tabs/scope while we were loading
      if (thisRequest !== requestId) {
        dbg("usage", "discarded stale response", { thisRequest, currentRequest: requestId });
        return;
      }

      data = result;
      dbg("usage", "loadData", {
        scope,
        days,
        scanMode: data?.scanMode,
        dailyLen: data?.daily.length,
        firstDaily: data?.daily[0],
        totalRuns: data?.totalRuns,
        byModelLen: data?.byModel.length,
      });
    } catch (e) {
      if (thisRequest !== requestId) return;
      error = String(e);
    } finally {
      clearTimeout(delayTimer);
      if (thisRequest === requestId) {
        loading = false;
        showFullScanMessage = false;
      }
    }
  }

  async function loadHeatmapData() {
    const token = ++heatmapRequestId;
    try {
      const result = await api.getHeatmapDaily(scope);
      if (token === heatmapRequestId) {
        heatmapDaily = result;
        dbg("usage", "heatmap loaded", { scope, days: result.length });
      } else {
        dbg("usage", "heatmap discarded stale", { token, current: heatmapRequestId });
      }
    } catch (e) {
      if (token === heatmapRequestId) {
        heatmapDaily = null;
        dbgWarn("usage", "heatmap load failed", e);
      }
    }
  }

  function selectRange(days: number | undefined) {
    selectedDays = days;
    loadData(days);
  }

  function selectScope(s: "app" | "global") {
    scope = s;
    // Reset chart mode if current mode isn't available for this scope
    if (s === "app" && (chartMode === "messages" || chartMode === "sessions")) {
      chartMode = "cost";
    }
    loadData(selectedDays);
    loadHeatmapData();
  }

  async function refreshCache() {
    if (refreshing) return;
    refreshing = true;
    try {
      await api.clearUsageCache();
      await Promise.all([loadData(selectedDays), loadHeatmapData()]);
    } finally {
      refreshing = false;
    }
  }

  function formatDate(isoStr: string): string {
    return fmtDate(isoStr);
  }

  function formatShortDate(dateStr: string): string {
    // dateStr is "YYYY-MM-DD"
    return dateStr.slice(5); // "MM-DD"
  }

  function getDailyValue(day: DailyAggregate): number {
    if (chartMode === "cost") return day.costUsd;
    if (chartMode === "messages") return day.messageCount ?? 0;
    if (chartMode === "sessions") return day.sessionCount ?? 0;
    return day.inputTokens + day.outputTokens;
  }

  function getDailyTooltip(day: DailyAggregate): string {
    const date = day.date;
    if (chartMode === "cost") return `${date}\n${formatCost(day.costUsd)}`;
    if (chartMode === "messages")
      return `${date}\n${t("usage_tooltipMessages", { count: fmtNumber(day.messageCount ?? 0) })}`;
    if (chartMode === "sessions")
      return `${date}\n${t("usage_tooltipSessions", { count: String(day.sessionCount ?? 0) })}`;
    return `${date}\n${t("usage_tooltipTokens", { count: formatTokenCount(day.inputTokens + day.outputTokens) })}`;
  }

  function formatAxisValue(v: number): string {
    if (chartMode === "cost") return formatCost(v);
    if (chartMode === "tokens") return formatTokenCount(v);
    if (v >= 1000) return `${(v / 1000).toFixed(1)}k`;
    return v.toFixed(0);
  }

  onMount(() => {
    loadData(selectedDays);
    loadHeatmapData();
    // Load balance helper + start auto-refresh
    let balanceTimer: ReturnType<typeof setInterval>;
    (async () => {
      try {
        const settings = await api.getUserSettings();
        balanceHelper = settings.balance_helper ?? null;
        packySessionInput = balanceHelper?.packy_session ?? "";
        packyTdcItokenInput = balanceHelper?.packy_tdc_itoken ?? "";
        packyUserIdInput = balanceHelper?.packy_user_id ?? "";
        mimoServiceTokenInput = balanceHelper?.mimo_service_token ?? "";
        mimoUserIdInput = balanceHelper?.mimo_user_id ?? "";
        mimoSlhInput = balanceHelper?.mimo_slh ?? "";
        mimoPhInput = balanceHelper?.mimo_ph ?? "";
        void refreshBalanceStatus("all");
        const secs = Math.max(
          60,
          Math.min(180, balanceHelper?.auto_refresh_secs ?? 120),
        );
        balanceTimer = setInterval(() => {
          void refreshBalanceStatus("all");
        }, secs * 1000);
      } catch (e) {
        dbgWarn("usage", "load balance helper failed", e);
      }
    })();
    return () => {
      if (balanceTimer) clearInterval(balanceTimer);
    };
  });
</script>

<div class="max-w-4xl mx-auto p-6 space-y-6 animate-slide-up">
  <!-- Header -->
  <div class="flex items-center gap-4">
    <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-emerald-500/10">
      <svg
        class="h-7 w-7 text-emerald-600 dark:text-emerald-400"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"><path d="M3 3v18h18" /><path d="m19 9-5 5-4-4-3 3" /></svg
      >
    </div>
    <div>
      <h1 class="text-2xl font-bold">{t("usage_title")}</h1>
      <p class="text-sm text-muted-foreground">{t("usage_subtitle")}</p>
    </div>
  </div>

  <!-- Scope tabs: App / Global -->
  <div class="flex items-center gap-4">
    <div class="flex gap-1 bg-muted/40 rounded-lg p-0.5">
      <button
        class="px-3 py-1.5 text-xs font-medium rounded-md transition-colors
          {scope === 'global'
          ? 'bg-background text-foreground shadow-sm'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => selectScope("global")}
      >
        {t("usage_scopeGlobal")}
      </button>
      <button
        class="px-3 py-1.5 text-xs font-medium rounded-md transition-colors
          {scope === 'app'
          ? 'bg-background text-foreground shadow-sm'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => selectScope("app")}
      >
        {t("usage_scopeApp")}
      </button>
    </div>

    <!-- Date range tabs -->
    <div class="flex gap-1">
      {#each DATE_RANGES as range}
        <button
          class="px-3 py-1.5 text-xs font-medium rounded-md transition-colors
            {selectedDays === range.days
            ? 'bg-primary text-primary-foreground'
            : 'bg-muted/50 text-muted-foreground hover:bg-muted'}"
          onclick={() => selectRange(range.days)}
        >
          {range.label}
        </button>
      {/each}
    </div>

    <!-- Refresh button (global scope only, stays in DOM to avoid layout shift) -->
    <button
      class="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors disabled:opacity-40 {scope !==
      'global'
        ? 'invisible'
        : ''}"
      title={t("usage_refreshTitle")}
      disabled={refreshing || scope !== "global"}
      onclick={refreshCache}
    >
      <svg
        class="h-4 w-4 {refreshing ? 'animate-spin' : ''}"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
        <path d="M3 3v5h5" />
        <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
        <path d="M16 21h5v-5" />
      </svg>
    </button>
  </div>

  {#if loading}
    <div class="flex flex-col items-center justify-center py-12 gap-3">
      <div
        class="h-6 w-6 border-2 border-primary/30 border-t-primary rounded-full animate-spin"
      ></div>
      {#if showFullScanMessage}
        <p class="text-sm text-muted-foreground animate-fade-in">
          {t("usage_firstLoadMessage")}
        </p>
      {/if}
    </div>
  {:else if error}
    <div
      class="rounded-lg border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive"
    >
      {error}
    </div>
  {:else if data}
    <!-- Balance card -->
    {#if balanceHelper}
      <Card class="p-6 space-y-5">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
              {t("settings_balance_title")}
            </h2>
            <p class="mt-1 text-xs text-muted-foreground">
              {t("settings_balance_desc")}
            </p>
          </div>
          <Button
            size="sm"
            variant="outline"
            disabled={balanceRefreshing}
            onclick={() => refreshBalanceStatus("all")}
          >
            {balanceRefreshing
              ? t("settings_balance_refreshing")
              : t("settings_balance_refresh")}
          </Button>
        </div>
        {#if balanceRefreshError}
          <p class="text-xs text-red-400">{balanceRefreshError}</p>
        {/if}

        {@const deepseek = balanceStatusText("deepseek")}
        {@const packy = balanceStatusText("packy")}
        {@const mimo = balanceStatusText("mimo")}
        <div class="grid gap-4 md:grid-cols-2">
          <!-- DeepSeek panel -->
          <div
            class="rounded-xl bg-gradient-to-br from-blue-500/10 via-blue-500/5 to-transparent border border-blue-500/20 p-5 space-y-3"
          >
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2.5">
                <div
                  class="flex h-9 w-9 items-center justify-center rounded-lg bg-blue-500/15"
                >
                  <svg
                    class="h-5 w-5 text-blue-400"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path d="M12 2L2 7l10 5 10-5-10-5z" />
                    <path d="M2 17l10 5 10-5" />
                    <path d="M2 12l10 5 10-5" />
                  </svg>
                </div>
                <div>
                  <div class="text-sm font-semibold">DeepSeek</div>
                  <div
                    class="flex items-center gap-1.5 text-xs text-muted-foreground"
                  >
                    <span
                      class="inline-block h-1.5 w-1.5 rounded-full {deepseek.dotClass}"
                    ></span>
                    {deepseek.label}
                  </div>
                </div>
              </div>
              <div class="text-right">
                <div class="text-lg font-bold tabular-nums">{deepseek.balance}</div>
                {#if deepseek.sub}
                  <div class="text-[10px] text-muted-foreground">{deepseek.sub}</div>
                {/if}
              </div>
            </div>
          </div>

          <!-- Packy panel -->
          <div
            class="rounded-xl bg-gradient-to-br from-emerald-500/10 via-emerald-500/5 to-transparent border border-emerald-500/20 p-5 space-y-3"
          >
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2.5">
                <div
                  class="flex h-9 w-9 items-center justify-center rounded-lg bg-emerald-500/15"
                >
                  <svg
                    class="h-5 w-5 text-emerald-400"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path d="M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z" />
                    <path d="M12 6v6l4 2" />
                  </svg>
                </div>
                <div>
                  <div class="text-sm font-semibold">Packy</div>
                  <div
                    class="flex items-center gap-1.5 text-xs text-muted-foreground"
                  >
                    <span
                      class="inline-block h-1.5 w-1.5 rounded-full {packy.dotClass}"
                    ></span>
                    {packy.label}
                  </div>
                </div>
              </div>
              <div class="text-right">
                <div class="text-lg font-bold tabular-nums">{packy.balance}</div>
                {#if packy.sub}
                  <div class="text-[10px] text-muted-foreground">{packy.sub}</div>
                {/if}
              </div>
            </div>

            <!-- Packy credential inputs (collapsible) -->
            <button
              class="w-full text-left text-xs text-muted-foreground hover:text-foreground transition-colors"
              onclick={() => (showPackyCredentials = !showPackyCredentials)}
            >
              <span class="flex items-center gap-1">
                <svg
                  class="h-3 w-3 transition-transform {showPackyCredentials ? 'rotate-90' : ''}"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path d="m9 18 6-6-6-6" />
                </svg>
                {t("settings_balance_packySession")}
              </span>
            </button>
            {#if showPackyCredentials}
              <div class="space-y-2">
                <Input
                  type="password"
                  placeholder={t("settings_balance_packySession")}
                  value={packySessionInput}
                  oninput={(e) =>
                    (packySessionInput = (e.currentTarget as HTMLInputElement).value)}
                />
                <Input
                  type="text"
                  placeholder={t("settings_balance_packyTdcItoken")}
                  value={packyTdcItokenInput}
                  oninput={(e) =>
                    (packyTdcItokenInput = (e.currentTarget as HTMLInputElement).value)}
                />
                <Input
                  type="text"
                  placeholder={t("settings_balance_packyUserId")}
                  value={packyUserIdInput}
                  oninput={(e) =>
                    (packyUserIdInput = (e.currentTarget as HTMLInputElement).value)}
                />
                <div class="flex justify-end gap-2">
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={balanceSaving}
                    onclick={clearPackyCredentials}
                  >
                    {t("settings_balance_clear")}
                  </Button>
                  <Button size="sm" disabled={balanceSaving} onclick={savePackyCredentials}>
                    {balanceSaving
                      ? t("settings_balance_saving")
                      : t("settings_balance_save")}
                  </Button>
                </div>
              </div>
            {/if}
          </div>

          <!-- MiMo panel -->
          <div
            class="rounded-xl bg-gradient-to-br from-amber-500/10 via-amber-500/5 to-transparent border border-amber-500/20 p-5 space-y-3"
          >
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2.5">
                <div
                  class="flex h-9 w-9 items-center justify-center rounded-lg bg-amber-500/15"
                >
                  <svg
                    class="h-5 w-5 text-amber-400"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                    stroke="none"
                  >
                    <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" />
                  </svg>
                </div>
                <div>
                  <div class="text-sm font-semibold">MiMo</div>
                  <div
                    class="flex items-center gap-1.5 text-xs text-muted-foreground"
                  >
                    <span
                      class="inline-block h-1.5 w-1.5 rounded-full {mimo.dotClass}"
                    ></span>
                    {mimo.label}
                  </div>
                </div>
              </div>
              <div class="text-right">
                <div class="text-lg font-bold tabular-nums">{mimo.balance}</div>
                {#if mimo.sub}
                  <div class="text-[10px] text-muted-foreground">{mimo.sub}</div>
                {/if}
              </div>
            </div>

            <!-- MiMo credential inputs (collapsible) -->
            <button
              class="w-full text-left text-xs text-muted-foreground hover:text-foreground transition-colors"
              onclick={() => (showMimoCredentials = !showMimoCredentials)}
            >
              <span class="flex items-center gap-1">
                <svg
                  class="h-3 w-3 transition-transform {showMimoCredentials ? 'rotate-90' : ''}"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path d="m9 18 6-6-6-6" />
                </svg>
                {t("settings_balance_mimoServiceToken")}
              </span>
            </button>
            {#if showMimoCredentials}
              <div class="space-y-2">
                <Input
                  type="password"
                  placeholder={t("settings_balance_mimoServiceToken")}
                  value={mimoServiceTokenInput}
                  oninput={(e) =>
                    (mimoServiceTokenInput = (e.currentTarget as HTMLInputElement).value)}
                />
                <Input
                  type="text"
                  placeholder={t("settings_balance_mimoUserId")}
                  value={mimoUserIdInput}
                  oninput={(e) =>
                    (mimoUserIdInput = (e.currentTarget as HTMLInputElement).value)}
                />
                <Input
                  type="text"
                  placeholder={t("settings_balance_mimoSlh")}
                  value={mimoSlhInput}
                  oninput={(e) =>
                    (mimoSlhInput = (e.currentTarget as HTMLInputElement).value)}
                />
                <Input
                  type="text"
                  placeholder={t("settings_balance_mimoPh")}
                  value={mimoPhInput}
                  oninput={(e) =>
                    (mimoPhInput = (e.currentTarget as HTMLInputElement).value)}
                />
                <div class="flex justify-end gap-2">
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={balanceSaving}
                    onclick={clearMimoCredentials}
                  >
                    {t("settings_balance_clear")}
                  </Button>
                  <Button size="sm" disabled={balanceSaving} onclick={saveMimoCredentials}>
                    {balanceSaving
                      ? t("settings_balance_saving")
                      : t("settings_balance_save")}
                  </Button>
                </div>
              </div>
            {/if}
          </div>
        </div>
      </Card>
    {/if}

    <!-- Summary cards -->
    <div class="grid grid-cols-2 gap-4 sm:grid-cols-4">
      <Card class="p-4 text-center">
        <p class="text-2xl font-bold">{formatCost(data.totalCostUsd)}</p>
        <p class="text-xs text-muted-foreground mt-1">{t("usage_totalCost")}</p>
      </Card>
      <Card class="p-4 text-center">
        <p class="text-2xl font-bold">{formatTokenCount(data.totalTokens)}</p>
        <p class="text-xs text-muted-foreground mt-1">{t("usage_totalTokens")}</p>
      </Card>
      <Card class="p-4 text-center">
        <p class="text-2xl font-bold">{data.totalRuns}</p>
        <p class="text-xs text-muted-foreground mt-1">
          {scope === "global" ? t("usage_sessions") : t("usage_runs")}
        </p>
      </Card>
      <Card class="p-4 text-center">
        {#if data.currentStreak > 0}
          <p class="text-2xl font-bold">
            {t("usage_currentStreak", { count: String(data.currentStreak) })}
          </p>
          <p class="text-xs text-muted-foreground mt-1">
            {t("usage_longestStreak", { count: String(data.longestStreak) })}
          </p>
        {:else}
          <p class="text-2xl font-bold">
            {t("usage_activeDays", { count: String(data.activeDays) })}
          </p>
          <p class="text-xs text-muted-foreground mt-1">
            {scope === "global" ? t("usage_avgCostSession") : t("usage_avgCostRun")}
          </p>
        {/if}
      </Card>
    </div>

    <!-- Activity Heatmap (always 52 weeks, independent of date filter) -->
    {#if heatmapDaily}
      <Card class="p-6 space-y-3">
        <div class="flex items-center justify-between">
          <h2 class="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
            {t("usage_activityHeatmap")}
          </h2>
          <div class="flex gap-3 text-xs text-muted-foreground">
            {#if data.activeDays > 0}
              <span>{t("usage_activeDays", { count: String(data.activeDays) })}</span>
            {/if}
            {#if data.longestStreak > 0}
              <span>{t("usage_longestStreak", { count: String(data.longestStreak) })}</span>
            {/if}
          </div>
        </div>
        <HeatmapCalendar daily={heatmapDaily} metric={chartMode} />
      </Card>
    {/if}

    <!-- Daily trend chart -->
    <Card class="p-6 space-y-4">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
          {t("usage_dailyTrend")}
        </h2>
        <div class="flex gap-1">
          <button
            class="px-2 py-0.5 text-[10px] font-medium rounded transition-colors
              {chartMode === 'cost'
              ? 'bg-primary/20 text-primary'
              : 'text-muted-foreground hover:text-foreground'}"
            onclick={() => (chartMode = "cost")}
          >
            {t("usage_chartCost")}
          </button>
          <button
            class="px-2 py-0.5 text-[10px] font-medium rounded transition-colors
              {chartMode === 'tokens'
              ? 'bg-primary/20 text-primary'
              : 'text-muted-foreground hover:text-foreground'}"
            onclick={() => (chartMode = "tokens")}
          >
            {t("usage_chartTokens")}
          </button>
          {#if scope === "global"}
            <button
              class="px-2 py-0.5 text-[10px] font-medium rounded transition-colors
                {chartMode === 'messages'
                ? 'bg-primary/20 text-primary'
                : 'text-muted-foreground hover:text-foreground'}"
              onclick={() => (chartMode = "messages")}
            >
              {t("usage_chartMessages")}
            </button>
            <button
              class="px-2 py-0.5 text-[10px] font-medium rounded transition-colors
                {chartMode === 'sessions'
                ? 'bg-primary/20 text-primary'
                : 'text-muted-foreground hover:text-foreground'}"
              onclick={() => (chartMode = "sessions")}
            >
              {t("usage_chartSessions")}
            </button>
          {/if}
        </div>
      </div>
      {#if data.daily.length > 0}
        {#if scope === "global" && chartMode === "tokens" && data.daily.some((d) => d.modelBreakdown)}
          <StackedModelChart daily={data.daily} />
        {:else}
          <div class="flex h-40">
            <!-- Y-axis labels -->
            <div
              class="flex flex-col justify-between items-end pr-2 text-[10px] text-muted-foreground tabular-nums shrink-0 py-0.5"
            >
              <span>{formatAxisValue(maxDailyValue)}</span>
              <span>{formatAxisValue(maxDailyValue / 2)}</span>
              <span>0</span>
            </div>
            <!-- Bars + X-axis -->
            <div class="flex-1 flex flex-col min-w-0">
              <div class="flex-1 flex gap-[2px] border-l border-b border-border/50 relative">
                <!-- 50% gridline -->
                <div
                  class="absolute inset-x-0 top-1/2 border-t border-border/30 pointer-events-none"
                ></div>
                {#each data.daily.slice(-30) as day}
                  {@const value = getDailyValue(day)}
                  {@const pct = Math.max((value / maxDailyValue) * 100, 2)}
                  <div
                    class="flex-1 min-w-0 flex items-end group cursor-default"
                    title={getDailyTooltip(day)}
                  >
                    <div
                      class="w-full rounded-t bg-primary/60 group-hover:bg-primary transition-colors"
                      style="height: {pct}%"
                    ></div>
                  </div>
                {/each}
              </div>
              <!-- X-axis date labels -->
              <div class="flex gap-[2px] mt-1">
                {#each data.daily.slice(-30) as day, i}
                  {@const showLabel =
                    data.daily.slice(-30).length <= 10 ||
                    i % Math.ceil(data.daily.slice(-30).length / 10) === 0}
                  <div class="flex-1 min-w-0 text-center">
                    {#if showLabel}
                      <span class="text-[10px] text-muted-foreground tabular-nums">
                        {formatShortDate(day.date)}
                      </span>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          </div>
        {/if}
      {:else}
        <p class="text-sm text-muted-foreground">{t("usage_noDailyData")}</p>
      {/if}
    </Card>

    <!-- By Model -->
    <Card class="p-6 space-y-4">
      <h2 class="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
        {t("usage_byModel")}
      </h2>
      {#if data.byModel.length > 0}
        <div class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead>
              <tr class="text-xs text-muted-foreground border-b border-border">
                <th class="text-left py-2 font-medium">{t("usage_thModel")}</th>
                {#if scope === "app"}
                  <th class="text-right py-2 font-medium">{t("usage_thRuns")}</th>
                {/if}
                <th class="text-right py-2 font-medium">{t("usage_thInTokens")}</th>
                <th class="text-right py-2 font-medium">{t("usage_thOutTokens")}</th>
                <th class="text-right py-2 font-medium">{t("usage_thCacheRead")}</th>
                <th class="text-right py-2 font-medium">{t("usage_thCacheWrite")}</th>
                <th class="text-right py-2 font-medium">{t("usage_thCost")}</th>
                <th class="text-right py-2 font-medium w-24">%</th>
              </tr>
            </thead>
            <tbody>
              {#each data.byModel as modelRow}
                <tr class="border-b border-border/50 hover:bg-muted/30">
                  <td class="py-2 font-mono text-xs truncate max-w-[180px]" title={modelRow.model}>
                    {modelRow.model}
                  </td>
                  {#if scope === "app"}
                    <td class="py-2 text-right tabular-nums">{modelRow.runs}</td>
                  {/if}
                  <td class="py-2 text-right tabular-nums font-mono text-xs">
                    {formatTokenCount(modelRow.inputTokens)}
                  </td>
                  <td class="py-2 text-right tabular-nums font-mono text-xs">
                    {formatTokenCount(modelRow.outputTokens)}
                  </td>
                  <td class="py-2 text-right tabular-nums font-mono text-xs text-muted-foreground">
                    {formatTokenCount(modelRow.cacheReadTokens)}
                  </td>
                  <td class="py-2 text-right tabular-nums font-mono text-xs text-muted-foreground">
                    {formatTokenCount(modelRow.cacheWriteTokens)}
                  </td>
                  <td class="py-2 text-right tabular-nums font-mono text-xs">
                    {formatCost(modelRow.costUsd)}
                  </td>
                  <td class="py-2 text-right">
                    <div class="flex items-center justify-end gap-2">
                      <div class="w-12 h-1.5 bg-muted rounded-full overflow-hidden">
                        <div
                          class="h-full bg-primary rounded-full"
                          style="width: {Math.min(modelRow.pct, 100)}%"
                        ></div>
                      </div>
                      <span class="text-xs tabular-nums text-muted-foreground w-8 text-right">
                        {modelRow.pct.toFixed(0)}%
                      </span>
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <p class="text-sm text-muted-foreground">{t("usage_noModelData")}</p>
      {/if}
    </Card>

    <!-- Run History (App mode only) -->
    {#if scope === "app"}
      <Card class="p-6 space-y-4">
        <h2 class="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
          {t("usage_runHistory")}
        </h2>
        {#if sortedRuns.length > 0}
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="text-xs text-muted-foreground border-b border-border">
                  <th
                    class="text-left py-2 font-medium cursor-pointer select-none hover:text-foreground"
                    onclick={() => toggleSort("date")}
                  >
                    {t("usage_thDate")}{sortIndicator("date")}
                  </th>
                  <th class="text-left py-2 font-medium">{t("usage_thName")}</th>
                  <th class="text-left py-2 font-medium">{t("usage_thModel")}</th>
                  <th
                    class="text-right py-2 font-medium cursor-pointer select-none hover:text-foreground"
                    onclick={() => toggleSort("tokens")}
                  >
                    {t("usage_thTokens")}{sortIndicator("tokens")}
                  </th>
                  <th
                    class="text-right py-2 font-medium cursor-pointer select-none hover:text-foreground"
                    onclick={() => toggleSort("cost")}
                  >
                    {t("usage_thCost")}{sortIndicator("cost")}
                  </th>
                  <th
                    class="text-right py-2 font-medium cursor-pointer select-none hover:text-foreground"
                    onclick={() => toggleSort("turns")}
                  >
                    {t("usage_thTurns")}{sortIndicator("turns")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {#each sortedRuns as run}
                  <tr
                    class="border-b border-border/50 hover:bg-muted/30 cursor-pointer"
                    onclick={() => goto(`/chat?run=${run.runId}`)}
                  >
                    <td class="py-2 text-xs text-muted-foreground whitespace-nowrap">
                      {formatDate(run.startedAt)}
                    </td>
                    <td class="py-2 truncate max-w-[200px]" title={run.name}>
                      {run.name}
                    </td>
                    <td
                      class="py-2 font-mono text-xs text-muted-foreground truncate max-w-[120px]"
                      title={run.model ?? run.agent}
                    >
                      {run.model ?? run.agent}
                    </td>
                    <td class="py-2 text-right tabular-nums font-mono text-xs">
                      {formatTokenCount(run.inputTokens + run.outputTokens)}
                    </td>
                    <td class="py-2 text-right tabular-nums font-mono text-xs">
                      {formatCost(run.totalCostUsd)}
                    </td>
                    <td class="py-2 text-right tabular-nums">
                      {run.numTurns}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <p class="text-sm text-muted-foreground">
            {t("usage_noUsageData")}
          </p>
        {/if}
      </Card>
    {/if}
  {/if}
</div>
