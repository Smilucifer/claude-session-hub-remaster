<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type { MemoryGraphData } from "$lib/types";

  let {
    graph,
    width = 600,
    height = 400,
  }: {
    graph: MemoryGraphData;
    width?: number;
    height?: number;
  } = $props();

  let container: HTMLDivElement;
  let renderer: { kill(): void } | null = null;

  const typeColors: Record<string, string> = {
    fact: "#60a5fa",
    experience: "#34d399",
    preference: "#a78bfa",
    rule: "#fbbf24",
    relationship: "#f472b6",
  };

  onMount(async () => {
    if (!container || graph.nodes.length === 0) return;

    const [{ default: Graph }, { default: Sigma }] = await Promise.all([
      import("graphology"),
      import("sigma"),
    ]);

    const g = new Graph();

    for (const node of graph.nodes) {
      g.addNode(node.id, {
        label: node.content.slice(0, 30),
        size: 6 + node.confidence * 0.04,
        color: typeColors[node.type] || "#888",
        x: Math.random() * width,
        y: Math.random() * height,
      });
    }

    for (const edge of graph.edges) {
      if (g.hasNode(edge.source_id) && g.hasNode(edge.target_id)) {
        try {
          g.addEdge(edge.source_id, edge.target_id, {
            size: edge.weight * 2,
            color: "rgba(255,255,255,0.08)",
          });
        } catch {
          // duplicate edge
        }
      }
    }

    // Apply ForceAtlas2 layout
    try {
      const { default: forceAtlas2 } = await import("graphology-layout-forceatlas2");
      const positions = forceAtlas2(g, {
        iterations: 100,
        settings: {
          gravity: 1,
          scalingRatio: 10,
          barnesHutOptimize: true,
        },
      });
      for (const [id, pos] of Object.entries(positions)) {
        if (g.hasNode(id)) {
          g.setNodeAttribute(id, "x", pos.x);
          g.setNodeAttribute(id, "y", pos.y);
        }
      }
    } catch {
      // layout failed, use random positions
    }

    renderer = new Sigma(g, container, {
      renderEdgeLabels: false,
      labelFont: "Inter, sans-serif",
      labelSize: 10,
      labelColor: { color: "#ccc" },
      labelRenderedSizeThreshold: 8,
      minCameraRatio: 0.1,
      maxCameraRatio: 10,
    });
  });

  onDestroy(() => {
    renderer?.kill();
  });
</script>

<div
  bind:this={container}
  style="width: {width}px; height: {height}px;"
  class="rounded-lg border border-[#1e1e2e] bg-[#0a0a0f]"
></div>
