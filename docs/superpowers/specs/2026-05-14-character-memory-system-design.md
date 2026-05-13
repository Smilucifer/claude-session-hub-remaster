# Character Memory System — Design Spec

**Date:** 2026-05-14
**Status:** Draft — awaiting review
**Phase:** Post-Phase-10

## Overview

Redesign the AiCharacter system from lightweight persona templates into **heavyweight persistent characters**, each with an independent memory system backed by LanceDB vector search and a graphology knowledge graph. Characters accumulate knowledge, experiences, preferences, and relationships across group chat sessions, and retrieve relevant memories to inject as system prompt context during orchestration.

## Motivation

Current `AiCharacter` is a flat template (label, role_type, role_instruction, provider, model). There is no persistent memory — role prompts are static. Characters "forget" everything after each session. The goal is to give each character a persistent "brain" that:
- Accumulates knowledge from conversations
- Supports semantic search via embeddings
- Models relationships between memories as a knowledge graph
- Detects knowledge gaps and clusters
- Injects relevant memories into group chat context

## Tech Stack

| Layer | Technology | Rationale |
|---|---|---|
| Vector DB | **LanceDB** (Rust SDK, embedded) | Already validated in trading-review-wiki Tauri v2 app; file-based, zero ops |
| Embeddings | **External API** (user-configured, OpenAI-compatible) | No local model dependency; supports Ollama/llama.cpp/any compatible endpoint |
| Knowledge Graph | **graphology** + **sigma.js** (pure TypeScript, frontend) | Proven in trading-review-wiki; Louvain community detection, ForceAtlas2 layout |
| Memory Formation | **LLM CoT ingestion pipeline** | Pattern borrowed from trading-review-wiki ingest; SHA256 dedup |
| Storage | **File-based JSON + JSONL + LanceDB** | Follows existing project patterns; no database server |

## Data Models

### AiCharacter (upgraded)

```typescript
interface AiCharacter {
  // --- existing fields, unchanged ---
  id: string;
  label: string;
  role_type: string;
  role_instruction?: string;
  default_provider: string;
  default_model?: string;
  created_at: string;
  updated_at: string;

  // --- new fields ---
  avatar_path?: string;          // local path to uploaded avatar image (jpg/png)
  personality?: string;          // free-text personality/character description
  expertise?: string[];          // domain expertise tags
  memory_config?: {
    auto_learn: boolean;         // auto-extract memories from conversations (default: true)
    retention_days?: number;     // memory retention period (null/undefined = permanent)
  };
}
```

### EmbeddingConfig (in UserSettings)

```typescript
interface EmbeddingConfig {
  enabled: boolean;
  endpoint: string;       // OpenAI-compatible embedding API endpoint
  api_key?: string;       // independent API key (falls back to provider key if empty)
  model: string;          // e.g. "text-embedding-3-small", "bge-large-zh"
}
```

### MemoryNode

```typescript
interface MemoryNode {
  id: string;
  character_id: string;
  content: string;              // memory text
  type: "fact" | "experience" | "preference" | "rule" | "relationship";
  confidence: number;           // 0.0–1.0
  source: {
    kind: "chat" | "manual" | "inference";
    run_id?: string;
    group_chat_id?: string;
  };
  tags: string[];
  created_at: string;
  updated_at: string;
}
```

### MemoryEdge

```typescript
interface MemoryEdge {
  id: string;
  source_id: string;
  target_id: string;
  relation: "supports" | "contradicts" | "extends" | "related" | "causes";
  weight: number;  // 0.0–1.0
}
```

## Storage Layout

```
~/.claw-go/characters/{character_id}/
├── character.json          # AiCharacter metadata (migrated OUT of settings.json)
├── avatar.png              # uploaded avatar image
├── lancedb/                # LanceDB vector index
├── memory-graph.json       # knowledge graph (nodes + edges, graphology-compatible)
└── memory-log.jsonl        # append-only memory event log (replayable)
```

- Characters are no longer stored inline in `settings.json`; they get dedicated directories
- `settings.json` retains a minimal `ai_characters` reference list (`[{id, label}]`) for backward compatibility
- LanceDB managed by Rust via `vectorstore.rs` commands (reuse trading-review-wiki pattern)
- Knowledge graph file managed by frontend graphology; backend provides file read/write

## Memory Formation Pipeline

### Source 1: Auto-extraction from group chat

1. After a group chat turn completes, collect the last N public turns
2. Send to LLM with Chain-of-Thought prompt: "Extract knowledge valuable to this character: facts, experiences, preferences, rules, relationships"
3. LLM returns structured MemoryNode list (JSON)
4. SHA256 dedup — skip if semantically equivalent memory already exists
5. Append to `memory-log.jsonl`
6. Incrementally update `memory-graph.json` (add nodes, compute edges via 4-signal relevance)
7. LanceDB upsert for each new node
8. User can review/edit/delete in the memory panel

### Source 2: Manual creation

1. User opens memory panel, clicks "手动添加"
2. Fills form: content, type, confidence, tags, optional relation to existing memory
3. Directly writes to `memory-log.jsonl` + updates graph + LanceDB upsert

## Memory Retrieval (Hybrid Search)

When a character is invoked in group chat (via `@mention`):

