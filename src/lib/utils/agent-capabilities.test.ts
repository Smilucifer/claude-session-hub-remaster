import { describe, expect, it } from "vitest";
import { canUseRoomActor, canUseRoomActorRun, capabilitiesForAgent } from "./agent-capabilities";

describe("agent capability matrix", () => {
  it("maps Claude and Codex explicitly", () => {
    expect(capabilitiesForAgent("claude")).toMatchObject({
      kind: "claude",
      stream_session: true,
      pipe_exec: true,
      resume: "session_id",
      prompt_injection: "system_prompt",
      mcp_config: true,
      context_usage: true,
      permission_protocol: true,
    });

    expect(capabilitiesForAgent("codex")).toMatchObject({
      kind: "codex",
      stream_session: false,
      pipe_exec: true,
      resume: "latest",
      prompt_injection: null,
    });
  });

  it("keeps Gemini unsupported until a Room adapter exists", () => {
    expect(capabilitiesForAgent("gemini")).toMatchObject({
      kind: "gemini",
      stream_session: false,
      pipe_exec: false,
      resume: "none",
    });
    expect(canUseRoomActor("gemini")).toBe(false);
  });

  it("requires a run-level session actor path for Room actors", () => {
    expect(canUseRoomActorRun({ agent: "claude", execution_path: "session_actor" })).toBe(true);
    expect(canUseRoomActorRun({ agent: "claude", execution_path: "pipe_exec" })).toBe(false);
    expect(canUseRoomActorRun({ agent: "codex", execution_path: "pipe_exec" })).toBe(false);
  });
});
