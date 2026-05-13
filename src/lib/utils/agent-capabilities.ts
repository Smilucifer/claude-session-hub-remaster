import type { AgentCapabilities, AgentKind } from "$lib/types";

export function capabilitiesForAgent(agent: string): AgentCapabilities {
  const kind = normalizeAgentKind(agent);
  if (kind === "claude") {
    return {
      kind,
      stream_session: true,
      pipe_exec: true,
      interactive_pty: false,
      resume: "session_id",
      prompt_injection: "system_prompt",
      mcp_config: true,
      context_usage: true,
      permission_protocol: true,
    };
  }
  if (kind === "codex") {
    return {
      kind,
      stream_session: false,
      pipe_exec: true,
      interactive_pty: false,
      resume: "latest",
      prompt_injection: null,
      mcp_config: false,
      context_usage: false,
      permission_protocol: false,
    };
  }
  return {
    kind,
    stream_session: false,
    pipe_exec: false,
    interactive_pty: false,
    resume: "none",
    prompt_injection: null,
    mcp_config: false,
    context_usage: false,
    permission_protocol: false,
  };
}

function normalizeAgentKind(agent: string): AgentKind {
  const normalized = agent.trim().toLowerCase();
  if (normalized === "claude" || normalized === "codex") {
    return normalized;
  }
  return "unknown";
}
