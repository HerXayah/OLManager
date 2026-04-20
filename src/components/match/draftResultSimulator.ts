import type { GameStateData } from "../../store/gameStore";
import type { ChampionDraftResultPayload } from "./ChampionDraft";
import type { MatchSnapshot } from "./types";
import teamsSeed from "../../../data/lec/draft/teams.json";
import playersSeed from "../../../Nueva carpeta/players.json";
import {
  DEFAULT_LOL_TACTICS,
  ROLE_ORDER,
  computeCoherenceBreakdown,
  computeRoleModifiers,
  type DraftRole,
} from "../../lib/lolTactics";
import { computeTeamTimingFit, getChampionTiming } from "../../lib/championTiming";

type Side = "blue" | "red";
type Role = DraftRole;

type LolTacticsData = NonNullable<GameStateData["teams"][number]["lol_tactics"]>;

interface TeamSeed {
  id: string;
  name: string;
}

interface PlayerSeed {
  ign: string;
  teamId: string;
  role: string;
  rating?: number;
}

const TEAM_SEEDS: TeamSeed[] = ((teamsSeed as { data?: { teams?: TeamSeed[] } }).data?.teams ?? []) as TeamSeed[];
const PLAYER_SEEDS: PlayerSeed[] =
  ((playersSeed as { data?: { rostered_seeds?: PlayerSeed[] } }).data?.rostered_seeds ?? []) as PlayerSeed[];

const ATTRIBUTE_KEYS = [
  "pace",
  "stamina",
  "strength",
  "agility",
  "passing",
  "shooting",
  "tackling",
  "dribbling",
  "defending",
  "positioning",
  "vision",
  "decisions",
  "composure",
  "aggression",
  "teamwork",
  "leadership",
] as const;

export interface DraftPlayerResult {
  side: Side;
  playerId: string;
  playerName: string;
  role: Role;
  championId: string | null;
  kills: number;
  deaths: number;
  assists: number;
  gold: number;
  rating: number;
}

export interface DraftTeamObjectives {
  voidgrubs: number;
  dragons: number;
  dragonSoul: boolean;
  elderDragons: number;
  heralds: number;
  barons: number;
  towers: number;
  inhibitors: number;
}

export interface DraftTimelineEvent {
  minute: number;
  side: Side;
  type:
    | "first_blood"
    | "voidgrubs"
    | "dragon"
    | "dragon_soul"
    | "elder"
    | "herald"
    | "baron"
    | "turret"
    | "inhibitor";
  label: string;
}

export interface DraftMatchResult {
  winnerSide: Side;
  durationMinutes: number;
  blueKills: number;
  redKills: number;
  mvp: DraftPlayerResult;
  playerResults: DraftPlayerResult[];
  goldDiffTimeline: Array<{ minute: number; diff: number }>;
  timelineEvents: DraftTimelineEvent[];
  objectives: {
    blue: DraftTeamObjectives;
    red: DraftTeamObjectives;
  };
  power: {
    blue: number;
    red: number;
    diff: number;
    autoWin: boolean;
    winProbBlue: number;
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function hashText(value: string): number {
  let hash = 0;
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash * 31 + value.charCodeAt(i)) >>> 0;
  }
  return hash;
}

function mulberry32(seed: number): () => number {
  return () => {
    let t = (seed += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function average(values: number[]): number {
  if (values.length === 0) return 0;
  return values.reduce((sum, item) => sum + item, 0) / values.length;
}

function normalizeKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]/g, "");
}

function mapSeedRoleToDraftRole(role: string): Role | null {
  const key = normalizeKey(role);
  if (key === "top") return "TOP";
  if (key === "jungle") return "JUNGLA";
  if (key === "mid" || key === "middle") return "MID";
  if (key === "bot" || key === "bottom" || key === "adc") return "ADC";
  if (key === "support" || key === "sup") return "SUPPORT";
  return null;
}

function gameStatePositionToDraftRole(position: string): Role | null {
  const normalized = normalizeKey(position);
  if (normalized === "defender") return "TOP";
  if (normalized === "midfielder") return "JUNGLA";
  if (normalized === "attackingmidfielder") return "MID";
  if (normalized === "forward") return "ADC";
  if (normalized === "defensivemidfielder" || normalized === "goalkeeper") return "SUPPORT";
  return null;
}

