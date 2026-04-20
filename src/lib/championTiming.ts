import championTimings from "../../data/lec/draft/champion-timings.json";

export type ChampionTiming = "Early" | "Mid" | "Late" | "Unknown";
export type TeamTimingPreference = "Early" | "Mid" | "Late";

const TIMING_BY_CHAMPION =
  (championTimings as { data?: Record<string, ChampionTiming> }).data ?? {};

const FIT_MATRIX: Record<TeamTimingPreference, Record<ChampionTiming, number>> = {
  Early: { Early: 1, Mid: 0.25, Late: -1, Unknown: 0 },
  Mid: { Early: 0.2, Mid: 1, Late: 0.2, Unknown: 0 },
  Late: { Early: -1, Mid: 0.25, Late: 1, Unknown: 0 },
};

export function getChampionTiming(championId: string | null | undefined): ChampionTiming {
  if (!championId) return "Unknown";
  return TIMING_BY_CHAMPION[championId] ?? "Unknown";
}

export function computeTeamTimingFit(params: {
  championIds: Array<string | null | undefined>;
  preference: TeamTimingPreference;
}): number {
  const { championIds, preference } = params;
  const mapped = championIds
    .map((id) => getChampionTiming(id))
    .filter((timing) => timing !== "Unknown");

  if (mapped.length === 0) return 0;

  const sum = mapped.reduce((acc, timing) => acc + FIT_MATRIX[preference][timing], 0);
  return sum / mapped.length;
}
