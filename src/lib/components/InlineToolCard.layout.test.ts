import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

const inlineToolCardPath = new URL("./InlineToolCard.svelte", import.meta.url);
const source = readFileSync(inlineToolCardPath, "utf8");

describe("InlineToolCard multi-question AskUserQuestion layout", () => {
  it("uses a grid layout for multi-question option groups", () => {
    expect(source).toContain('class="grid gap-1.5 sm:grid-cols-2"');
  });

  it("lets option buttons fill the grid width and wrap long provider labels", () => {
    expect(source).toContain("w-full min-w-0");
    expect(source).toContain("text-left text-xs");
    expect(source).toContain("break-words");
  });
});
