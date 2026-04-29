import * as api from "$lib/api";
import type { MemoItem, MemoScope } from "$lib/types";
import { dbg, dbgWarn } from "$lib/utils/debug";

function scopeKey(scope: MemoScope): string {
  return scope.kind === "project" ? `project:${scope.cwd}` : "global";
}

export class MemoStore {
  scope = $state<MemoScope>({ kind: "global" });
  items = $state<MemoItem[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);

  #loadSeq = 0;

  async load(scope: MemoScope = this.scope): Promise<void> {
    const seq = ++this.#loadSeq;
    this.scope = scope;
    this.loading = true;
    this.error = null;
    try {
      const items = await api.listMemos(scope);
      if (seq !== this.#loadSeq) return;
      this.items = items;
      dbg("memos", "load", { scope, count: items.length });
    } catch (e) {
      if (seq !== this.#loadSeq) return;
      this.error = String(e);
      this.items = [];
      dbgWarn("memos", "load error", e);
    } finally {
      if (seq === this.#loadSeq) {
        this.loading = false;
      }
    }
  }

  async add(text: string): Promise<MemoItem> {
    this.#loadSeq++;
    const scope = this.scope;
    const key = scopeKey(scope);
    this.error = null;
    try {
      const item = await api.addMemo(scope, text);
      if (scopeKey(this.scope) === key) {
        this.items = [...this.items, item];
      }
      return item;
    } catch (e) {
      if (scopeKey(this.scope) === key) {
        this.error = String(e);
      }
      dbgWarn("memos", "add error", e);
      throw e;
    }
  }

  async update(id: string, text: string): Promise<MemoItem> {
    this.#loadSeq++;
    const scope = this.scope;
    const key = scopeKey(scope);
    this.error = null;
    try {
      const item = await api.updateMemo(scope, id, text);
      if (scopeKey(this.scope) === key) {
        this.items = this.items.map((existing) => (existing.id === id ? item : existing));
      }
      return item;
    } catch (e) {
      if (scopeKey(this.scope) === key) {
        this.error = String(e);
      }
      dbgWarn("memos", "update error", e);
      throw e;
    }
  }

  async delete(id: string): Promise<void> {
    this.#loadSeq++;
    const scope = this.scope;
    const key = scopeKey(scope);
    this.error = null;
    try {
      await api.deleteMemo(scope, id);
      if (scopeKey(this.scope) === key) {
        this.items = this.items.filter((item) => item.id !== id);
      }
    } catch (e) {
      if (scopeKey(this.scope) === key) {
        this.error = String(e);
      }
      dbgWarn("memos", "delete error", e);
      throw e;
    }
  }

  async clear(): Promise<void> {
    this.#loadSeq++;
    const scope = this.scope;
    const key = scopeKey(scope);
    this.error = null;
    try {
      await api.clearMemos(scope);
      if (scopeKey(this.scope) === key) {
        this.items = [];
      }
    } catch (e) {
      if (scopeKey(this.scope) === key) {
        this.error = String(e);
      }
      dbgWarn("memos", "clear error", e);
      throw e;
    }
  }
}
