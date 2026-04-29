<script lang="ts">
  import Modal from "$lib/components/Modal.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { MemoStore } from "$lib/stores/memo-store.svelte";
  import type { MemoScope } from "$lib/types";

  let {
    open = $bindable(false),
    cwd = "",
  }: {
    open: boolean;
    cwd?: string;
  } = $props();

  const store = new MemoStore();

  let selectedScope = $state<"global" | "project">("global");
  let draft = $state("");
  let editingId = $state<string | null>(null);
  let editingText = $state("");
  let copiedId = $state<string | null>(null);
  let lastLoadKey = "";

  const hasProject = $derived(Boolean(cwd && cwd.trim()));

  function currentScope(): MemoScope {
    if (selectedScope === "project" && hasProject) {
      return { kind: "project", cwd };
    }
    return { kind: "global" };
  }

  $effect(() => {
    if (selectedScope === "project" && !hasProject) {
      selectedScope = "global";
    }

    if (!open) {
      lastLoadKey = "";
      editingId = null;
      copiedId = null;
      return;
    }

    const key = `${selectedScope}:${hasProject ? cwd : ""}`;
    if (key !== lastLoadKey) {
      lastLoadKey = key;
      void store.load(currentScope());
    }
  });

  async function addMemo() {
    const text = draft.trim();
    if (!text) return;
    await store.add(text);
    draft = "";
  }

  function startEdit(id: string, text: string) {
    editingId = id;
    editingText = text;
  }

  async function saveEdit(id: string) {
    const text = editingText.trim();
    if (!text) return;
    await store.update(id, text);
    editingId = null;
    editingText = "";
  }

  async function copyMemo(id: string, text: string) {
    await navigator.clipboard.writeText(text);
    copiedId = id;
    window.setTimeout(() => {
      if (copiedId === id) copiedId = null;
    }, 1200);
  }

  async function clearAll() {
    if (store.items.length === 0) return;
    if (!window.confirm(t("memo_clearConfirm"))) return;
    await store.clear();
  }
</script>

<Modal bind:open title={t("memo_title")}>
  <div class="flex max-h-[70vh] flex-col gap-4">
    <div class="flex items-center justify-between gap-3">
      <div class="inline-flex rounded-md border border-border p-0.5">
        <button
          type="button"
          class="h-8 px-3 text-xs font-medium rounded-sm transition-colors {selectedScope ===
          'global'
            ? 'bg-primary text-primary-foreground'
            : 'text-muted-foreground hover:bg-muted'}"
          onclick={() => (selectedScope = "global")}
        >
          {t("memo_scopeGlobal")}
        </button>
        <button
          type="button"
          class="h-8 px-3 text-xs font-medium rounded-sm transition-colors disabled:opacity-40 {selectedScope ===
          'project'
            ? 'bg-primary text-primary-foreground'
            : 'text-muted-foreground hover:bg-muted'}"
          disabled={!hasProject}
          onclick={() => (selectedScope = "project")}
        >
          {t("memo_scopeProject")}
        </button>
      </div>

      <button
        type="button"
        class="h-8 px-3 text-xs text-muted-foreground hover:text-foreground disabled:opacity-40"
        disabled={store.items.length === 0}
        onclick={clearAll}
      >
        {t("memo_clear")}
      </button>
    </div>

    <form
      class="flex gap-2"
      onsubmit={(e) => {
        e.preventDefault();
        void addMemo();
      }}
    >
      <input
        class="h-9 flex-1 rounded-md border border-input bg-background px-3 text-sm outline-none focus:ring-2 focus:ring-ring"
        bind:value={draft}
        placeholder={t("memo_addPlaceholder")}
      />
      <button
        type="submit"
        class="h-9 rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground disabled:opacity-40"
        disabled={!draft.trim()}
      >
        {t("memo_add")}
      </button>
    </form>

    {#if store.error}
      <div
        class="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive"
      >
        {store.error}
      </div>
    {/if}

    <div class="min-h-24 overflow-y-auto">
      {#if store.loading}
        <div class="py-8 text-center text-sm text-muted-foreground">{t("common_loading")}</div>
      {:else if store.items.length === 0}
        <div class="py-8 text-center text-sm text-muted-foreground">{t("memo_empty")}</div>
      {:else}
        <ul class="space-y-2">
          {#each store.items as item (item.id)}
            <li class="rounded-md border border-border bg-muted/20 p-3">
              {#if editingId === item.id}
                <textarea
                  class="min-h-20 w-full resize-y rounded-md border border-input bg-background p-2 text-sm outline-none focus:ring-2 focus:ring-ring"
                  bind:value={editingText}
                ></textarea>
                <div class="mt-2 flex justify-end gap-2">
                  <button
                    type="button"
                    class="h-8 px-2 text-xs text-muted-foreground hover:text-foreground"
                    onclick={() => (editingId = null)}
                  >
                    {t("common_cancel")}
                  </button>
                  <button
                    type="button"
                    class="h-8 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground disabled:opacity-40"
                    disabled={!editingText.trim()}
                    onclick={() => void saveEdit(item.id)}
                  >
                    {t("common_save")}
                  </button>
                </div>
              {:else}
                <p class="whitespace-pre-wrap break-words text-sm leading-5 text-foreground">
                  {item.text}
                </p>
                <div class="mt-3 flex items-center justify-between gap-2">
                  <span class="text-[11px] text-muted-foreground">
                    {new Date(item.updatedAt).toLocaleString()}
                  </span>
                  <div class="flex gap-1">
                    <button
                      type="button"
                      class="h-7 px-2 text-xs text-muted-foreground hover:text-foreground"
                      onclick={() => void copyMemo(item.id, item.text)}
                    >
                      {copiedId === item.id ? t("common_copied") : t("common_copy")}
                    </button>
                    <button
                      type="button"
                      class="h-7 px-2 text-xs text-muted-foreground hover:text-foreground"
                      onclick={() => startEdit(item.id, item.text)}
                    >
                      {t("common_edit")}
                    </button>
                    <button
                      type="button"
                      class="h-7 px-2 text-xs text-destructive hover:text-destructive/80"
                      onclick={() => void store.delete(item.id)}
                    >
                      {t("memo_delete")}
                    </button>
                  </div>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
</Modal>
