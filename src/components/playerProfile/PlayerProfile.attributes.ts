import type { PlayerData } from "../../store/gameStore";
import {
  LOL_VISIBLE_STAT_GROUPS,
  LOL_VISIBLE_STAT_LABEL_KEYS,
  getLolVisibleStatValue,
} from "../../lib/lolPlayerStats";

type TranslateFn = (key: string) => string;

export interface PlayerAttributeEntry {
  name: string;
  value: number;
}

export interface PlayerAttributeGroup {
  label: string;
  attrs: PlayerAttributeEntry[];
  average: number;
}

function createAttributeGroup(
  label: string,
  attrs: PlayerAttributeEntry[],
): PlayerAttributeGroup {
  return {
    label,
    attrs,
    average: Math.round(
      attrs.reduce((sum, attribute) => sum + attribute.value, 0) / attrs.length,
    ),
  };
}

export function buildPlayerAttributeGroups(
  player: PlayerData,
  translate: TranslateFn,
): PlayerAttributeGroup[] {
  return LOL_VISIBLE_STAT_GROUPS.map((group) =>
    createAttributeGroup(
      translate(group.labelKey),
      group.statIds.map((statId) => ({
        name: translate(LOL_VISIBLE_STAT_LABEL_KEYS[statId]),
        value: getLolVisibleStatValue(player, statId),
      })),
    ),
  );
}
