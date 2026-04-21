import type { PlayerData } from "../store/gameStore";

export type LolVisibleStatId =
  | "mechanics"
  | "laning"
  | "teamfighting"
  | "macro"
  | "consistency"
  | "shotcalling"
  | "championPool"
  | "discipline"
  | "mentalResilience";

export const LOL_VISIBLE_STAT_LABEL_KEYS: Record<LolVisibleStatId, string> = {
  mechanics: "playerProfile.lolStats.mechanics",
  laning: "playerProfile.lolStats.laning",
  teamfighting: "playerProfile.lolStats.teamfighting",
  macro: "playerProfile.lolStats.macro",
  consistency: "playerProfile.lolStats.consistency",
  shotcalling: "playerProfile.lolStats.shotcalling",
  championPool: "playerProfile.lolStats.championPool",
  discipline: "playerProfile.lolStats.discipline",
  mentalResilience: "playerProfile.lolStats.mentalResilience",
};

export const LOL_VISIBLE_STAT_GROUPS = [
  {
    labelKey: "playerProfile.lolStatGroups.gameplay",
    statIds: ["mechanics", "laning", "teamfighting"],
  },
  {
    labelKey: "playerProfile.lolStatGroups.gameIq",
    statIds: ["macro", "consistency", "shotcalling"],
  },
  {
    labelKey: "playerProfile.lolStatGroups.competitive",
    statIds: ["championPool", "discipline", "mentalResilience"],
  },
] as const;

function avg(...values: number[]): number {
  return Math.round(values.reduce((sum, value) => sum + value, 0) / values.length);
}

function clampOvr(value: number): number {
  return Math.max(1, Math.min(99, Math.round(value)));
}

export function getLolVisibleStatValue(player: PlayerData, statId: LolVisibleStatId): number {
  const a = player.attributes;

  switch (statId) {
    case "mechanics":
      return avg(a.dribbling, a.agility, a.pace, a.composure);
    case "laning":
      return avg(a.shooting, a.positioning, a.dribbling, a.composure);
    case "teamfighting":
      return avg(a.teamwork, a.stamina, a.decisions, a.composure);
    case "macro":
      return avg(a.vision, a.decisions, a.positioning, a.passing);
    case "consistency":
      return avg(a.decisions, a.vision, a.composure, a.teamwork);
    case "shotcalling":
      return avg(a.leadership, a.teamwork, a.vision, a.decisions);
    case "championPool":
      return avg(a.dribbling, a.agility, a.vision, a.passing);
    case "discipline":
      return avg(a.decisions, a.composure, a.teamwork, a.leadership);
    case "mentalResilience":
      return avg(a.composure, a.teamwork, a.leadership, a.stamina);
  }
}

export function calculateLolOvr(player: PlayerData): number {
  const mechanics = getLolVisibleStatValue(player, "mechanics");
  const laning = getLolVisibleStatValue(player, "laning");
  const teamfighting = getLolVisibleStatValue(player, "teamfighting");
  const macro = getLolVisibleStatValue(player, "macro");
  const consistency = getLolVisibleStatValue(player, "consistency");

  return clampOvr(
    mechanics * 0.28 +
      laning * 0.16 +
      teamfighting * 0.22 +
      macro * 0.22 +
      consistency * 0.12,
  );
}
