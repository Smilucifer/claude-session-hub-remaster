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

  it("maps no-review modes to native yolo", () => {
    expect(permissionModeToNativeYolo("bypassPermissions")).toBe(true);
    expect(permissionModeToNativeYolo("dontAsk")).toBe(true);
    expect(permissionModeToNativeYolo("default")).toBe(false);
    expect(permissionModeToNativeYolo("acceptEdits")).toBe(false);
  });

  it("maps native yolo state back to the prompt permission mode", () => {
    expect(nativeYoloToPermissionMode(true)).toBe("bypassPermissions");
    expect(nativeYoloToPermissionMode(false)).toBe("default");
    expect(nativeYoloToPermissionMode(null)).toBe("default");
  });

  it("hides unsupported native permission modes", () => {
    expect(isPermissionModeVisibleForAgent("codex", "default")).toBe(true);
    expect(isPermissionModeVisibleForAgent("codex", "bypassPermissions")).toBe(true);
    expect(isPermissionModeVisibleForAgent("codex", "acceptEdits")).toBe(false);
    expect(isPermissionModeVisibleForAgent("gemini", "plan")).toBe(false);
    expect(isPermissionModeVisibleForAgent("claude", "plan")).toBe(true);
  });
});
