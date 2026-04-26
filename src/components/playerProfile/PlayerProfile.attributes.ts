import type { PlayerData, ScoutReportData } from "../../store/gameStore";
import {
  LOL_VISIBLE_STAT_GROUPS,
  LOL_VISIBLE_STAT_LABEL_KEYS,
  getLolVisibleStatValue,
} from "../../lib/lolPlayerStats";

type TranslateFn = (key: string) => string;

export interface PlayerAttributeEntry {
  name: string;
  value: number | null;
}

export interface PlayerAttributeGroup {
  label: string;
  attrs: PlayerAttributeEntry[];
  average: number | null;
}

function createAttributeGroup(
  label: string,
  attrs: PlayerAttributeEntry[],
): PlayerAttributeGroup {
  return {
    label,
    attrs,
    average: (() => {
      const revealed = attrs.filter((attribute) => attribute.value !== null);
      if (revealed.length === 0) return null;
      return Math.round(
        revealed.reduce((sum, attribute) => sum + (attribute.value ?? 0), 0) / revealed.length,
      );
    })(),
  };
}

const SCOUT_REPORT_STAT_FIELD_BY_VISIBLE_STAT = {
  mechanics: "mechanics",
  laning: "laning",
  teamfighting: "teamfighting",
  macro: "macro",
  consistency: null,
  shotcalling: null,
  championPool: "champion_pool",
  discipline: "discipline",
  mentalResilience: null,
} as const satisfies Record<string, keyof ScoutReportData | null>;

const LEGACY_SCOUT_REPORT_STAT_FIELD_BY_VISIBLE_STAT = {
  mechanics: "pace",
  laning: "shooting",
  teamfighting: "passing",
  macro: "dribbling",
  consistency: null,
  shotcalling: null,
  championPool: "defending",
  discipline: "physical",
  mentalResilience: null,
} as const satisfies Record<string, keyof ScoutReportData | null>;

function getScoutedVisibleStatValue(
  report: ScoutReportData,
  statId: keyof typeof SCOUT_REPORT_STAT_FIELD_BY_VISIBLE_STAT,
): number | null {
  const field = SCOUT_REPORT_STAT_FIELD_BY_VISIBLE_STAT[statId];
  const legacyField = LEGACY_SCOUT_REPORT_STAT_FIELD_BY_VISIBLE_STAT[statId];
  const value = field ? report[field] : null;
  const legacyValue = legacyField ? report[legacyField] : null;

  return typeof value === "number"
    ? value
    : typeof legacyValue === "number"
      ? legacyValue
      : null;
}

export function buildPlayerAttributeGroups(
  player: PlayerData,
  translate: TranslateFn,
  scoutReport?: ScoutReportData | null,
): PlayerAttributeGroup[] {
  return LOL_VISIBLE_STAT_GROUPS.map((group) =>
    createAttributeGroup(
      translate(group.labelKey),
      group.statIds.map((statId) => ({
        name: translate(LOL_VISIBLE_STAT_LABEL_KEYS[statId]),
        value: scoutReport === undefined
          ? getLolVisibleStatValue(player, statId)
          : scoutReport
            ? getScoutedVisibleStatValue(scoutReport, statId)
            : null,
      })),
    ),
  );
}
