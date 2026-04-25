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

function clampOvr(value: number): number {
  return Math.max(1, Math.min(99, Math.round(value)));
}

export function getLolVisibleStatValue(player: PlayerData, statId: LolVisibleStatId): number {
  const a = player.attributes;

  switch (statId) {
    case "mechanics":
      return a.dribbling;
    case "laning":
      return a.shooting;
    case "teamfighting":
      return a.teamwork;
    case "macro":
      return a.vision;
    case "consistency":
      return a.decisions;
    case "shotcalling":
      return a.leadership;
    case "championPool":
      return a.agility;
    case "discipline":
      return a.composure;
    case "mentalResilience":
      return a.stamina;
  }
}

export function calculateLolOvr(player: PlayerData): number {
  const mechanics = getLolVisibleStatValue(player, "mechanics");
  const laning = getLolVisibleStatValue(player, "laning");
  const teamfighting = getLolVisibleStatValue(player, "teamfighting");
  const macro = getLolVisibleStatValue(player, "macro");
  const consistency = getLolVisibleStatValue(player, "consistency");
  const shotcalling = getLolVisibleStatValue(player, "shotcalling");
  const championPool = getLolVisibleStatValue(player, "championPool");
  const discipline = getLolVisibleStatValue(player, "discipline");
  const mentalResilience = getLolVisibleStatValue(player, "mentalResilience");

  return clampOvr(
    (mechanics +
      laning +
      teamfighting +
      macro +
      consistency +
      shotcalling +
      championPool +
      discipline +
      mentalResilience) / 9,
  );
}