function toEnginePlayerFromState(
  player: GameStateData["players"][number],
): MatchSnapshot["home_team"]["players"][number] {
  return {
    id: player.id,
    name: player.match_name,
    position: player.position,
    condition: player.condition,
    pace: player.attributes.pace,
    stamina: player.attributes.stamina,
    strength: player.attributes.strength,
    agility: player.attributes.agility,
    passing: player.attributes.passing,
    shooting: player.attributes.shooting,
    tackling: player.attributes.tackling,
    dribbling: player.attributes.dribbling,
    defending: player.attributes.defending,
    positioning: player.attributes.positioning,
    vision: player.attributes.vision,
    decisions: player.attributes.decisions,
    composure: player.attributes.composure,
    aggression: player.attributes.aggression,
    teamwork: player.attributes.teamwork,
    leadership: player.attributes.leadership,
    handling: player.attributes.handling,
    reflexes: player.attributes.reflexes,
    aerial: player.attributes.aerial,
    traits: player.traits,
  };
}

function resolvePlayersFromSeed(
  snapshot: MatchSnapshot,
  gameState: GameStateData,
  side: Side,
): MatchSnapshot["home_team"]["players"] {
  const team = side === "blue" ? snapshot.home_team : snapshot.away_team;
  const starters = side === "blue" ? snapshot.home_team.players : snapshot.away_team.players;

  const seedTeam = TEAM_SEEDS.find((item) => normalizeKey(item.name) === normalizeKey(team.name));
  if (!seedTeam) return starters.slice(0, 5);

  const teamSeedPlayers = PLAYER_SEEDS.filter((player) => player.teamId === seedTeam.id);
  if (teamSeedPlayers.length === 0) return starters.slice(0, 5);

  const stateTeam = gameState.teams.find(
    (item) => normalizeKey(item.name) === normalizeKey(team.name),
  );

  const stateTeamPlayers = stateTeam
    ? gameState.players.filter((player) => player.team_id === stateTeam.id)
    : [];

  const selected: MatchSnapshot["home_team"]["players"] = [];
  const usedIds = new Set<string>();

  ROLE_ORDER.forEach((role) => {
    const roleSeed = teamSeedPlayers
      .filter((player) => mapSeedRoleToDraftRole(player.role) === role)
      .sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0))[0];
    if (!roleSeed) return;

    const match = stateTeamPlayers.find(
      (player) =>
        !usedIds.has(player.id) &&
        normalizeKey(player.match_name) === normalizeKey(roleSeed.ign),
    );
    if (!match) return;

    selected.push(toEnginePlayerFromState(match));
    usedIds.add(match.id);
  });

  if (selected.length < 5 && stateTeamPlayers.length > 0) {
    ROLE_ORDER.forEach((role) => {
      if (selected.length >= 5) return;
      const fallback = stateTeamPlayers
        .filter(
          (player) =>
            !usedIds.has(player.id) &&
            gameStatePositionToDraftRole(player.position) === role,
        )
        .sort((a, b) => b.attributes.teamwork + b.attributes.decisions - (a.attributes.teamwork + a.attributes.decisions))[0];

      if (!fallback) return;
      selected.push(toEnginePlayerFromState(fallback));
      usedIds.add(fallback.id);
    });
  }

  if (selected.length < 5) {
    starters.forEach((player) => {
      if (selected.length >= 5) return;
      if (usedIds.has(player.id)) return;
      selected.push(player);
      usedIds.add(player.id);
    });
  }

  return selected.slice(0, 5);
}

function teamSideData(snapshot: MatchSnapshot, gameState: GameStateData, side: Side) {
  const team = side === "blue" ? snapshot.home_team : snapshot.away_team;
  const stateTeam = gameState.teams.find(
    (item) => normalizeKey(item.name) === normalizeKey(team.name),
  );

  return side === "blue"
    ? {
      team: snapshot.home_team,
      players: resolvePlayersFromSeed(snapshot, gameState, "blue"),
      tactics: stateTeam?.lol_tactics ?? DEFAULT_LOL_TACTICS,
    }
    : {
      team: snapshot.away_team,
      players: resolvePlayersFromSeed(snapshot, gameState, "red"),
      tactics: stateTeam?.lol_tactics ?? DEFAULT_LOL_TACTICS,
    };
}

