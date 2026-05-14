import * as api from "$lib/api";
import type { CommunityInfo, KnowledgeGapInfo, MemoryGraphData, MemoryNode } from "$lib/types";

export class CharacterMemoryStore {
  characterId = $state<string | null>(null);
  memories = $state<MemoryNode[]>([]);
  graph = $state<MemoryGraphData | null>(null);
  communities = $state<CommunityInfo[]>([]);
  gaps = $state<KnowledgeGapInfo[]>([]);
  loading = $state(false);
  activeTab = $state<"memories" | "graph" | "gaps" | "communities" | "review">("memories");
  searchQuery = $state("");
  sortBy = $state<"newest" | "confidence">("newest");

  async load(characterId: string) {
    this.characterId = characterId;
    this.loading = true;
    const [memories, graph, communities, gaps] = await Promise.all([
      api.listCharacterMemories(characterId).catch(() => [] as MemoryNode[]),
      api.getMemoryGraph(characterId).catch(() => null),
      api.getMemoryCommunities(characterId).catch(() => [] as CommunityInfo[]),
      api.getKnowledgeGaps(characterId).catch(() => [] as KnowledgeGapInfo[]),
    ]);
    this.memories = memories;
    this.graph = graph;
    this.communities = communities;
    this.gaps = gaps;
    this.loading = false;
  }

  get sortedMemories(): MemoryNode[] {
    let filtered = this.memories;
    if (this.searchQuery) {
      const q = this.searchQuery.toLowerCase();
      filtered = filtered.filter(
        (m) =>
          m.content.toLowerCase().includes(q) ||
          m.tags.some((t) => t.toLowerCase().includes(q)),
      );
    }
    if (this.sortBy === "newest") {
      return [...filtered].sort((a, b) => b.created_at.localeCompare(a.created_at));
    }
    if (this.sortBy === "confidence") {
      return [...filtered].sort((a, b) => b.confidence - a.confidence);
    }
    return filtered;
  }

  async addMemory(content: string, type: MemoryNode["type"], confidence: number, tags: string[]) {
    if (!this.characterId) return;
    const node = await api.createCharacterMemory(this.characterId, content, type, confidence, tags);
    this.memories = [node, ...this.memories];
  }

  async deleteMemory(memoryId: string) {
    if (!this.characterId) return;
    await api.deleteCharacterMemory(this.characterId, memoryId);
    this.memories = this.memories.filter((m) => m.id !== memoryId);
  }

  async updateMemory(
    memoryId: string,
    updates: { content?: string; memoryType?: string; confidence?: number; tags?: string[] },
  ) {
    if (!this.characterId) return;
    const updated = await api.updateCharacterMemory(this.characterId, memoryId, updates);
    const idx = this.memories.findIndex((m) => m.id === memoryId);
    if (idx >= 0) {
      this.memories = [
        ...this.memories.slice(0, idx),
        updated,
        ...this.memories.slice(idx + 1),
      ];
    }
  }
}

export const characterMemoryStore = new CharacterMemoryStore();
