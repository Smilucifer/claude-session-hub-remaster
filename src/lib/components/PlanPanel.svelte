<script lang="ts">
  import type { PlanArtifact, PlanStatus, TaskStatus, GroupChatParticipantDetail } from "$lib/types";
  import * as api from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";
  import { dbgWarn } from "$lib/utils/debug";

  let {
    plan,
    groupId,
    participants,
    onPlanUpdated,
  }: {
    plan: PlanArtifact | null;
    groupId: string;
    participants: GroupChatParticipantDetail[];
    onPlanUpdated: (plan: PlanArtifact | null) => void;
  } = $props();

  let creating = $state(false);
  let newTitle = $state("");
  let newTaskDesc = $state("");
  let newTasks = $state<{ description: string; assignee_id?: string }[]>([]);
  let notesText = $state("");
  let collapsed = $state(false);
  let saving = $state(false);

  // Sync notes when plan prop changes
  $effect(() => {
    const p = plan;
    if (p) notesText = p.user_notes ?? "";
  });

  function participantLabel(id: string | undefined): string {
    if (!id) return "";
    const p = participants.find((pp) => pp.participant.id === id);
    return p?.participant.label ?? id.slice(0, 6);
  }

  function statusBadge(status: PlanStatus): string {
    switch (status) {
      case "draft": return "bg-yellow-500/15 text-yellow-400";
      case "active": return "bg-green-500/15 text-green-400";
      case "completed": return "bg-blue-500/15 text-blue-400";
    }
  }

  function taskStatusIcon(status: TaskStatus): string {
    switch (status) {
      case "todo": return "bg-muted-foreground/30";
      case "in_progress": return "bg-yellow-500";
      case "done": return "bg-green-500";
      case "blocked": return "bg-red-500";
    }
  }

  function taskStatusLabel(status: TaskStatus): string {
    switch (status) {
      case "todo": return t("planTaskStatus_todo");
      case "in_progress": return t("planTaskStatus_inProgress");
      case "done": return t("planTaskStatus_done");
      case "blocked": return t("planTaskStatus_blocked");
    }
  }

  // ── Create plan ──

  function addTaskRow() {
    if (!newTaskDesc.trim()) return;
    newTasks = [...newTasks, { description: newTaskDesc.trim() }];
    newTaskDesc = "";
  }

  function removeTaskRow(idx: number) {
    newTasks = newTasks.filter((_, i) => i !== idx);
  }

  async function handleCreate() {
    if (!newTitle.trim() || creating) return;
    creating = true;
    try {
      const inputs = newTasks.map((t) => ({ description: t.description, assignee_id: t.assignee_id }));
      const created = await api.createPlan(groupId, newTitle.trim(), inputs);
      onPlanUpdated(created);
      newTitle = "";
      newTasks = [];
      newTaskDesc = "";
    } catch (e) {
      dbgWarn("PlanPanel", "createPlan failed", e);
    } finally {
      creating = false;
    }
  }

  // ── Approve / Complete ──

  async function handleApprove() {
    if (!plan || saving) return;
    saving = true;
    try {
      const updated = await api.approvePlan(plan.id);
      onPlanUpdated(updated);
    } catch (e) {
      dbgWarn("PlanPanel", "approvePlan failed", e);
    } finally {
      saving = false;
    }
  }

  async function handleComplete() {
    if (!plan || saving) return;
    saving = true;
    try {
      const updated = await api.completePlan(plan.id);
      onPlanUpdated(updated);
    } catch (e) {
      dbgWarn("PlanPanel", "completePlan failed", e);
    } finally {
      saving = false;
    }
  }

  // ── Toggle task status (cycle: todo -> in_progress -> done -> todo) ──

  async function cycleTaskStatus(taskId: string) {
    if (!plan || saving) return;
    const task = plan.tasks.find((t) => t.id === taskId);
    if (!task) return;
    const nextStatus: TaskStatus =
      task.status === "todo" ? "in_progress" :
      task.status === "in_progress" ? "done" :
      "todo";
    const updatedTasks = plan.tasks.map((t) =>
      t.id === taskId ? { ...t, status: nextStatus } : t
    );
    saving = true;
    try {
      const updated = await api.updatePlan(plan.id, undefined, updatedTasks);
      onPlanUpdated(updated);
    } catch (e) {
      dbgWarn("PlanPanel", "updateTask failed", e);
    } finally {
      saving = false;
    }
  }

  // ── Save notes ──

  async function saveNotes() {
    if (!plan || saving) return;
    saving = true;
    try {
      const updated = await api.updatePlan(plan.id, undefined, undefined, notesText);
      onPlanUpdated(updated);
    } catch (e) {
      dbgWarn("PlanPanel", "saveNotes failed", e);
    } finally {
      saving = false;
    }
  }
</script>