function laneOvrByRole(players: MatchSnapshot["home_team"]["players"]): Record<Role, number> {
  const map: Record<Role, number> = {
    TOP: 70,
    JUNGLA: 70,
    MID: 70,
    ADC: 70,
    SUPPORT: 70,
  };

  players.forEach((player, index) => {
    const role = ROLE_ORDER[index] ?? "MID";
    map[role] = playerOverall(player);
  });

  return map;
}

function tacticsPowerBonus(params: {
  own: LolTacticsData;
  ownPlayers: MatchSnapshot["home_team"]["players"];
  enemyPlayers: MatchSnapshot["home_team"]["players"];
  ownPicks: ChampionDraftResultPayload["blue"]["picks"];
}): number {
  const { own, ownPlayers, enemyPlayers, ownPicks } = params;

  const ownLanesRaw = laneOvrByRole(ownPlayers);
  const roleModifiers = computeRoleModifiers(own);
  const pickByRole = new Map<Role, string | null>(
    ownPicks.map((pick) => [pick.role as Role, pick.championId]),
  );

  const adjustedRoleModifiers: Record<Role, number> = { ...roleModifiers };
  ROLE_ORDER.forEach((role) => {
    const current = roleModifiers[role];
    if (current >= 0) return;

    const championId = pickByRole.get(role) ?? null;
    const timing = getChampionTiming(championId);

    // Weakside tech: campeones de early ayudan a neutralizar castigo del rol débil.
    if (timing === "Early") {
      adjustedRoleModifiers[role] = Math.min(0, current + Math.abs(current));
    }
  });

  const ownLanes = {
    TOP: ownLanesRaw.TOP + adjustedRoleModifiers.TOP * 1.8,
    JUNGLA: ownLanesRaw.JUNGLA + adjustedRoleModifiers.JUNGLA * 1.8,
    MID: ownLanesRaw.MID + adjustedRoleModifiers.MID * 1.8,
    ADC: ownLanesRaw.ADC + adjustedRoleModifiers.ADC * 1.8,
    SUPPORT: ownLanesRaw.SUPPORT + adjustedRoleModifiers.SUPPORT * 1.8,
  };
  const enemyLanes = laneOvrByRole(enemyPlayers);

  let score = 0;

  const laneDelta = {
    Top: ownLanes.TOP - enemyLanes.TOP,
    Mid: ownLanes.MID - enemyLanes.MID,
    Bot: (ownLanes.ADC + ownLanes.SUPPORT) / 2 - (enemyLanes.ADC + enemyLanes.SUPPORT) / 2,
  };

  score += laneDelta[own.strong_side] * 0.22;

  const ownCondition = average(ownPlayers.map((player) => Number(player.condition ?? 70)));
  const ownComposure = average(ownPlayers.map((player) => Number(player.composure ?? 70)));
  const ownTeamwork = average(ownPlayers.map((player) => Number(player.teamwork ?? 70)));

  if (own.game_timing === "Early") score += (ownCondition - 72) * 0.15;
  if (own.game_timing === "Mid") score += (ownTeamwork - 70) * 0.11;
  if (own.game_timing === "Late") score += (ownComposure - 70) * 0.14;

  if (own.jungle_style === "Ganker") score += 1.8;
  if (own.jungle_style === "Invader") score += 1.2;
  if (own.jungle_style === "Farmer") score += 0.9;
  if (own.jungle_style === "Enabler") score += 1.5;

  if (own.strong_side === "Bot" && own.jungle_pathing === "TopToBot") score += 1.4;
  if (own.strong_side === "Top" && own.jungle_pathing === "BotToTop") score += 1.4;
  if (own.strong_side === "Mid") score += 0.6;

  if (own.fight_plan === "FrontToBack") score += (ownLanes.SUPPORT - 68) * 0.09;
  if (own.fight_plan === "Pick") score += (ownLanes.MID - 70) * 0.11;
  if (own.fight_plan === "Dive") score += (ownLanes.TOP - 70) * 0.1;
  if (own.fight_plan === "Siege") score += (ownLanes.ADC - 70) * 0.12;

  if (own.support_roaming === "Lane") {
    score += ((ownLanes.ADC + ownLanes.SUPPORT) / 2 - 68) * 0.08;
  } else if (own.support_roaming === "RoamMid") {
    score += (ownLanes.MID - 70) * 0.09;
    score += (ownLanes.JUNGLA - 70) * 0.05;
  } else {
    score += (ownLanes.TOP - 70) * 0.09;
    score += (ownLanes.JUNGLA - 70) * 0.05;
  }

  const coherenceScore = computeCoherenceBreakdown(own).reduce((sum, item) => sum + item.delta, 0);
  score += coherenceScore * 2.2;

  const teamTimingFit = computeTeamTimingFit({
    championIds: ownPicks.map((pick) => pick.championId),
    preference: own.game_timing,
  });
  score += teamTimingFit * 2.5;

  return clamp(score, -12, 12);
}

