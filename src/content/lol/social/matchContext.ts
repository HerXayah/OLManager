import type { DraftMatchResult } from "../../../components/match/draftResultSimulator";

type Side = "blue" | "red";
type Result = "win" | "loss";
type Timing = "Early" | "Mid" | "Late";

export interface MatchContext {
  facts: Record<string, string | number | boolean>;
  tags: string[];
}

export interface CompatiblePlayerResult {
  side: Side;
  playerId?: string;
  player_id?: string;
  playerName?: string;
  player_name?: string;
  role?: string;
  kills?: number;
  deaths?: number;
  assists?: number;
  rating?: number;
}

export interface CompatibleMatchSummary {
  winnerSide?: Side;
  winner_side?: Side;
  durationMinutes?: number;
  duration_minutes?: number;
  blueKills?: number;
  blue_kills?: number;
  redKills?: number;
  red_kills?: number;
  mvp?: CompatiblePlayerResult;
  playerResults?: CompatiblePlayerResult[];
  player_results?: CompatiblePlayerResult[];
  goldDiffTimeline?: Array<{ minute: number; diff: number }>;
  gold_diff_timeline?: Array<{ minute: number; diff: number }>;
  objectives?: DraftMatchResult["objectives"];
}

export interface ExtractMatchContextParams {
  match: DraftMatchResult | CompatibleMatchSummary;
  userSide: Side;
  leagueId?: string;
  rivalry?: boolean;
  streak?: { type: Result; count: number };
  draft?: { strongSide?: "Top" | "Mid" | "Bot"; timing?: Timing };
}

const OBJECTIVE_WEIGHTS = {
  voidgrubs: 0.5,
  dragons: 1,
  dragonSoul: 2,
  elderDragons: 2,
  heralds: 1,
  barons: 2,
  towers: 0.75,
  inhibitors: 1.5,
} as const;

function addTag(tags: string[], tag: string): void {
  if (!tags.includes(tag)) {
    tags.push(tag);
  }
}

function otherSide(side: Side): Side {
  return side === "blue" ? "red" : "blue";
}

function winnerSide(match: DraftMatchResult | CompatibleMatchSummary): Side {
  return ("winnerSide" in match && match.winnerSide) ||
    ("winner_side" in match && match.winner_side) ||
    "blue";
}

function numberField(
  match: DraftMatchResult | CompatibleMatchSummary,
  camel: keyof CompatibleMatchSummary,
  snake: keyof CompatibleMatchSummary,
): number {
  const value = match[camel] ?? match[snake];
  return typeof value === "number" ? value : 0;
}

function players(match: DraftMatchResult | CompatibleMatchSummary): CompatiblePlayerResult[] {
  return (
    ("playerResults" in match && match.playerResults) ||
    ("player_results" in match && match.player_results) ||
    []
  );
}

function timeline(match: DraftMatchResult | CompatibleMatchSummary): Array<{ minute: number; diff: number }> {
  return (
    ("goldDiffTimeline" in match && match.goldDiffTimeline) ||
    ("gold_diff_timeline" in match && match.gold_diff_timeline) ||
    []
  );
}

function playerId(player: CompatiblePlayerResult | undefined): string | undefined {
  return player?.playerId ?? player?.player_id;
}

function normalizeRole(role: string | undefined): string | undefined {
  if (!role) return undefined;
  return role.toLowerCase();
}

function objectiveScore(objectives: DraftMatchResult["objectives"] | undefined, side: Side): number {
  const sideObjectives = objectives?.[side];
  if (!sideObjectives) return 0;

  return (
    sideObjectives.voidgrubs * OBJECTIVE_WEIGHTS.voidgrubs +
    sideObjectives.dragons * OBJECTIVE_WEIGHTS.dragons +
    Number(sideObjectives.dragonSoul) * OBJECTIVE_WEIGHTS.dragonSoul +
    sideObjectives.elderDragons * OBJECTIVE_WEIGHTS.elderDragons +
    sideObjectives.heralds * OBJECTIVE_WEIGHTS.heralds +
    sideObjectives.barons * OBJECTIVE_WEIGHTS.barons +
    sideObjectives.towers * OBJECTIVE_WEIGHTS.towers +
    sideObjectives.inhibitors * OBJECTIVE_WEIGHTS.inhibitors
  );
}

function comebackGoldDeficit(params: {
  result: Result;
  userSide: Side;
  goldDiffTimeline: Array<{ minute: number; diff: number }>;
}): number {
  if (params.result !== "win") return 0;

  const userPerspectiveDiffs = params.goldDiffTimeline.map(({ diff }) =>
    params.userSide === "blue" ? diff : -diff,
  );
  const lowestDiff = Math.min(0, ...userPerspectiveDiffs);
  return Math.abs(lowestDiff);
}

