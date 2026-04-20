import type { LolTacticsData } from "../store/types";

export type DraftRole = "TOP" | "JUNGLA" | "MID" | "ADC" | "SUPPORT";

export const ROLE_ORDER: DraftRole[] = ["TOP", "JUNGLA", "MID", "ADC", "SUPPORT"];

export const DEFAULT_LOL_TACTICS: LolTacticsData = {
  strong_side: "Bot",
  game_timing: "Mid",
  jungle_style: "Enabler",
  jungle_pathing: "TopToBot",
  fight_plan: "FrontToBack",
  support_roaming: "Lane",
};

export function computeRoleModifiers(tactics: LolTacticsData): Record<DraftRole, number> {
  const mod: Record<DraftRole, number> = {
    TOP: 0,
    JUNGLA: 0,
    MID: 0,
    ADC: 0,
    SUPPORT: 0,
  };

  if (tactics.strong_side === "Top") {
    mod.TOP += 2;
    mod.ADC -= 1;
  } else if (tactics.strong_side === "Mid") {
    mod.MID += 2;
    mod.TOP -= 1;
  } else {
    mod.ADC += 2;
    mod.SUPPORT += 1;
    mod.TOP -= 1;
  }

  if (tactics.jungle_pathing === "TopToBot") {
    mod.ADC += 1;
    mod.SUPPORT += 1;
    mod.TOP -= 1;
  } else {
    mod.TOP += 1;
    mod.JUNGLA += 1;
    mod.ADC -= 1;
  }

  if (tactics.jungle_style === "Ganker") {
    mod.JUNGLA += 1;
    if (tactics.strong_side === "Top") mod.TOP += 1;
    if (tactics.strong_side === "Mid") mod.MID += 1;
    if (tactics.strong_side === "Bot") mod.ADC += 1;
  } else if (tactics.jungle_style === "Invader") {
    mod.JUNGLA += 1;
    mod.SUPPORT += 1;
  } else if (tactics.jungle_style === "Farmer") {
    mod.JUNGLA += 2;
    mod.TOP -= 0.5;
  } else {
    mod.SUPPORT += 1;
    mod.ADC += 1;
  }

  if (tactics.game_timing === "Early") {
    mod.JUNGLA += 1;
    mod.MID += 1;
  } else if (tactics.game_timing === "Late") {
    mod.ADC += 1;
    mod.SUPPORT += 1;
  } else {
    mod.MID += 0.5;
  }

  if (tactics.fight_plan === "FrontToBack") {
    mod.TOP += 1;
    mod.ADC += 1;
    mod.SUPPORT += 1;
    mod.MID -= 0.5;
  } else if (tactics.fight_plan === "Pick") {
    mod.MID += 1;
    mod.JUNGLA += 1;
    mod.SUPPORT += 0.5;
    mod.TOP -= 0.5;
  } else if (tactics.fight_plan === "Dive") {
    mod.TOP += 1;
    mod.JUNGLA += 1;
    mod.MID += 1;
    mod.ADC -= 1;
  } else {
    mod.MID += 1;
    mod.ADC += 1;
    mod.SUPPORT += 0.5;
    mod.TOP -= 0.5;
  }

  if (tactics.support_roaming === "RoamMid") {
    mod.SUPPORT -= 0.75;
    mod.MID += 1.5;
    mod.TOP -= 0.25;
  } else if (tactics.support_roaming === "RoamTop") {
    mod.SUPPORT -= 1;
    mod.TOP += 1.5;
    mod.ADC -= 0.5;
  } else {
    mod.SUPPORT += 0.75;
    mod.ADC += 0.75;
  }

  return mod;
}

export function computeCoherenceBreakdown(tactics: LolTacticsData): Array<{ label: string; delta: number }> {
  const checks: Array<{ label: string; delta: number }> = [];

  if (tactics.strong_side === "Bot") {
    checks.push({
      label: "Ruta de jungla hacia bot side",
      delta: tactics.jungle_pathing === "TopToBot" ? 0.5 : -0.5,
    });
  }
  if (tactics.strong_side === "Top") {
    checks.push({
      label: "Ruta de jungla hacia top side",
      delta: tactics.jungle_pathing === "BotToTop" ? 0.5 : -0.5,
    });
  }

  checks.push({
    label: "Timing vs estilo de jungla",
    delta:
      tactics.game_timing === "Early"
        ? tactics.jungle_style === "Ganker" || tactics.jungle_style === "Invader"
          ? 0.5
          : -0.5
        : tactics.game_timing === "Late"
          ? tactics.jungle_style === "Farmer" || tactics.jungle_style === "Enabler"
            ? 0.5
            : -0.5
          : 0.25,
  });

  checks.push({
    label: "Plan de pelea vs ejecución",
    delta:
      tactics.fight_plan === "Pick"
        ? tactics.jungle_style === "Ganker" || tactics.jungle_style === "Invader"
          ? 0.5
          : -0.5
        : tactics.fight_plan === "FrontToBack"
          ? tactics.strong_side === "Bot"
            ? 0.5
            : 0.25
          : 0.25,
  });

  checks.push({
    label: "Roaming support vs lado fuerte",
    delta:
      tactics.support_roaming === "Lane"
        ? tactics.strong_side === "Bot"
          ? 0.5
          : 0
        : tactics.support_roaming === "RoamMid"
          ? tactics.strong_side === "Mid"
            ? 0.5
            : 0.1
          : tactics.strong_side === "Top"
            ? 0.5
            : -0.25,
  });

  return checks;
}