<!-- Plan panel header -->
<div class="border-b border-border px-3 py-2 flex items-center justify-between shrink-0">
  <button
    class="flex items-center gap-1.5 text-xs font-semibold text-muted-foreground hover:text-foreground transition-colors"
    onclick={() => (collapsed = !collapsed)}
  >
    <svg class="h-3 w-3 transition-transform {collapsed ? '' : 'rotate-90'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="m9 18 6-6-6-6" />
    </svg>
    {t("planPanel_title")}
  </button>
  {#if plan}
    <span class="text-[10px] font-medium rounded px-1.5 py-0.5 {statusBadge(plan.status)}">
      {t(`planStatus_${plan.status}`)}
    </span>
  {/if}
</div>

{#if !collapsed}
  <div class="overflow-y-auto px-3 py-2 space-y-3">
    {#if plan}
      <!-- Plan title -->
      <h4 class="text-xs font-semibold text-foreground">{plan.title}</h4>

      <!-- Task checklist -->
      {#if plan.tasks.length > 0}
        <div class="space-y-1">
          {#each plan.tasks as task (task.id)}
            <button
              class="flex items-center gap-2 w-full text-left rounded-md px-2 py-1.5 hover:bg-accent/30 transition-colors group"
              onclick={() => cycleTaskStatus(task.id)}
              disabled={saving}
              title={taskStatusLabel(task.status)}
            >
              <span class="w-2 h-2 rounded-full shrink-0 {taskStatusIcon(task.status)}"></span>
              <span class="text-[11px] text-foreground flex-1 {task.status === 'done' ? 'line-through opacity-60' : ''}">{task.description}</span>
              {#if task.assignee_id}
                <span class="text-[10px] text-muted-foreground/60 shrink-0">@{participantLabel(task.assignee_id)}</span>
              {/if}
            </button>
          {/each}
        </div>
      {:else}
        <p class="text-[11px] text-muted-foreground/60">{t("planPanel_noTasks")}</p>
      {/if}

      <!-- Action buttons -->
      <div class="flex items-center gap-1.5">
        {#if plan.status === "draft"}
          <button
            class="h-6 rounded-md bg-green-500/15 text-green-400 px-2 text-[11px] font-medium hover:bg-green-500/25 transition-colors disabled:opacity-50"
            onclick={handleApprove}
            disabled={saving}
          >
            {t("planPanel_approve")}
          </button>
        {/if}
        {#if plan.status === "active"}
          <button
            class="h-6 rounded-md bg-blue-500/15 text-blue-400 px-2 text-[11px] font-medium hover:bg-blue-500/25 transition-colors disabled:opacity-50"
            onclick={handleComplete}
            disabled={saving}
          >
            {t("planPanel_complete")}
          </button>
        {/if}
      </div>

      <!-- User notes -->
      <div>
        <label class="text-[11px] text-muted-foreground/60 mb-1 block" for="plan-notes">{t("planPanel_notes")}</label>
        <textarea
          id="plan-notes"
          bind:value={notesText}
          rows={2}
          placeholder={t("planPanel_notesPlaceholder")}
          class="w-full resize-none rounded-md border border-border bg-background px-2 py-1.5 text-[11px] text-foreground placeholder:text-muted-foreground/40 focus:outline-none focus:border-ring"
          onblur={saveNotes}
        ></textarea>
      </div>

    {:else}
      <!-- Create plan form -->
      <div class="space-y-2">
        <h4 class="text-[11px] font-semibold text-muted-foreground">{t("planPanel_createTitle")}</h4>
        <input
          type="text"
          bind:value={newTitle}
          placeholder={t("planPanel_titlePlaceholder")}
          class="w-full h-7 rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-ring"
        />

        <!-- Add tasks -->
        <div class="space-y-1">
          {#each newTasks as task, i (i)}
            <div class="flex items-center gap-1.5">
              <span class="text-[11px] text-muted-foreground/60">{i + 1}.</span>
              <span class="text-[11px] text-foreground flex-1 truncate">{task.description}</span>
              <button
                class="h-4 w-4 flex items-center justify-center rounded text-muted-foreground/40 hover:text-destructive transition-colors"
                onclick={() => removeTaskRow(i)}
                aria-label="Remove task"
              >
                <svg class="h-2.5 w-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6 6 18" /><path d="m6 6 12 12" /></svg>
              </button>
            </div>
          {/each}
        </div>

        <div class="flex items-center gap-1.5">
          <input
            type="text"
            bind:value={newTaskDesc}
            placeholder={t("planPanel_taskPlaceholder")}
            class="flex-1 h-7 rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-ring"
            onkeydown={(e) => { if (e.key === "Enter") { e.preventDefault(); addTaskRow(); } }}
          />
          <button
            class="h-7 rounded-md border border-border px-2 text-[11px] text-muted-foreground hover:bg-accent transition-colors"
            onclick={addTaskRow}
          >
            {t("planPanel_addTask")}
          </button>
        </div>

        <button
          class="h-7 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50"
          onclick={handleCreate}
          disabled={creating || !newTitle.trim()}
        >
          {creating ? t("planPanel_creating") : t("planPanel_create")}
        </button>
      </div>
    {/if}
  </div>
{/if}
