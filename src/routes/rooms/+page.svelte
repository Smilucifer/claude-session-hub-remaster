<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { listRuns } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";
  import { RoomStore } from "$lib/stores/room-store.svelte";
  import type { TaskRun } from "$lib/types";
  import { relativeTime, truncate } from "$lib/utils/format";

  const store = new RoomStore();

  let runs = $state<TaskRun[]>([]);
  let createName = $state("");
  let createDescription = $state("");
  let createCwd = $state("");
  let attachRunId = $state("");
  let participantPrompt = $state("");
  let participantCwd = $state("");
  let memoDraft = $state("");
  let deletingRoomId = $state("");

  let claudeRuns = $derived(runs.filter((run) => run.agent === "claude"));
  let selectedRunIds = $derived(
    new Set(store.room?.participants.map((p) => p.participant.run_id) ?? []),
  );
  let attachableRuns = $derived(claudeRuns.filter((run) => !selectedRunIds.has(run.id)));

  onMount(async () => {
    await Promise.all([store.loadRooms(), loadRuns()]);
    if (store.rooms.length > 0 && !store.selectedRoomId) {
      await selectRoom(store.rooms[0].id);
    }
  });

  async function loadRuns() {
    runs = await listRuns();
  }

  async function refreshParticipants() {
    await Promise.all([
      loadRuns(),
      store.selectedRoomId ? store.selectRoom(store.selectedRoomId) : Promise.resolve(),
    ]);
  }

  async function selectRoom(id: string) {
    await store.selectRoom(id);
    memoDraft = store.room?.memo ?? "";
    participantCwd = store.room?.cwd ?? "";
    attachRunId = "";
  }

  async function handleCreateRoom() {
    const name = createName.trim();
    if (!name) return;
    await store.createRoom(name, createDescription.trim(), createCwd.trim() || undefined);
    createName = "";
    createDescription = "";
    createCwd = "";
    memoDraft = store.room?.memo ?? "";
    participantCwd = store.room?.cwd ?? "";
  }

  async function handleAttachRun() {
    if (!attachRunId) return;
    await store.attachRun(attachRunId);
    attachRunId = "";
  }

  async function handleCreateParticipant() {
    const prompt = participantPrompt.trim();
    const cwd = participantCwd.trim() || store.room?.cwd || "/";
    if (!prompt) return;
    await store.createClaudeParticipant(prompt, cwd, undefined, undefined, "Claude", "participant");
    participantPrompt = "";
    await loadRuns();
  }

  async function handleSaveMemo() {
    await store.updateMemo(memoDraft);
  }

  async function handleDeleteRoom(id: string) {
    await store.deleteRoom(id);
    deletingRoomId = "";
    if (store.rooms.length > 0) {
      await selectRoom(store.rooms[0].id);
    }
  }

  function runLabel(run?: TaskRun): string {
    if (!run) return t("room_missingRun");
    return run.name || run.last_message_preview || run.prompt || run.id;
  }

  function statusClass(status?: string): string {
    if (status === "running" || status === "idle") return "bg-emerald-500/15 text-emerald-500";
    if (status === "failed") return "bg-red-500/15 text-red-500";
    if (status === "pending") return "bg-amber-500/15 text-amber-500";
    return "bg-muted text-muted-foreground";
  }
</script>

