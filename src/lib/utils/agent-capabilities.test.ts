import { describe, expect, it } from "vitest";
import { capabilitiesForAgent } from "./agent-capabilities";

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
});