function worstUserPlayer(userPlayers: CompatiblePlayerResult[]): CompatiblePlayerResult | undefined {
  return [...userPlayers].sort((a, b) => {
    const aDeaths = a.deaths ?? 0;
    const bDeaths = b.deaths ?? 0;
    const aRating = a.rating ?? 10;
    const bRating = b.rating ?? 10;
    return bDeaths - aDeaths || aRating - bRating;
  })[0];
}

function botlaneUnderperformed(userPlayers: CompatiblePlayerResult[]): boolean {
  const botlane = userPlayers.filter((player) => player.role === "ADC" || player.role === "SUPPORT");
  return botlane.length > 0 && botlane.every((player) => (player.rating ?? 10) < 5 || (player.deaths ?? 0) >= 5);
}

export function extractMatchContext(params: ExtractMatchContextParams): MatchContext {
  const { match, userSide } = params;
  const enemySide = otherSide(userSide);
  const result: Result = winnerSide(match) === userSide ? "win" : "loss";
  const userKills = numberField(match, `${userSide}Kills` as keyof CompatibleMatchSummary, `${userSide}_kills` as keyof CompatibleMatchSummary);
  const enemyKills = numberField(match, `${enemySide}Kills` as keyof CompatibleMatchSummary, `${enemySide}_kills` as keyof CompatibleMatchSummary);
  const killDiff = userKills - enemyKills;
  const objectiveDiff = Math.round(
    (objectiveScore(match.objectives, userSide) - objectiveScore(match.objectives, enemySide)) * 100,
  ) / 100;
  const matchPlayers = players(match);
  const userPlayers = matchPlayers.filter((player) => player.side === userSide);
  const tags: string[] = [];
  const facts: MatchContext["facts"] = {
    result,
    userSide,
    killDiff,
    objectiveDiff,
  };

  if (params.leagueId) facts.leagueId = params.leagueId;

  addTag(tags, result);

  if (result === "win" && (killDiff >= 8 || objectiveDiff >= 6)) addTag(tags, "stomp");
  if (result === "loss" && (killDiff <= -8 || objectiveDiff <= -6)) addTag(tags, "stomped");

  const deficit = comebackGoldDeficit({
    result,
    userSide,
    goldDiffTimeline: timeline(match),
  });
  if (deficit >= 1500) {
    addTag(tags, "comeback");
    facts.comebackGoldDeficit = deficit;
  }

  if (objectiveDiff >= 2) addTag(tags, "neutral_objectives");

  const underperformers = userPlayers.filter((player) => (player.rating ?? 10) < 5 || (player.deaths ?? 0) >= 5);
  if (result === "loss" && underperformers.length > 0) addTag(tags, "underperformance");
  if (result === "loss" && underperformers.some((player) => (player.deaths ?? 0) >= 6)) addTag(tags, "decisive_mistake");

  if (params.draft) {
    addTag(tags, "draft");
    if (params.draft.strongSide) facts.strongSide = params.draft.strongSide;
    if (params.draft.timing) {
      facts.timing = params.draft.timing;
      addTag(tags, `${params.draft.timing.toLowerCase()}_game`);
    }
  }

  const mvp = "mvp" in match ? match.mvp : undefined;
  if (result === "win" && mvp?.side === userSide) {
    addTag(tags, "mvp");
    const mvpId = playerId(mvp);
    if (mvpId) facts.mvpPlayerId = mvpId;
    if (mvp.role) {
      facts.mvpRole = mvp.role;
      addTag(tags, `role_${normalizeRole(mvp.role)}`);
    }
  }

  const worst = worstUserPlayer(userPlayers);
  if (result === "loss" && worst) {
    const worstId = playerId(worst);
    if (worstId) facts.worstPlayerId = worstId;
    if (worst.role) {
      facts.worstRole = worst.role;
      addTag(tags, `role_${normalizeRole(worst.role)}`);
    }
  }

  if (result === "loss" && botlaneUnderperformed(userPlayers)) {
    addTag(tags, "role_adc");
    addTag(tags, "role_support");
    addTag(tags, "botlane_underperformed");
  }

  if (params.rivalry) addTag(tags, "rivalry");
  if (params.streak && params.streak.count >= 2) {
    facts.streakCount = params.streak.count;
    addTag(tags, `streak_${params.streak.type}`);
  }

  return { facts, tags };
}
