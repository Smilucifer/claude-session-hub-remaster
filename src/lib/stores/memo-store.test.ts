import { describe, it, expect, vi, beforeEach } from "vitest";
import type { MemoItem, MemoScope } from "$lib/types";

vi.mock("$lib/api", () => ({
  listMemos: vi.fn(),
  addMemo: vi.fn(),
  updateMemo: vi.fn(),
  deleteMemo: vi.fn(),
  clearMemos: vi.fn(),
}));

vi.mock("$lib/utils/debug", () => ({
  dbg: vi.fn(),
  dbgWarn: vi.fn(),
}));

import { MemoStore } from "./memo-store.svelte";
import * as api from "$lib/api";

const globalScope: MemoScope = { kind: "global" };

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function memo(id: string, text: string): MemoItem {
  return {
    id,
    text,
    createdAt: "2026-04-30T00:00:00Z",
    updatedAt: "2026-04-30T00:00:00Z",
  };
}

describe("MemoStore", () => {
  let store: MemoStore;

  beforeEach(() => {
    store = new MemoStore();
    vi.clearAllMocks();
  });

  it("starts with global scope and empty items", () => {
    expect(store.scope).toEqual(globalScope);
    expect(store.items).toEqual([]);
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it("loads items for the requested scope", async () => {
    const projectScope: MemoScope = { kind: "project", cwd: "D:\\work" };
    const items = [memo("1", "project note")];
    vi.mocked(api.listMemos).mockResolvedValue(items);

    await store.load(projectScope);

    expect(api.listMemos).toHaveBeenCalledWith(projectScope);
    expect(store.scope).toEqual(projectScope);
    expect(store.items).toEqual(items);
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it("adds returned item to current items", async () => {
    const item = memo("1", "new note");
    vi.mocked(api.addMemo).mockResolvedValue(item);

    await store.add("new note");

    expect(api.addMemo).toHaveBeenCalledWith(globalScope, "new note");
    expect(store.items).toEqual([item]);
  });

  it("updates an existing item in place", async () => {
    store.items = [memo("1", "old"), memo("2", "keep")];
    const updated = memo("1", "new");
    vi.mocked(api.updateMemo).mockResolvedValue(updated);

    await store.update("1", "new");

    expect(api.updateMemo).toHaveBeenCalledWith(globalScope, "1", "new");
    expect(store.items).toEqual([updated, memo("2", "keep")]);
  });

  it("deletes item locally after backend delete succeeds", async () => {
    store.items = [memo("1", "remove"), memo("2", "keep")];
    vi.mocked(api.deleteMemo).mockResolvedValue(undefined);

    await store.delete("1");

    expect(api.deleteMemo).toHaveBeenCalledWith(globalScope, "1");
    expect(store.items).toEqual([memo("2", "keep")]);
  });

  it("clears items after backend clear succeeds", async () => {
    store.items = [memo("1", "remove")];
    vi.mocked(api.clearMemos).mockResolvedValue(undefined);

    await store.clear();

    expect(api.clearMemos).toHaveBeenCalledWith(globalScope);
    expect(store.items).toEqual([]);
  });

  it("records load errors and clears stale items", async () => {
    store.items = [memo("1", "stale")];
    vi.mocked(api.listMemos).mockRejectedValue(new Error("boom"));

    await store.load(globalScope);

    expect(store.items).toEqual([]);
    expect(store.error).toContain("boom");
    expect(store.loading).toBe(false);
  });

  it("ignores stale load responses when scope changes quickly", async () => {
    const projectScope: MemoScope = { kind: "project", cwd: "D:\\work" };
    const globalLoad = deferred<MemoItem[]>();
    const projectLoad = deferred<MemoItem[]>();
    vi.mocked(api.listMemos)
      .mockReturnValueOnce(globalLoad.promise)
      .mockReturnValueOnce(projectLoad.promise);

    const first = store.load(globalScope);
    const second = store.load(projectScope);

    projectLoad.resolve([memo("project", "project note")]);
    await second;
    globalLoad.resolve([memo("global", "global note")]);
    await first;

    expect(store.scope).toEqual(projectScope);
    expect(store.items).toEqual([memo("project", "project note")]);
    expect(store.loading).toBe(false);
  });

  it("does not apply mutation results after switching scope", async () => {
    const projectScope: MemoScope = { kind: "project", cwd: "D:\\work" };
    const add = deferred<MemoItem>();
    vi.mocked(api.addMemo).mockReturnValueOnce(add.promise);
    vi.mocked(api.listMemos).mockResolvedValueOnce([memo("project", "project note")]);

    const pendingAdd = store.add("global note");
    await store.load(projectScope);

    add.resolve(memo("global", "global note"));
    await pendingAdd;

    expect(store.scope).toEqual(projectScope);
    expect(store.items).toEqual([memo("project", "project note")]);
  });
});