function playerOverall(player: MatchSnapshot["home_team"]["players"][number]): number {
  const values = ATTRIBUTE_KEYS.map((key) => Number(player[key] ?? 0));
  return average(values);
}

function teamMorale(gameState: GameStateData, players: MatchSnapshot["home_team"]["players"]): number {
  const moraleValues = players
    .map((player) => gameState.players.find((p) => p.id === player.id)?.morale)
    .filter((value): value is number => typeof value === "number");
  return moraleValues.length > 0 ? average(moraleValues) : 60;
}

function teamCondition(gameState: GameStateData, players: MatchSnapshot["home_team"]["players"]): number {
  const conditionValues = players
    .map((player) => {
      const fromState = gameState.players.find((p) => p.id === player.id)?.condition;
      return typeof fromState === "number" ? fromState : player.condition;
    })
    .filter((value): value is number => typeof value === "number");
  return conditionValues.length > 0 ? average(conditionValues) : 75;
}

function pickChampionByRole(
  picks: ChampionDraftResultPayload["blue"]["picks"],
  role: Role,
): string | null {
  return picks.find((pick) => pick.role === role)?.championId ?? null;
}

function weightedAllocation(total: number, weights: number[], rand: () => number): number[] {
  if (weights.length === 0) return [];
  const result = Array.from({ length: weights.length }, () => 0);
  const safeWeights = weights.map((w) => Math.max(0.001, w));
  const weightSum = safeWeights.reduce((sum, w) => sum + w, 0);

  for (let i = 0; i < total; i += 1) {
    let needle = rand() * weightSum;
    for (let idx = 0; idx < safeWeights.length; idx += 1) {
      needle -= safeWeights[idx];
      if (needle <= 0) {
        result[idx] += 1;
        break;
      }
    }
  }

  return result;
}

function pickMatchDuration(rand: () => number): number {
  const bucket = rand();
  if (bucket < 0.7) {
    // Mayoría de partidas: 25-40
    return 25 + Math.floor(rand() * 16);
  }
  if (bucket < 0.9) {
    // Partidas cortas: 18-24
    return 18 + Math.floor(rand() * 7);
  }
  // Partidas largas: 41-58
  return 41 + Math.floor(rand() * 18);
}