<div class="flex h-full min-h-0 bg-background">
  <aside class="w-72 shrink-0 border-r border-border bg-muted/20">
    <div class="border-b border-border p-3">
      <div class="flex items-center justify-between gap-2">
        <h1 class="text-sm font-semibold">{t("room_title")}</h1>
        <button
          class="rounded-md border border-border px-2 py-1 text-xs hover:bg-accent"
          onclick={() => store.loadRooms()}
        >
          {t("room_refresh")}
        </button>
      </div>
      <div class="mt-3 space-y-2">
        <input
          class="w-full rounded-md border border-border bg-background px-2 py-1.5 text-sm"
          placeholder={t("room_namePlaceholder")}
          bind:value={createName}
        />
        <input
          class="w-full rounded-md border border-border bg-background px-2 py-1.5 text-sm"
          placeholder={t("room_descriptionPlaceholder")}
          bind:value={createDescription}
        />
        <input
          class="w-full rounded-md border border-border bg-background px-2 py-1.5 text-sm"
          placeholder={t("room_projectPathPlaceholder")}
          bind:value={createCwd}
        />
        <button
          class="w-full rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground disabled:opacity-50"
          disabled={!createName.trim() || store.saving}
          onclick={handleCreateRoom}
        >
          {t("room_create")}
        </button>
      </div>
    </div>

    <div class="h-[calc(100%-184px)] overflow-y-auto p-2">
      {#if store.loading && store.rooms.length === 0}
        <div class="flex justify-center py-8">
          <div
            class="h-4 w-4 animate-spin rounded-full border-2 border-primary/30 border-t-primary"
          ></div>
        </div>
      {:else if store.rooms.length === 0}
        <p class="px-2 py-8 text-center text-sm text-muted-foreground">{t("room_empty")}</p>
      {:else}
        {#each store.rooms as room}
          <button
            class="mb-1 flex w-full flex-col gap-1 rounded-md px-2.5 py-2 text-left transition-colors
              {store.selectedRoomId === room.id
              ? 'bg-accent text-accent-foreground'
              : 'hover:bg-accent/50'}"
            onclick={() => selectRoom(room.id)}
          >
            <div class="flex items-center gap-2">
              <span class="min-w-0 flex-1 truncate text-sm font-medium">{room.name}</span>
              <span class="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground"
                >{room.participant_count}</span
              >
            </div>
            {#if room.description}
              <p class="truncate text-xs text-muted-foreground">{room.description}</p>
            {/if}
            {#if room.memo_preview}
              <p class="truncate text-xs text-muted-foreground/80">{room.memo_preview}</p>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  </aside>

  <section class="flex min-w-0 flex-1 flex-col">
    {#if store.error}
      <div
        class="border-b border-destructive/30 bg-destructive/10 px-5 py-2 text-sm text-destructive"
      >
        {store.error}
      </div>
    {/if}
    {#if store.room}
      <div class="flex items-start justify-between gap-4 border-b border-border px-5 py-4">
        <div class="min-w-0">
          <h2 class="truncate text-lg font-semibold">{store.room.name}</h2>
          <div class="mt-1 flex flex-wrap gap-2 text-xs text-muted-foreground">
            {#if store.room.description}<span>{store.room.description}</span>{/if}
            {#if store.room.cwd}<span>{store.room.cwd}</span>{/if}
            <span
              >{t("room_participantsCount", {
                count: String(store.room.participants.length),
              })}</span
            >
          </div>
        </div>
        <div class="flex shrink-0 items-center gap-2">
          {#if deletingRoomId === store.room.id}
            <span class="text-xs text-muted-foreground">{t("room_deletePrompt")}</span>
            <button
              class="rounded-md bg-destructive px-2 py-1 text-xs text-destructive-foreground"
              onclick={() => handleDeleteRoom(store.room!.id)}
            >
              {t("room_confirmYes")}
            </button>
            <button
              class="rounded-md border border-border px-2 py-1 text-xs"
              onclick={() => (deletingRoomId = "")}
            >
              {t("room_confirmNo")}
            </button>
          {:else}
            <button
              class="rounded-md border border-border px-2 py-1 text-xs hover:bg-accent"
              onclick={() => (deletingRoomId = store.room!.id)}
            >
              {t("room_delete")}
            </button>
          {/if}
        </div>
      </div>

      <div class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_320px] overflow-hidden">
        <div class="min-w-0 overflow-y-auto p-5">
          <div class="mb-4 flex items-center justify-between">
            <h3 class="text-sm font-semibold">{t("room_participants")}</h3>
            <button
              class="rounded-md border border-border px-2 py-1 text-xs hover:bg-accent"
              onclick={refreshParticipants}
            >
              {t("room_refreshRuns")}
            </button>
          </div>

          <div class="space-y-2">
            {#if store.room.participants.length === 0}
              <p
                class="rounded-md border border-dashed border-border px-3 py-8 text-center text-sm text-muted-foreground"
              >
                {t("room_noParticipants")}
              </p>
            {:else}
              {#each store.room.participants as item}
                <div class="rounded-md border border-border bg-card p-3">
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <div class="flex items-center gap-2">
                        <span class="text-sm font-medium">{item.participant.label}</span>
                        <span
                          class="rounded px-1.5 py-0.5 text-[10px] {statusClass(item.run?.status)}"
                          >{item.run?.status ?? "missing"}</span
                        >
                      </div>
                      <p class="mt-1 line-clamp-2 text-sm text-muted-foreground">
                        {truncate(runLabel(item.run), 180)}
                      </p>
                      <div class="mt-2 flex flex-wrap gap-2 text-xs text-muted-foreground">
                        <span>{item.participant.agent}</span>
                        <span>{item.participant.role}</span>
                        {#if item.run?.model}<span>{item.run.model}</span>{/if}
                        {#if item.run?.last_activity_at}
                          <span>{relativeTime(item.run.last_activity_at)}</span>
                        {/if}
                      </div>
                    </div>
                    <button
                      class="shrink-0 rounded-md border border-border px-2 py-1 text-xs hover:bg-accent"
                      onclick={() => goto(`/chat?run=${item.participant.run_id}`)}
                    >
                      {t("room_open")}
                    </button>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        </div>

        <aside class="min-h-0 overflow-y-auto border-l border-border p-4">
          <div class="space-y-6">
            <section>
              <h3 class="mb-2 text-sm font-semibold">{t("room_attachRun")}</h3>
              <select
                class="w-full rounded-md border border-border bg-background px-2 py-1.5 text-sm"
                bind:value={attachRunId}
              >
                <option value="">{t("room_selectClaudeRun")}</option>
                {#each attachableRuns as run}
                  <option value={run.id}>{truncate(runLabel(run), 80)}</option>
                {/each}
              </select>
              <button
                class="mt-2 w-full rounded-md border border-border px-3 py-1.5 text-sm hover:bg-accent disabled:opacity-50"
                disabled={!attachRunId || store.saving}
                onclick={handleAttachRun}
              >
                {t("room_attach")}
              </button>
            </section>

            <section>
              <h3 class="mb-2 text-sm font-semibold">{t("room_newClaudeParticipant")}</h3>
              <textarea
                class="min-h-24 w-full resize-y rounded-md border border-border bg-background px-2 py-1.5 text-sm"
                placeholder={t("room_initialPromptPlaceholder")}
                bind:value={participantPrompt}
              ></textarea>
              <input
                class="mt-2 w-full rounded-md border border-border bg-background px-2 py-1.5 text-sm"
                placeholder={t("room_projectPathPlaceholder")}
                bind:value={participantCwd}
              />
              <button
                class="mt-2 w-full rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground disabled:opacity-50"
                disabled={!participantPrompt.trim() || store.saving}
                onclick={handleCreateParticipant}
              >
                {t("room_createParticipant")}
              </button>
            </section>

            <section>
              <h3 class="mb-2 text-sm font-semibold">{t("room_memo")}</h3>
              <textarea
                class="min-h-32 w-full resize-y rounded-md border border-border bg-background px-2 py-1.5 text-sm"
                bind:value={memoDraft}
              ></textarea>
              <button
                class="mt-2 w-full rounded-md border border-border px-3 py-1.5 text-sm hover:bg-accent disabled:opacity-50"
                disabled={store.saving}
                onclick={handleSaveMemo}
              >
                {t("room_saveMemo")}
              </button>
            </section>
          </div>
        </aside>
      </div>
    {:else}
      <div class="flex h-full items-center justify-center">
        <p class="text-sm text-muted-foreground">{t("room_selectOrCreate")}</p>
      </div>
    {/if}
  </section>
</div>
