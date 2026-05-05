import { describe, expect, it } from "vitest";
import {
  isNativeAgent,
  isPermissionModeVisibleForAgent,
  nativeYoloToPermissionMode,
  permissionModeToNativeYolo,
} from "../native-permission";

describe("native permission mapping", () => {
  it("recognizes native pipe-exec agents", () => {
    expect(isNativeAgent("codex")).toBe(true);
    expect(isNativeAgent("gemini")).toBe(true);
    expect(isNativeAgent("claude")).toBe(false);
  });

  it("maps native permission selections to forced elevated policy", () => {
    expect(permissionModeToNativeYolo("bypassPermissions")).toBe(true);
    expect(permissionModeToNativeYolo("dontAsk")).toBe(true);
    expect(permissionModeToNativeYolo("default")).toBe(true);
    expect(permissionModeToNativeYolo("acceptEdits")).toBe(false);
  });

  it("maps stored native yolo state back to visible forced bypass mode", () => {
    expect(nativeYoloToPermissionMode(true)).toBe("bypassPermissions");
    expect(nativeYoloToPermissionMode(false)).toBe("bypassPermissions");
    expect(nativeYoloToPermissionMode(null)).toBe("bypassPermissions");
  });

  it("shows only forced bypass for native agents", () => {
    expect(isPermissionModeVisibleForAgent("codex", "default")).toBe(false);
    expect(isPermissionModeVisibleForAgent("codex", "bypassPermissions")).toBe(true);
    expect(isPermissionModeVisibleForAgent("codex", "acceptEdits")).toBe(false);
    expect(isPermissionModeVisibleForAgent("gemini", "plan")).toBe(false);
    expect(isPermissionModeVisibleForAgent("claude", "plan")).toBe(true);
  });
});
