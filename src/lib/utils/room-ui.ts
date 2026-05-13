import type { RoomTurnMode } from "$lib/types";
import { getPhase7Provider, providerIdForRun } from "$lib/utils/provider-catalog";

export type RoomPlaceholderKey = "room_roundtablePlaceholder";

export function canSendRoomMessage(
  participantCount: number,
  message: string,
): boolean {
  if (!message.trim()) return false;
  return participantCount > 0;
}

export function roomMessagePlaceholderKey(): RoomPlaceholderKey {
  return "room_roundtablePlaceholder";
}

export function roomParticipantBadge(participantCount: number): string {
  return String(participantCount);
}

export function roomParticipantProviderLabel(agent: string, platformId?: string | null): string {
  return getPhase7Provider(providerIdForRun(agent, platformId)).label;
}

export function roomParticipantMetaLabel(
  agent: string,
  platformId?: string | null,
  model?: string | null,
): string {
  const providerLabel = roomParticipantProviderLabel(agent, platformId);
  const cleanModel = model?.trim();
  return cleanModel ? `${providerLabel} · ${cleanModel}` : providerLabel;
}

const TURN_MODE_LABEL_KEYS: Record<RoomTurnMode, string> = {
  fanout: "room_turnFanout",
  debate: "room_turnDebate",
  summary: "room_turnSummary",
  private: "room_turnPrivate",
  singletarget: "room_turnSingleTarget",
};

/** Returns the i18n key for a turn mode label. Caller should pass to `t()`. */
export function roomTurnModeKey(mode: RoomTurnMode): string {
  return TURN_MODE_LABEL_KEYS[mode] ?? mode;
}
