import type { LolVisibleStatId } from "./lolPlayerStats";

export const DEFAULT_TRAINING_FOCUS = "Scrims";
export const RECOVERY_TRAINING_FOCUS = "MentalResetRecovery";

export const TRAINING_FOCUS_IDS = [
  "Scrims",
  "VODReview",
  "IndividualCoaching",
  "ChampionPoolPractice",
  "MacroSystems",
  "MentalResetRecovery",
] as const;

export const TRAINING_FOCUS_ATTRS: Record<string, LolVisibleStatId[]> = {
  Scrims: ["teamfighting", "macro", "consistency"],
  VODReview: ["macro", "consistency", "discipline"],
  IndividualCoaching: ["mechanics", "laning", "consistency"],
  ChampionPoolPractice: ["mechanics", "championPool", "laning"],
  MacroSystems: ["macro", "shotcalling", "teamfighting"],
  MentalResetRecovery: [],
};

const LEGACY_TRAINING_FOCUS_MAP: Record<string, string> = {
  Physical: "Scrims",
  Technical: "ChampionPoolPractice",
  Tactical: "MacroSystems",
  Defending: "VODReview",
  Attacking: "IndividualCoaching",
  Recovery: "MentalResetRecovery",
  General: "Scrims",
};

export function normalizeTrainingFocus(focus?: string | null): string {
  if (!focus) {
    return DEFAULT_TRAINING_FOCUS;
  }

  return LEGACY_TRAINING_FOCUS_MAP[focus] ?? focus;
}

export function normalizeOptionalTrainingFocus(focus?: string | null): string | null {
  if (!focus) {
    return null;
  }

  return normalizeTrainingFocus(focus);
}

export function isRecoveryTrainingFocus(focus?: string | null): boolean {
  return normalizeTrainingFocus(focus) === RECOVERY_TRAINING_FOCUS;
}
