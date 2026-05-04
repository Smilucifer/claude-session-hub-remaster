import { describe, expect, it } from "vitest";
import { routeMatches } from "./nav-route";

describe("routeMatches", () => {
  it("matches exact routes and child routes", () => {
    expect(routeMatches("/memory", "/memory")).toBe(true);
    expect(routeMatches("/memory/files", "/memory")).toBe(true);
  });

  it("does not match similarly named sibling routes", () => {
    expect(routeMatches("/memory", "/memo")).toBe(false);
    expect(routeMatches("/memo", "/memory")).toBe(false);
  });
});