1. **Vector search** — embed the user's query + recent context, search LanceDB for top-K
2. **Graph expansion** — from matched nodes, traverse 1–2 hops to pull in connected memories
3. **Keyword scoring** — BM25-style boost for exact term matches
4. **Merge & rank** — weighted combination → top-N memories
5. **Inject** — format memories into system prompt alongside role instruction

Configurable parameters:
- `max_retrieval_count` (default: 5)
- `relevance_threshold` (default: 0.6)
- `graph_expansion_hops` (default: 1)

## Knowledge Graph Features

Borrowed directly from trading-review-wiki patterns:

### 4-Signal Relevance Model (graph-relevance.ts)

| Signal | Weight | Description |
|---|---|---|
| Direct link | 3.0 | Page A links to B or vice versa |
| Source overlap | 4.0 | Number of shared source sessions |
| Common neighbors (Adamic-Adar) | 1.5 | Normalized shared neighbor count |
| Type affinity | 1.0 | Predefined type affinity matrix |

### Community Detection (Louvain)

- Uses `graphology-communities-louvain` to auto-detect knowledge clusters
- Displays community cohesion scores and member nodes

### Knowledge Gap Detection

- **Isolated nodes** — degree ≤ 1, suggest connections
- **Sparse communities** — cohesion < 0.15 with ≥ 3 nodes
- **Bridge nodes** — connected to ≥ 3 communities (high-value hubs)

### Surprise Connections

- Cross-community edges, cross-type edges, periphery-to-center edges
- Scored and surfaced for review

## New Tauri Commands

```rust
// Character memory CRUD
list_character_memories(character_id, filter?) -> Vec<MemoryNode>
get_character_memory(character_id, memory_id) -> MemoryNode
create_character_memory(character_id, content, type, confidence, tags, relations) -> MemoryNode
update_character_memory(character_id, memory_id, updates) -> MemoryNode
delete_character_memory(character_id, memory_id) -> ()

// Hybrid search
search_character_memories(character_id, query, top_k, threshold, graph_hops) -> Vec<MemoryNode>

// Knowledge graph
get_memory_graph(character_id) -> GraphData
get_memory_communities(character_id) -> Vec<CommunityInfo>
get_knowledge_gaps(character_id) -> Vec<GapInfo>

// Auto-extraction (async, triggered after group chat turn)
extract_memories_from_turns(character_id, turns: Vec<TurnData>) -> Vec<MemoryNode>

// Vector store (shared with trading-review-wiki pattern)
vector_upsert(character_id, page_id, vector: Vec<f32>)
vector_search(character_id, query_vector: Vec<f32>, top_k: u32) -> Vec<SearchResult>
vector_delete(character_id, page_id)

// Character avatar
upload_character_avatar(character_id, file_path: String) -> String  // returns stored path

// Embedding config
get_embedding_config() -> EmbeddingConfig
update_embedding_config(config) -> EmbeddingConfig
test_embedding_connection() -> TestResult
```

## UI Pages

### 1. Character Editor Upgrade (`/settings/characters`)
- Avatar upload (jpg/png) with preview, hover-to-replace
- New fields: personality, expertise tags, memory config (auto_learn toggle, retention_days)
- "Manage Memory" button per character → opens memory panel

### 2. Character Memory Panel (new)
- **Tabs:** 全部记忆 / 知识图谱 / 知识缺口 / 社区 Community
- **Left pane:** Interactive knowledge graph (sigma.js + ForceAtlas2, zoom/pan/click)
- **Right pane:** Memory list with type badges, confidence bars, source traces
- Search bar with hybrid retrieval
- "手动添加" button → modal form
- Footer: export/clear memory, navigation between characters

### 3. Embedding Config (`/settings` → 角色记忆 section)
- Enable/disable toggle (preserves data, stops updates)
- API endpoint, API key (optional, fallback to provider), model name
- Quick presets (OpenAI, Ollama, BGE, DeepSeek)
- "Test Connection" button with latency/dimension display

### 4. Group Chat Memory Injection (existing GroupChatLayout enhancement)
- During orchestration, before sending prompt to a character:
  - Build embedding from user message + context
  - Retrieve top-K relevant memories
  - Inject formatted memories into system prompt
- Injection config (max count, threshold, graph hops) exposed in group chat settings

## Migration Path

1. Existing `AiCharacter[]` in `settings.json` stays; new fields are optional with defaults
2. On first access to a character's memory, create the `characters/{id}/` directory lazily
3. `character_id` linkage in `GroupChatParticipant` — change from label-based matching to ID-based lookup. The new `character_id` field in `GroupChatParticipant` (already declared in TypeScript but always `""`) becomes the canonical link
4. Backward compatible: if `character_id` is empty, fall back to label matching

## Implementation Phases

1. **Foundation** — LanceDB integration, vectorstore commands, EmbeddingConfig model + settings UI
2. **Storage** — Character directory structure, MemoryNode/Edge CRUD, memory-log.jsonl
3. **Knowledge Graph** — graphology integration, 4-signal relevance, community detection, gap detection
4. **Memory Panel UI** — graph visualization, memory list, manual add, tabs
5. **Auto-Extraction** — LLM ingestion pipeline, SHA256 dedup, async trigger from group chat
6. **Injection** — hybrid search, prompt formatting, injection config
7. **Character Editor Upgrade** — avatar upload, new fields, migration from settings.json
