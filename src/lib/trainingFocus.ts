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

export const TRAINING_FOCUS_ATTRS: Record<string, string[]> = {
  Scrims: ["decisions", "positioning", "teamwork", "composure"],
  VODReview: ["vision", "decisions", "positioning", "composure"],
  IndividualCoaching: ["passing", "shooting", "dribbling", "composure"],
  ChampionPoolPractice: ["dribbling", "agility", "passing", "shooting"],
  MacroSystems: ["positioning", "vision", "decisions", "teamwork"],
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
