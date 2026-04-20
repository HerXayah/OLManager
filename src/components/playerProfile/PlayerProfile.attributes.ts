import type { PlayerData } from "../../store/gameStore";

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

function avg(...values: number[]): number {
  return Math.round(values.reduce((sum, value) => sum + value, 0) / values.length);
}

export function buildPlayerAttributeGroups(
  player: PlayerData,
  _translate: TranslateFn,
): PlayerAttributeGroup[] {
  const a = player.attributes;

  const mechanics = avg(a.dribbling, a.agility, a.pace, a.composure);
  const laning = avg(a.shooting, a.positioning, a.dribbling, a.agility);
  const teamfight = avg(a.teamwork, a.stamina, a.strength, a.composure);
  const macro = avg(a.vision, a.decisions, a.positioning, a.passing);
  const consistency = avg(a.decisions, a.vision, a.positioning, a.composure);
  const shotcalling = avg(a.leadership, a.teamwork, a.vision, a.decisions);
  const versatility = avg(a.passing, a.defending, a.tackling, a.vision);
  const discipline = avg(a.decisions, a.composure, a.teamwork, a.aggression);

  return [
    createAttributeGroup("Gameplay", [
      { name: "Mecánicas", value: mechanics },
      { name: "Laning", value: laning },
      { name: "Teamfight", value: teamfight },
    ]),
    createAttributeGroup("Game IQ", [
      { name: "Macro", value: macro },
      { name: "Consistencia", value: consistency },
      { name: "Shotcalling", value: shotcalling },
    ]),
    createAttributeGroup("Competitivo", [
      { name: "Versatilidad", value: versatility },
      { name: "Disciplina", value: discipline },
      { name: "Mentalidad", value: avg(a.composure, a.teamwork, a.leadership) },
    ]),
  ];
}