export function simulateDraftMatchResult(params: {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  draft: ChampionDraftResultPayload;
}): DraftMatchResult {
  const { snapshot, gameState, draft } = params;

  const seed = hashText(
    `${snapshot.home_team.name}|${snapshot.away_team.name}|${draft.history.join("|")}|${draft.blue.score.total}|${draft.red.score.total}`,
  );
  const rand = mulberry32(seed);

  const blueData = teamSideData(snapshot, gameState, "blue");
  const redData = teamSideData(snapshot, gameState, "red");

  const blueOverall = average(blueData.players.map(playerOverall));
  const redOverall = average(redData.players.map(playerOverall));

  const blueTacticsBonus = tacticsPowerBonus({
    own: blueData.tactics,
    ownPlayers: blueData.players,
    enemyPlayers: redData.players,
    ownPicks: draft.blue.picks,
  });
  const redTacticsBonus = tacticsPowerBonus({
    own: redData.tactics,
    ownPlayers: redData.players,
    enemyPlayers: blueData.players,
    ownPicks: draft.red.picks,
  });

  const blueMorale = teamMorale(gameState, blueData.players);
  const redMorale = teamMorale(gameState, redData.players);
  const blueCondition = teamCondition(gameState, blueData.players);
  const redCondition = teamCondition(gameState, redData.players);

  const blueDraftStrength = clamp(50 + draft.blue.score.total * 2.5, 0, 100);
  const redDraftStrength = clamp(50 + draft.red.score.total * 2.5, 0, 100);

  const bluePower =
    blueOverall * 0.35 +
    blueDraftStrength * 0.20 +
    blueTacticsBonus +
    blueMorale * 0.15 +
    blueCondition * 0.10;
  const redPower =
    redOverall * 0.35 +
    redDraftStrength * 0.20 +
    redTacticsBonus +
    redMorale * 0.15 +
    redCondition * 0.10;

  const powerDiff = bluePower - redPower;
  const autoWinThreshold = 16;
  const autoWin = Math.abs(powerDiff) >= autoWinThreshold;
  const winProbBlue = clamp(50 + powerDiff * 3.5, 5, 95);
  const winnerSide: Side = autoWin
    ? powerDiff >= 0
      ? "blue"
      : "red"
    : rand() * 100 <= winProbBlue
      ? "blue"
      : "red";

  const strongSideBias = (side: Side): number => {
    const plan = side === "blue" ? blueData.tactics : redData.tactics;
    if (plan.strong_side === "Bot") return 0.03;
    if (plan.strong_side === "Top") return -0.03;
    return 0;
  };

  const timingBias = (side: Side, minute: number): number => {
    const plan = side === "blue" ? blueData.tactics : redData.tactics;
    if (plan.game_timing === "Early") return minute <= 14 ? 0.06 : -0.03;
    if (plan.game_timing === "Late") return minute >= 22 ? 0.06 : -0.03;
    return minute >= 12 && minute <= 24 ? 0.04 : 0;
  };

  const supportRoamBias = (side: Side, minute: number): number => {
    const plan = side === "blue" ? blueData.tactics : redData.tactics;
    if (plan.support_roaming === "Lane") return minute <= 14 ? 0.03 : 0;
    if (plan.support_roaming === "RoamMid") return minute <= 18 ? 0.04 : 0.01;
    return minute <= 14 ? 0.035 : -0.005;
  };

  const durationMinutes = pickMatchDuration(rand);
  const winnerBonus = clamp(Math.round(Math.abs(powerDiff) / 4), 0, 6);

  const blueKills = clamp(
    Math.round(9 + rand() * 8 + (winnerSide === "blue" ? 3 + winnerBonus : -1)),
    4,
    35,
  );
  const redKills = clamp(
    Math.round(9 + rand() * 8 + (winnerSide === "red" ? 3 + winnerBonus : -1)),
    4,
    35,
  );

  const blueObjectives: DraftTeamObjectives = {
    voidgrubs: 0,
    dragons: 0,
    dragonSoul: false,
    elderDragons: 0,
    heralds: 0,
    barons: 0,
    towers: 0,
    inhibitors: 0,
  };
  const redObjectives: DraftTeamObjectives = {
    voidgrubs: 0,
    dragons: 0,
    dragonSoul: false,
    elderDragons: 0,
    heralds: 0,
    barons: 0,
    towers: 0,
    inhibitors: 0,
  };

  const timelineEvents: DraftTimelineEvent[] = [];

  // First blood (temprano)
  const firstBloodMinute = clamp(2 + Math.floor(rand() * 7), 2, Math.min(9, durationMinutes - 1));
  const earlyBlueBias =
    strongSideBias("blue") - strongSideBias("red") +
    timingBias("blue", firstBloodMinute) - timingBias("red", firstBloodMinute) +
    supportRoamBias("blue", firstBloodMinute) - supportRoamBias("red", firstBloodMinute);
  const firstBloodSide: Side = rand() < 0.5 + earlyBlueBias ? "blue" : "red";
  timelineEvents.push({
    minute: firstBloodMinute,
    side: firstBloodSide,
    type: "first_blood",
    label: "First Blood",
  });

  // Voidgrubs: spawn 8:00, camp único (3 unidades), desaparece antes de 15:00
  if (durationMinutes >= 8) {
    const voidgrubMinute = clamp(8 + Math.floor(rand() * 5), 8, Math.min(14, durationMinutes - 1));
    const voidgrubBias =
      strongSideBias("blue") - strongSideBias("red") +
      timingBias("blue", voidgrubMinute) - timingBias("red", voidgrubMinute) +
      supportRoamBias("blue", voidgrubMinute) - supportRoamBias("red", voidgrubMinute);
    const voidgrubTaker: Side = rand() < 0.5 + voidgrubBias ? "blue" : "red";
    if (voidgrubTaker === "blue") blueObjectives.voidgrubs += 3;
    else redObjectives.voidgrubs += 3;

    timelineEvents.push({
      minute: voidgrubMinute,
      side: voidgrubTaker,
      type: "voidgrubs",
      label: "Voidgrubs x3",
    });
  }

  // Dragones: spawn 5:00, respawn 5:00; Soul al 4to, luego Elder cada 6:00
  let nextDragonSpawn = 5;
  let soulSide: Side | null = null;

  while (nextDragonSpawn < durationMinutes) {
    const takeMinute = clamp(nextDragonSpawn + Math.floor(rand() * 3), nextDragonSpawn, durationMinutes - 1);

    if (soulSide) {
      const elderBias = timingBias("blue", takeMinute) - timingBias("red", takeMinute);
      const elderTaker: Side = rand() < 0.5 + elderBias ? "blue" : "red";
      if (elderTaker === "blue") blueObjectives.elderDragons += 1;
      else redObjectives.elderDragons += 1;

      timelineEvents.push({
        minute: takeMinute,
        side: elderTaker,
        type: "elder",
        label: "Elder Dragon",
      });
      nextDragonSpawn = takeMinute + 6;
      continue;
    }

    const dragonBias =
      strongSideBias("blue") - strongSideBias("red") +
      timingBias("blue", takeMinute) - timingBias("red", takeMinute) +
      supportRoamBias("blue", takeMinute) - supportRoamBias("red", takeMinute);
    const dragonTaker: Side = rand() < 0.5 + dragonBias ? "blue" : "red";
    if (dragonTaker === "blue") blueObjectives.dragons += 1;
    else redObjectives.dragons += 1;

    timelineEvents.push({
      minute: takeMinute,
      side: dragonTaker,
      type: "dragon",
      label: "Dragon",
    });

    const blueHasSoul = blueObjectives.dragons >= 4;
    const redHasSoul = redObjectives.dragons >= 4;
    if (blueHasSoul || redHasSoul) {
      soulSide = blueHasSoul ? "blue" : "red";
      if (soulSide === "blue") blueObjectives.dragonSoul = true;
      else redObjectives.dragonSoul = true;

      timelineEvents.push({
        minute: takeMinute,
        side: soulSide,
        type: "dragon_soul",
        label: "Dragon Soul",
      });
      nextDragonSpawn = takeMinute + 6;
      continue;
    }

    nextDragonSpawn = takeMinute + 5;
  }

  // Herald: aparece 15:00, desaparece antes de 20:00
  if (durationMinutes >= 15) {
    const heraldMinute = clamp(15 + Math.floor(rand() * 4), 15, Math.min(19, durationMinutes - 1));
    const heraldBias =
      strongSideBias("blue") - strongSideBias("red") +
      timingBias("blue", heraldMinute) - timingBias("red", heraldMinute) +
      supportRoamBias("blue", heraldMinute) - supportRoamBias("red", heraldMinute);
    const heraldTaker: Side = rand() < 0.5 + heraldBias ? "blue" : "red";
    if (heraldTaker === "blue") blueObjectives.heralds += 1;
    else redObjectives.heralds += 1;

    timelineEvents.push({
      minute: heraldMinute,
      side: heraldTaker,
      type: "herald",
      label: "Herald",
    });
  }

  // Baron: 20:00, respawn 6:00
  let nextBaronSpawn = 20;
  while (nextBaronSpawn < durationMinutes) {
    const takeMinute = clamp(nextBaronSpawn + Math.floor(rand() * 3), nextBaronSpawn, durationMinutes - 1);
    const baronBias = timingBias("blue", takeMinute) - timingBias("red", takeMinute);
    const baronTaker: Side = rand() < 0.5 + baronBias ? "blue" : "red";
    if (baronTaker === "blue") blueObjectives.barons += 1;
    else redObjectives.barons += 1;

    timelineEvents.push({
      minute: takeMinute,
      side: baronTaker,
      type: "baron",
      label: "Baron",
    });

    nextBaronSpawn = takeMinute + 6;
  }

  // Torres/Inhibidores: escalado más realista por duración
  const winnerTowers = clamp(Math.round(2 + durationMinutes * 0.14 + rand() * 1.5), 4, 11);
  const loserTowers = clamp(Math.round(1 + durationMinutes * 0.08 + rand() * 1.3), 1, 8);

  const adjustedWinnerTowers = Math.max(winnerTowers, loserTowers + 1);
  const adjustedLoserTowers = Math.min(loserTowers, adjustedWinnerTowers - 1);

  const winnerInhibitors = clamp(
    Math.floor(Math.max(0, adjustedWinnerTowers - 6) / 2) + (rand() < 0.35 ? 1 : 0),
    0,
    3,
  );
  const loserInhibitors = clamp(
    Math.floor(Math.max(0, adjustedLoserTowers - 8) / 3) + (rand() < 0.1 ? 1 : 0),
    0,
    2,
  );

  if (winnerSide === "blue") {
    blueObjectives.towers = adjustedWinnerTowers;
    redObjectives.towers = adjustedLoserTowers;
    blueObjectives.inhibitors = winnerInhibitors;
    redObjectives.inhibitors = loserInhibitors;
  } else {
    redObjectives.towers = adjustedWinnerTowers;
    blueObjectives.towers = adjustedLoserTowers;
    redObjectives.inhibitors = winnerInhibitors;
    blueObjectives.inhibitors = loserInhibitors;
  }

  for (let i = 0; i < blueObjectives.towers; i += 1) {
    const minute = clamp(12 + Math.floor(rand() * Math.max(1, durationMinutes - 12)), 12, durationMinutes - 1);
    timelineEvents.push({ minute, side: "blue", type: "turret", label: "Turret" });
  }
  for (let i = 0; i < redObjectives.towers; i += 1) {
    const minute = clamp(12 + Math.floor(rand() * Math.max(1, durationMinutes - 12)), 12, durationMinutes - 1);
    timelineEvents.push({ minute, side: "red", type: "turret", label: "Turret" });
  }
  for (let i = 0; i < blueObjectives.inhibitors; i += 1) {
    const minute = clamp(22 + Math.floor(rand() * Math.max(1, durationMinutes - 22)), 22, durationMinutes - 1);
    timelineEvents.push({ minute, side: "blue", type: "inhibitor", label: "Inhibitor" });
  }
  for (let i = 0; i < redObjectives.inhibitors; i += 1) {
    const minute = clamp(22 + Math.floor(rand() * Math.max(1, durationMinutes - 22)), 22, durationMinutes - 1);
    timelineEvents.push({ minute, side: "red", type: "inhibitor", label: "Inhibitor" });
  }

  timelineEvents.sort((a, b) => a.minute - b.minute);

  const blueKillWeights = [1.1, 1.15, 1.35, 1.45, 0.65].map((base) => base + rand() * 0.4);
  const redKillWeights = [1.1, 1.15, 1.35, 1.45, 0.65].map((base) => base + rand() * 0.4);
  const blueDeathWeights = [1.0, 1.0, 1.05, 1.05, 0.9].map((base) => base + rand() * 0.4);
  const redDeathWeights = [1.0, 1.0, 1.05, 1.05, 0.9].map((base) => base + rand() * 0.4);

  const blueKillsByPlayer = weightedAllocation(blueKills, blueKillWeights, rand);
  const redKillsByPlayer = weightedAllocation(redKills, redKillWeights, rand);
  const blueDeathsByPlayer = weightedAllocation(redKills, blueDeathWeights, rand);
  const redDeathsByPlayer = weightedAllocation(blueKills, redDeathWeights, rand);

  const blueAssistPool = clamp(Math.round(blueKills * (1.7 + rand() * 0.8)), blueKills, blueKills * 4);
  const redAssistPool = clamp(Math.round(redKills * (1.7 + rand() * 0.8)), redKills, redKills * 4);
  const blueAssistsByPlayer = weightedAllocation(blueAssistPool, [1, 1.1, 1, 1, 1.7], rand);
  const redAssistsByPlayer = weightedAllocation(redAssistPool, [1, 1.1, 1, 1, 1.7], rand);

  const buildSideResults = (
    side: Side,
    teamPlayers: typeof blueData.players,
    picks: ChampionDraftResultPayload["blue"]["picks"],
    killsByPlayer: number[],
    deathsByPlayer: number[],
    assistsByPlayer: number[],
    isWinner: boolean,
  ): DraftPlayerResult[] => {
    return teamPlayers.map((player, index) => {
      const role = ROLE_ORDER[index] ?? "MID";
      const kills = killsByPlayer[index] ?? 0;
      const deaths = deathsByPlayer[index] ?? 0;
      const assists = assistsByPlayer[index] ?? 0;
      const championId = pickChampionByRole(picks, role);
      const ratingRaw = 5.5 + kills * 0.45 + assists * 0.12 - deaths * 0.35 + (isWinner ? 0.8 : -0.4);
      const rating = clamp(Number(ratingRaw.toFixed(1)), 1, 10);
      const gold = Math.max(
        6000,
        Math.round(
          6800 +
            (durationMinutes - 20) * 120 +
            kills * 340 +
            assists * 120 -
            deaths * 60 +
            rand() * 1400 +
            (isWinner ? 800 : 0),
        ),
      );

      return {
        side,
        playerId: player.id,
        playerName: player.name,
        role,
        championId,
        kills,
        deaths,
        assists,
        gold,
        rating,
      };
    });
  };

  const bluePlayerResults = buildSideResults(
    "blue",
    blueData.players,
    draft.blue.picks,
    blueKillsByPlayer,
    blueDeathsByPlayer,
    blueAssistsByPlayer,
    winnerSide === "blue",
  );
  const redPlayerResults = buildSideResults(
    "red",
    redData.players,
    draft.red.picks,
    redKillsByPlayer,
    redDeathsByPlayer,
    redAssistsByPlayer,
    winnerSide === "red",
  );

  const playerResults = [...bluePlayerResults, ...redPlayerResults];
  const mvp = [...playerResults].sort((a, b) => {
    if (b.rating !== a.rating) return b.rating - a.rating;
    const aImpact = a.kills + a.assists;
    const bImpact = b.kills + b.assists;
    return bImpact - aImpact;
  })[0];

  const timelineMinutes = Array.from({ length: Math.floor(durationMinutes / 5) + 1 }, (_, idx) => idx * 5);

  const eventGoldImpact = (event: DraftTimelineEvent): number => {
    const sign = event.side === "blue" ? 1 : -1;
    switch (event.type) {
      case "first_blood":
        return sign * 450;
      case "voidgrubs":
        return sign * 320;
      case "dragon":
        return sign * 220;
      case "dragon_soul":
        return 0;
      case "elder":
        return sign * 1100;
      case "herald":
        return sign * 380;
      case "baron":
        return sign * 900;
      case "turret":
        return sign * 650;
      case "inhibitor":
        return sign * 850;
      default:
        return 0;
    }
  };

  const blueSoulMinute = timelineEvents.find((event) => event.type === "dragon_soul" && event.side === "blue")?.minute;
  const redSoulMinute = timelineEvents.find((event) => event.type === "dragon_soul" && event.side === "red")?.minute;

  const winnerTrendPerMinute =
    (winnerSide === "blue" ? 1 : -1) *
    (110 + Math.abs(powerDiff) * 15 + Math.abs(blueKills - redKills) * 5);

  const goldDiffTimeline = timelineMinutes.map((minute, idx) => {
    if (idx === 0) return { minute, diff: 0 };

    const eventContribution = timelineEvents
      .filter((event) => event.minute <= minute)
      .reduce((sum, event) => sum + eventGoldImpact(event), 0);

    const trendContribution = Math.round(winnerTrendPerMinute * minute);

    let soulMomentum = 0;
    if (typeof blueSoulMinute === "number" && minute > blueSoulMinute) {
      soulMomentum += (minute - blueSoulMinute) * 40;
    }
    if (typeof redSoulMinute === "number" && minute > redSoulMinute) {
      soulMomentum -= (minute - redSoulMinute) * 40;
    }

    const jitter = Math.round((rand() - 0.5) * 240 * (1 - minute / Math.max(1, durationMinutes)));

    const diff = clamp(
      Math.round(eventContribution + trendContribution + soulMomentum + jitter),
      -20000,
      20000,
    );
    return { minute, diff };
  });

  if (goldDiffTimeline.length > 0) {
    const last = goldDiffTimeline[goldDiffTimeline.length - 1];
    if ((winnerSide === "blue" && last.diff < 0) || (winnerSide === "red" && last.diff > 0)) {
      goldDiffTimeline[goldDiffTimeline.length - 1] = {
        minute: last.minute,
        diff: Math.round(-last.diff * 0.65),
      };
    }
  }

  return {
    winnerSide,
    durationMinutes,
    blueKills,
    redKills,
    mvp,
    playerResults,
    goldDiffTimeline,
    timelineEvents,
    objectives: {
      blue: blueObjectives,
      red: redObjectives,
    },
    power: {
      blue: Number(bluePower.toFixed(2)),
      red: Number(redPower.toFixed(2)),
      diff: Number(powerDiff.toFixed(2)),
      autoWin,
      winProbBlue,
    },
  };
}
