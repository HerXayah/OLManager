import { useEffect, useState, useCallback, useMemo, useRef } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { useGameStore, GameStateData, LolTacticsData } from "../store/gameStore";
import { useSettingsStore } from "../store/settingsStore";
import {
  MatchSnapshot,
  MatchEvent,
  MatchDayStage,
} from "../components/match/types";
import { resolveMatchFixture } from "../components/match/helpers";
import PreMatchSetup from "../components/match/PreMatchSetup";
import ChampionDraft from "../components/match/ChampionDraft";
import type { ChampionDraftResultPayload } from "../components/match/ChampionDraft";
import LolMatchLive from "../components/match/LolMatchLive";
import type { ChampionSelectionByPlayer } from "../components/match/LolMatchLive";
import MatchTacticsStage from "../components/match/MatchTacticsStage";
import LolResultScreen from "../components/match/LolResultScreen";
import DraftResultScreen from "../components/match/DraftResultScreen";
import PressConference from "../components/match/PressConference";
import {
  simulateDraftMatchResult,
  type DraftPlayerResult,
  type DraftMatchResult,
} from "../components/match/draftResultSimulator";
import {
  lolSimV2ClearTelemetryFiles,
  lolSimV2RunToCompletion,
} from "../components/match/lol-prototype/backend/tauri-client";
import type {
  LolSimV1MatchReportInput,
  LolSimV1PolicyConfig,
  LolSimV1RuntimeState,
} from "../components/match/lol-prototype/backend/contract-v1";
import { computeRoleModifiers, ROLE_ORDER, type DraftRole } from "../lib/lolTactics";

// ---------------------------------------------------------------------------
// Multi-stage Match Day Orchestrator
// ---------------------------------------------------------------------------

interface MatchRouteState {
  fixtureIndex?: number;
  mode?: string;
  snapshot?: MatchSnapshot;
}

interface FinishLiveMatchResponse {
  game: GameStateData;
  round_summary?: unknown;
}

interface StoredFixtureDraftResult {
  snapshot: MatchSnapshot;
  controlledSide: "blue" | "red";
  result: DraftMatchResult;
  seriesLength?: 1 | 3 | 5;
  seriesGameIndex?: number;
  userSeriesWins?: number;
  opponentSeriesWins?: number;
  homeSeriesWins?: number;
  awaySeriesWins?: number;
}

const DEFAULT_LOL_TACTICS: LolTacticsData = {
  strong_side: "Bot",
  game_timing: "Mid",
  jungle_style: "Enabler",
  jungle_pathing: "TopToBot",
  fight_plan: "FrontToBack",
  support_roaming: "Lane",
};

function attachLolTacticsToSnapshot(snapshot: MatchSnapshot, gameState: GameStateData): MatchSnapshot {
  const homeTeam = gameState.teams.find((team) => team.id === snapshot.home_team.id);
  const awayTeam = gameState.teams.find((team) => team.id === snapshot.away_team.id);

  const normalizePosition = (position: string) => position.toLowerCase().replace(/[^a-z]/g, "");
  const positionToRole = (position: string): DraftRole | null => {
    const normalized = normalizePosition(position);
    if (normalized === "defender") return "TOP";
    if (normalized === "midfielder") return "JUNGLE";
    if (normalized === "attackingmidfielder") return "MID";
    if (normalized === "forward") return "ADC";
    if (normalized === "defensivemidfielder" || normalized === "goalkeeper") return "SUPPORT";
    return null;
  };

  const buildImpactByPlayer = (
    players: MatchSnapshot["home_team"]["players"],
    tactics: LolTacticsData,
  ): Record<string, { modifier: number; variance: number }> => {
    const roleModifiers = computeRoleModifiers(tactics);
    const byRole = new Map<DraftRole, MatchSnapshot["home_team"]["players"][number]>();

    players.forEach((player) => {
      const role = positionToRole(player.position);
      if (!role || byRole.has(role)) return;
      byRole.set(role, player);
    });

    const impact: Record<string, { modifier: number; variance: number }> = {};
    ROLE_ORDER.forEach((role) => {
      const player = byRole.get(role);
      if (!player) return;
      const modifier = roleModifiers[role] ?? 0;
      const variance = Math.max(0.5, Math.abs(modifier) * 0.6 + 0.6);
      impact[player.id] = { modifier, variance };
    });

    return impact;
  };

  const homeTactics = homeTeam?.lol_tactics ?? DEFAULT_LOL_TACTICS;
  const awayTactics = awayTeam?.lol_tactics ?? DEFAULT_LOL_TACTICS;
  const roleImpactByPlayer = {
    home: buildImpactByPlayer(snapshot.home_team.players, homeTactics),
    away: buildImpactByPlayer(snapshot.away_team.players, awayTactics),
  };

  return {
    ...snapshot,
    // extra payload consumed by Rust sim v2
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    lol_tactics: {
      home: homeTactics,
      away: awayTactics,
    },
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    lol_role_impact_by_player: roleImpactByPlayer,
  } as MatchSnapshot;
}

function buildLolMatchReport(runtime: LolSimV1RuntimeState): LolSimV1MatchReportInput {
  const safeStats = {
    blue: {
      kills: runtime.stats?.blue?.kills ?? 0,
      towers: runtime.stats?.blue?.towers ?? 0,
      dragons: runtime.stats?.blue?.dragons ?? 0,
      barons: runtime.stats?.blue?.barons ?? 0,
      gold: runtime.stats?.blue?.gold ?? 0,
    },
    red: {
      kills: runtime.stats?.red?.kills ?? 0,
      towers: runtime.stats?.red?.towers ?? 0,
      dragons: runtime.stats?.red?.dragons ?? 0,
      barons: runtime.stats?.red?.barons ?? 0,
      gold: runtime.stats?.red?.gold ?? 0,
    },
  };

  return {
    winner: runtime.winner,
    timeSec: runtime.timeSec ?? 0,
    events: (runtime.events ?? []).map((event) => ({
      t: event.t,
      text: event.text,
      type: event.type,
    })),
    stats: {
      blue: {
        kills: safeStats.blue.kills,
        deaths: safeStats.red.kills,
        gold: safeStats.blue.gold,
        towers: safeStats.blue.towers,
        dragons: safeStats.blue.dragons,
        barons: safeStats.blue.barons,
      },
      red: {
        kills: safeStats.red.kills,
        deaths: safeStats.blue.kills,
        gold: safeStats.red.gold,
        towers: safeStats.red.towers,
        dragons: safeStats.red.dragons,
        barons: safeStats.red.barons,
      },
    },
    champions: (runtime.champions ?? []).map((champion) => ({
      id: champion.id,
      name: champion.name,
      team: champion.team,
      role: champion.role,
      kills: champion.kills,
      deaths: champion.deaths,
      assists: champion.assists,
      cs: champion.cs,
      gold: champion.gold,
      spentGold: champion.spentGold,
    })),
  };
}

function normalizeDraftPayload(
  payload: ChampionDraftResultPayload | null,
  selections: ChampionSelectionByPlayer | null,
  snapshot: MatchSnapshot | null,
): ChampionDraftResultPayload | null {
  if (
    payload?.blue?.picks &&
    payload?.red?.picks &&
    Array.isArray(payload.history)
  ) {
    return payload;
  }

  if (!selections || !snapshot) return null;

  const roles = ["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"] as const;

  const buildPicks = (
    players: MatchSnapshot["home_team"]["players"],
    championsByPlayer: Record<string, string>,
    rolesByPlayer: Record<string, "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT">,
  ) => {
    const taken = new Set<string>();
    return roles.map((role) => {
      const byRole = players.find((player) => rolesByPlayer[player.id] === role && !taken.has(player.id));
      const fallback = players.find((player) => !taken.has(player.id));
      const selectedPlayer = byRole ?? fallback;
      if (selectedPlayer) {
        taken.add(selectedPlayer.id);
      }
      return {
        role,
        championId: selectedPlayer ? championsByPlayer[selectedPlayer.id] ?? `generic-${role.toLowerCase()}` : `generic-${role.toLowerCase()}`,
      };
    });
  };

  const baseScore = { mastery: 0, synergy: 0, counter: 0, comfort: 0, total: 0 };

  return {
    blue: {
      picks: buildPicks(
        snapshot.home_team.players,
        selections.home,
        selections.homeRoles,
      ),
      bans: [],
      score: baseScore,
    },
    red: {
      picks: buildPicks(
        snapshot.away_team.players,
        selections.away,
        selections.awayRoles,
      ),
      bans: [],
      score: baseScore,
    },
    history: [],
  };
}

function parseRuntimeEventSide(text: string | undefined): "blue" | "red" | null {
  const upper = (text ?? "").toUpperCase();
  if (upper.includes("BLUE")) return "blue";
  if (upper.includes("RED")) return "red";
  return null;
}

function runtimeEventToDraftType(event: { type?: string; text?: string }):
  | "first_blood"
  | "voidgrubs"
  | "dragon"
  | "dragon_soul"
  | "elder"
  | "herald"
  | "baron"
  | "turret"
  | "inhibitor"
  | "nexus_turret"
  | "nexus" {
  const text = (event.text ?? "").toLowerCase();
  if (text.includes("first blood")) return "first_blood";
  if (text.includes("voidgrub")) return "voidgrubs";
  if (text.includes("dragon soul") || text.includes(" soul")) return "dragon_soul";
  if (text.includes("elder")) return "elder";
  if (text.includes("baron")) return "baron";
  if (text.includes("herald")) return "herald";
  if (text.includes("inhib")) return "inhibitor";
  if (text.includes("nexus") && text.includes("turret")) return "nexus_turret";
  if (text.includes("nexus")) return "nexus";
  if (text.includes("tower") || text.includes("turret")) return "turret";
  if (text.includes("dragon")) return "dragon";

  switch ((event.type ?? "").toLowerCase()) {
    case "kill":
      return "first_blood";
    case "tower":
      return "turret";
    case "dragon":
      return "dragon";
    case "baron":
      return "baron";
    case "nexus":
      return "nexus";
    default:
      return "herald";
  }
}

function draftTypeLabel(type: ReturnType<typeof runtimeEventToDraftType>): string {
  switch (type) {
    case "first_blood":
      return "First Blood";
    case "voidgrubs":
      return "Voidgrubs x3";
    case "dragon":
      return "Dragon";
    case "dragon_soul":
      return "Dragon Soul";
    case "elder":
      return "Elder Dragon";
    case "herald":
      return "Herald";
    case "baron":
      return "Baron";
    case "turret":
      return "Turret";
    case "inhibitor":
      return "Inhibitor";
    case "nexus_turret":
      return "Nexus Turret";
    case "nexus":
      return "Nexus";
  }
}

function runtimeRoleToDraftRole(role: string | undefined): DraftPlayerResult["role"] {
  switch ((role ?? "").toUpperCase()) {
    case "JGL":
    case "JUNGLE":
      return "JUNGLE";
    case "SUP":
    case "SUPPORT":
      return "SUPPORT";
    case "ADC":
      return "ADC";
    case "TOP":
      return "TOP";
    case "MID":
    default:
      return "MID";
  }
}

function buildDraftResultFromRuntime(params: {
  runtime: LolSimV1RuntimeState;
  snapshot: MatchSnapshot;
  championSelections: ChampionSelectionByPlayer | null;
}): DraftMatchResult {
  const { runtime, snapshot, championSelections } = params;
  const durationMinutes = Math.max(1, Math.floor((runtime.timeSec ?? 0) / 60));

  const blueKills = runtime.stats?.blue?.kills ?? 0;
  const redKills = runtime.stats?.red?.kills ?? 0;

  const champions = runtime.champions ?? [];
  const selectionByPlayerId = {
    ...(championSelections?.home ?? {}),
    ...(championSelections?.away ?? {}),
  };
  const playerResults: DraftPlayerResult[] = champions.map((champion) => ({
    side: champion.team === "red" ? "red" : "blue",
    playerId: champion.id,
    playerName: champion.name,
    role: runtimeRoleToDraftRole(champion.role),
    championId: selectionByPlayerId[champion.id] ?? null,
    kills: champion.kills ?? 0,
    deaths: champion.deaths ?? 0,
    assists: champion.assists ?? 0,
    gold: champion.gold ?? 0,
    rating: ((champion.kills ?? 0) * 2 + (champion.assists ?? 0) * 1.1 - (champion.deaths ?? 0) * 1.2 + (champion.gold ?? 0) / 1000),
  }));

  const fallbackRows: DraftPlayerResult[] = [
    ...snapshot.home_team.players.map((player, idx) => ({
      side: "blue" as const,
      playerId: player.id,
      playerName: player.name,
      role: (["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"] as const)[idx] ?? "MID",
      championId: selectionByPlayerId[player.id] ?? null,
      kills: 0,
      deaths: 0,
      assists: 0,
      gold: 0,
      rating: 5,
    })),
    ...snapshot.away_team.players.map((player, idx) => ({
      side: "red" as const,
      playerId: player.id,
      playerName: player.name,
      role: (["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"] as const)[idx] ?? "MID",
      championId: selectionByPlayerId[player.id] ?? null,
      kills: 0,
      deaths: 0,
      assists: 0,
      gold: 0,
      rating: 5,
    })),
  ];

  const resolvedRows = playerResults.length > 0 ? playerResults : fallbackRows;
  const mvp = [...resolvedRows].sort((a, b) => b.rating - a.rating)[0] ?? fallbackRows[0];

  const blueGold = runtime.stats?.blue?.gold ?? 0;
  const redGold = runtime.stats?.red?.gold ?? 0;
  const timelineEvents = (runtime.events ?? [])
    .map((event) => {
      const side = parseRuntimeEventSide(event.text);
      if (!side) return null;
      const type = runtimeEventToDraftType(event);
      return {
        minute: Math.max(0, Math.floor((event.t ?? 0) / 60)),
        side,
        type,
        label: draftTypeLabel(type),
      };
    })
    .filter((item): item is NonNullable<typeof item> => item !== null)
    .slice(-20);

  const finalDelta = blueGold - redGold;
  const timelinePoints = Math.max(8, Math.min(24, Math.floor(durationMinutes / 2) + 1));
  const timelineMinutes = Array.from({ length: timelinePoints }, (_, idx) =>
    Math.floor((idx / Math.max(1, timelinePoints - 1)) * durationMinutes),
  );
  if (timelineMinutes[timelineMinutes.length - 1] !== durationMinutes) {
    timelineMinutes.push(durationMinutes);
  }

  const eventImpact = (type: ReturnType<typeof runtimeEventToDraftType>, side: "blue" | "red") => {
    const sign = side === "blue" ? 1 : -1;
    switch (type) {
      case "first_blood":
        return sign * 450;
      case "voidgrubs":
        return sign * 320;
      case "dragon":
        return sign * 220;
      case "dragon_soul":
        return sign * 700;
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
      case "nexus_turret":
        return sign * 1050;
      case "nexus":
        return sign * 2000;
      default:
        return 0;
    }
  };

  const rawTimeline = timelineMinutes.map((minute, idx) => {
    if (idx === 0) return 0;
    const progress = Math.max(0, Math.min(1, minute / Math.max(1, durationMinutes)));
    const base = finalDelta * Math.pow(progress, 1.08);
    const events = timelineEvents
      .filter((event) => event.minute <= minute)
      .reduce((sum, event) => sum + eventImpact(event.type, event.side), 0);
    const jitter = Math.round((Math.sin(minute + idx) * 0.5 + 0.5) * 120 * (1 - progress));
    return base + events * 0.35 + jitter;
  });

  const rawFirst = rawTimeline[0] ?? 0;
  const rawLast = rawTimeline[rawTimeline.length - 1] ?? 0;
  const scale = Math.abs(rawLast - rawFirst) < 1 ? 1 : (finalDelta / (rawLast - rawFirst));
  const goldDiffTimeline = timelineMinutes.map((minute, idx) => ({
    minute,
    diff: Math.round((rawTimeline[idx] - rawFirst) * scale),
  }));

  const winnerSide = runtime.winner === "red" ? "red" : "blue";

  return {
    winnerSide,
    durationMinutes,
    blueKills,
    redKills,
    mvp,
    playerResults: resolvedRows,
    goldDiffTimeline,
    timelineEvents,
    objectives: {
      blue: {
        voidgrubs: 0,
        dragons: runtime.stats?.blue?.dragons ?? 0,
        dragonSoul: false,
        elderDragons: 0,
        heralds: 0,
        barons: runtime.stats?.blue?.barons ?? 0,
        towers: runtime.stats?.blue?.towers ?? 0,
        inhibitors: 0,
      },
      red: {
        voidgrubs: 0,
        dragons: runtime.stats?.red?.dragons ?? 0,
        dragonSoul: false,
        elderDragons: 0,
        heralds: 0,
        barons: runtime.stats?.red?.barons ?? 0,
        towers: runtime.stats?.red?.towers ?? 0,
        inhibitors: 0,
      },
    },
    power: {
      blue: 50,
      red: 50,
      diff: 0,
      autoWin: false,
      winProbBlue: 50,
    },
  };
}

const PARALLEL_SIMULATION_COUNT = 8;
const PARALLEL_SIM_MAX_TICKS = 3600;
const PARALLEL_SIM_DT_SEC = 0.2;
const PARALLEL_SIM_SPEED = 12;
const PARALLEL_SIMS_CHECKPOINT_KEY = "lol-sim-v2:parallel-sims-checkpoints";

interface ParallelSimsCheckpoint {
  runId: string;
  atIso: string;
  batch: number;
  batchDurationSec: number;
  simsPerHour: number;
  blueWins: number;
  redWins: number;
  unresolved: number;
  totalSims: number;
  totalBlueWins: number;
  totalRedWins: number;
  totalUnresolved: number;
}

function saveParallelSimsCheckpoint(checkpoint: ParallelSimsCheckpoint) {
  if (typeof window === "undefined") return;

  try {
    const raw = window.localStorage.getItem(PARALLEL_SIMS_CHECKPOINT_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    const list = Array.isArray(parsed) ? parsed : [];
    list.push(checkpoint);

    const bounded = list.slice(-1000);
    window.localStorage.setItem(PARALLEL_SIMS_CHECKPOINT_KEY, JSON.stringify(bounded));
  } catch (error) {
    console.warn("[MatchSimulation] parallelSims:checkpointSaveFailed", error);
  }
}

function persistFixtureDraftResult(
  fixtureId: string,
  payload: StoredFixtureDraftResult,
) {
  if (typeof window === "undefined") return;

  try {
    window.localStorage.setItem(
      `fixture-draft-result:${fixtureId}`,
      JSON.stringify(payload),
    );
  } catch (error) {
    console.warn("[MatchSimulation] fixtureResult:saveFailed", {
      error,
      fixtureId,
    });
  }
}

function readSeriesWins(value: unknown): number {
  if (typeof value !== "number" || !Number.isFinite(value)) return 0;
  return Math.max(0, Math.floor(value));
}

export default function MatchSimulation() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const routeState = (location.state as MatchRouteState | null) ?? null;
  const matchMode = routeState?.mode || "live";
  const { gameState, setGameState } = useGameStore();
  const { settings } = useSettingsStore();
  const simPolicy = useMemo<LolSimV1PolicyConfig>(() => ({
    hybridOpenTradeConfidenceHigh: settings.lol_hybrid_open_trade_confidence_high,
    hybridDisengageConfidenceLow: settings.lol_hybrid_disengage_confidence_low,
  }), [settings.lol_hybrid_disengage_confidence_low, settings.lol_hybrid_open_trade_confidence_high]);
  const [snapshot, setSnapshot] = useState<MatchSnapshot | null>(
    routeState?.snapshot ?? null,
  );
  const [stage, setStage] = useState<MatchDayStage>("prematch");
  const [importantEvents, setImportantEvents] = useState<MatchEvent[]>([]);
  const [finalRuntimeState, setFinalRuntimeState] = useState<LolSimV1RuntimeState | null>(null);
  const [draftPayload, setDraftPayload] = useState<ChampionDraftResultPayload | null>(null);
  const [draftResultSimulation, setDraftResultSimulation] = useState<DraftMatchResult | null>(null);
  const [championSelections, setChampionSelections] = useState<ChampionSelectionByPlayer | null>(null);
  const [userSide, setUserSide] = useState<"Home" | "Away" | null>(null);
  const [isSpectator, setIsSpectator] = useState(matchMode === "spectator");
  const [hasFinalizedMatch, setHasFinalizedMatch] = useState(false);
  const [isRunningParallelSims, setIsRunningParallelSims] = useState(false);
  const [parallelSimsFeedback, setParallelSimsFeedback] = useState<string | null>(null);
  const parallelSimsLoopActiveRef = useRef(false);

  useEffect(() => {
    console.info("[MatchSimulation] mount", {
      fixtureIndex: routeState?.fixtureIndex,
      hasGameState: !!gameState,
      hasRouteSnapshot: !!routeState?.snapshot,
      matchMode,
    });
  }, [gameState, matchMode, routeState?.fixtureIndex, routeState?.snapshot]);

  // Determine user side from game state
  useEffect(() => {
    if (!gameState || !snapshot) return;
    const utid = gameState.manager.team_id;
    if (!utid) {
      setIsSpectator(true);
      return;
    }
    if (snapshot.home_team.id === utid) setUserSide("Home");
    else if (snapshot.away_team.id === utid) setUserSide("Away");
    else setIsSpectator(true);

    // If mode is spectator, force spectator regardless of team
    if (matchMode === "spectator") setIsSpectator(true);

    console.info("[MatchSimulation] resolveSide", {
      awayTeamId: snapshot.away_team.id,
      homeTeamId: snapshot.home_team.id,
      matchMode,
      managerTeamId: utid,
      resolvedUserSide:
        snapshot.home_team.id === utid
          ? "Home"
          : snapshot.away_team.id === utid
            ? "Away"
            : null,
    });
  }, [gameState, snapshot?.home_team.id, snapshot?.away_team.id, matchMode]);

  useEffect(() => {
    console.info("[MatchSimulation] stage", {
      hasSnapshot: !!snapshot,
      isSpectator,
      stage,
      userSide,
    });
  }, [isSpectator, snapshot, stage, userSide]);

  useEffect(() => {
    return () => {
      parallelSimsLoopActiveRef.current = false;
    };
  }, []);

  // Fetch initial snapshot
  useEffect(() => {
    let isCancelled = false;

    const fetchSnapshot = async () => {
      console.info("[MatchSimulation] fetchSnapshot:start", {
        fixtureIndex: routeState?.fixtureIndex,
        hasRouteSnapshot: !!routeState?.snapshot,
        matchMode,
      });
      try {
        const snap = await invoke<MatchSnapshot>("get_match_snapshot");
        console.info("[MatchSimulation] fetchSnapshot:success", {
          awayPlayers: snap.away_team.players.length,
          awayTeam: snap.away_team.name,
          homePlayers: snap.home_team.players.length,
          homeTeam: snap.home_team.name,
          phase: snap.phase,
        });
        if (!isCancelled) {
          setSnapshot(snap);
        }
        return;
      } catch (snapshotError) {
        console.warn("[MatchSimulation] fetchSnapshot:failed", snapshotError);
        if (typeof routeState?.fixtureIndex !== "number") {
          console.error("Failed to get match snapshot:", snapshotError);
          navigate("/dashboard");
          return;
        }

        try {
          console.info("[MatchSimulation] restoreLiveMatch:start", {
            fixtureIndex: routeState.fixtureIndex,
            matchMode,
          });
          const restoredSnapshot = await invoke<MatchSnapshot>(
            "start_live_match",
            {
              allowsExtraTime: false,
              fixtureIndex: routeState.fixtureIndex,
              mode: matchMode,
            },
          );

          console.info("[MatchSimulation] restoreLiveMatch:success", {
            awayPlayers: restoredSnapshot.away_team.players.length,
            awayTeam: restoredSnapshot.away_team.name,
            homePlayers: restoredSnapshot.home_team.players.length,
            homeTeam: restoredSnapshot.home_team.name,
            phase: restoredSnapshot.phase,
          });

          if (!isCancelled) {
            setSnapshot(restoredSnapshot);
          }
        } catch (restoreError) {
          console.error("Failed to restore live match session:", restoreError);
          navigate("/dashboard");
        }
      }
    };

    fetchSnapshot();

    return () => {
      isCancelled = true;
    };
  }, [matchMode, navigate, routeState?.fixtureIndex]);

  // Skip pre-match for spectators
  useEffect(() => {
    if (isSpectator && stage === "prematch") {
      setStage("draft");
    }
  }, [isSpectator, stage]);

  const currentFixture =
    gameState && snapshot
      ? resolveMatchFixture(gameState, snapshot, routeState?.fixtureIndex)
      : null;

  const playoffFixtures =
    gameState?.league?.fixtures.filter(
      (fixture) => fixture.competition === "Playoffs",
    ) ?? [];

  const playoffFinalMatchday =
    playoffFixtures.length > 0
      ? Math.max(...playoffFixtures.map((fixture) => fixture.matchday))
      : null;

  const seriesLength: 1 | 3 | 5 =
    currentFixture?.competition !== "Playoffs"
      ? 1
      : playoffFinalMatchday !== null &&
          currentFixture.matchday >= playoffFinalMatchday
        ? 5
        : 3;

  const normalizeTeamKey = (value: string): string =>
    value.trim().toLowerCase().replace(/[^a-z0-9]/g, "");

  const managerTeamId = gameState?.manager.team_id ?? null;
  const managerTeamName =
    gameState?.teams.find((team) => team.id === managerTeamId)?.name ?? null;

  const defaultControlledDraftSide: "blue" | "red" = (() => {
    if (!snapshot) return "blue";
    if (managerTeamId && managerTeamId === snapshot.away_team.id) return "red";
    if (managerTeamId && managerTeamId === snapshot.home_team.id) return "blue";

    if (managerTeamName) {
      const managerNameKey = normalizeTeamKey(managerTeamName);
      if (managerNameKey === normalizeTeamKey(snapshot.away_team.name)) return "red";
      if (managerNameKey === normalizeTeamKey(snapshot.home_team.name)) return "blue";
    }

    if (userSide === "Away") return "red";
    return "blue";
  })();

  const [userSelectedSide, setUserSelectedSide] = useState<"blue" | "red">(
    defaultControlledDraftSide,
  );

  useEffect(() => {
    setUserSelectedSide(defaultControlledDraftSide);
  }, [defaultControlledDraftSide]);

  const swapSnapshotSides = useCallback((snap: MatchSnapshot): MatchSnapshot => {
    return {
      ...snap,
      home_team: snap.away_team,
      away_team: snap.home_team,
      home_bench: snap.away_bench,
      away_bench: snap.home_bench,
    };
  }, []);

  const activeSnapshot = useMemo(() => {
    if (!snapshot || !managerTeamId) return snapshot;

    const isUserHome = managerTeamId === snapshot.home_team.id;
    const shouldBeBlue = userSelectedSide === "blue";

    if ((isUserHome && shouldBeBlue) || (!isUserHome && !shouldBeBlue)) {
      return snapshot;
    }

    return swapSnapshotSides(snapshot);
  }, [managerTeamId, snapshot, swapSnapshotSides, userSelectedSide]);

  const renderSnapshot = activeSnapshot ?? snapshot;
  const renderSnapshotWithTactics = useMemo(() => {
    if (!renderSnapshot || !gameState) return renderSnapshot;
    return attachLolTacticsToSnapshot(renderSnapshot, gameState);
  }, [gameState, renderSnapshot]);

  // Callbacks for stage transitions
  const handleStartMatch = useCallback(() => {
    console.info("[MatchSimulation] handleStartMatch");
    setStage("draft");
  }, []);

  const handleDraftComplete = useCallback((_payload: ChampionDraftResultPayload) => {
    console.info("[MatchSimulation] handleDraftComplete");
    const payload = _payload;
    setDraftPayload(payload);
    if (activeSnapshot) {
      const roles = ["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"] as const;
      const inferRole = (position: string): typeof roles[number] => {
        const p = position.toLowerCase();
        if (p.includes("top")) return "TOP";
        if (p.includes("jung")) return "JUNGLE";
        if (p.includes("mid")) return "MID";
        if (p.includes("adc") || p.includes("bot") || p.includes("carry")) return "ADC";
        return "SUPPORT";
      };

      const mapSide = (
        players: MatchSnapshot["home_team"]["players"],
        picks: ChampionDraftResultPayload["blue"]["picks"],
      ): {
        champions: Record<string, string>;
        roles: Record<string, "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT">;
      } => {
        const champions: Record<string, string> = {};
        const roleByPlayer: Record<string, "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT"> = {};
        const roleOrder: Record<typeof roles[number], number> = {
          TOP: 0,
          JUNGLE: 1,
          MID: 2,
          ADC: 3,
          SUPPORT: 4,
        };
        const usedPlayerIds = new Set<string>();
        for (const role of roles) {
          const pick = picks.find((entry) => entry.role === role);
          if (!pick) continue;

          const exact = players.find(
            (entry) => !usedPlayerIds.has(entry.id) && inferRole(entry.position) === role,
          );
          const slot = players[roleOrder[role]];
          const slotCandidate = slot && !usedPlayerIds.has(slot.id) ? slot : null;
          const fallback = players.find((entry) => !usedPlayerIds.has(entry.id)) ?? players[0];
          const player = exact ?? slotCandidate ?? fallback;

          if (player) {
            usedPlayerIds.add(player.id);
            champions[player.id] = pick.championId;
            roleByPlayer[player.id] = role;
          }
        }
        return { champions, roles: roleByPlayer };
      };

      const homeDraft = mapSide(activeSnapshot.home_team.players, payload.blue.picks);
      const awayDraft = mapSide(activeSnapshot.away_team.players, payload.red.picks);

      setChampionSelections({
        home: homeDraft.champions,
        away: awayDraft.champions,
        homeRoles: homeDraft.roles,
        awayRoles: awayDraft.roles,
      });
    }
    setStage("tactics");
  }, [activeSnapshot]);

  const handleContinueFromTactics = useCallback(() => {
    console.info("[MatchSimulation] handleContinueFromTactics");
    setStage("first_half");
  }, []);

  const runSingleParallelSimulation = useCallback(async (
    runIndex: number,
    runSnapshot: MatchSnapshot,
    championMapByPlayerId: Record<string, string>,
    runSeedBase: string,
  ): Promise<LolSimV1RuntimeState["winner"]> => {
    const response = await lolSimV2RunToCompletion({
      seed: `${runSeedBase}-${runIndex + 1}`,
      aiMode: "hybrid",
      policy: simPolicy,
      snapshot: runSnapshot,
      championByPlayerId: championMapByPlayerId,
      championProfilesById: {},
      dtSec: PARALLEL_SIM_DT_SEC,
      speed: PARALLEL_SIM_SPEED,
      maxTicks: PARALLEL_SIM_MAX_TICKS,
    });

    return response.winner ?? null;
  }, [simPolicy]);

  const handleRunParallelSims = useCallback(async () => {
    if (!renderSnapshotWithTactics) {
      return;
    }

    if (isRunningParallelSims) {
      parallelSimsLoopActiveRef.current = false;
      setParallelSimsFeedback(
        t("match.parallelSimsStopping", { defaultValue: "Deteniendo simulaciones..." }),
      );
      return;
    }

    setIsRunningParallelSims(true);
    parallelSimsLoopActiveRef.current = true;
    setParallelSimsFeedback(
      t("match.parallelSimsPreparing", { defaultValue: "Preparando simulaciones..." }),
    );

    const championMapByPlayerId: Record<string, string> = {
      ...(championSelections?.home ?? {}),
      ...(championSelections?.away ?? {}),
    };
    const runSeedBase = `post-draft-${Date.now()}`;
    const runId = `parallel-sims-${Date.now()}`;

    try {
      const clearResult = await lolSimV2ClearTelemetryFiles();
      console.info("[MatchSimulation] parallelSims:telemetryCleared", clearResult);

      let batch = 0;
      let totalSims = 0;
      let totalBlueWins = 0;
      let totalRedWins = 0;
      let totalUnresolved = 0;

      while (parallelSimsLoopActiveRef.current) {
        batch += 1;
        setParallelSimsFeedback(
          t("match.parallelSimsRunning", {
            defaultValue: `Corriendo 8 simulaciones en paralelo... (Lote ${batch})`,
          }),
        );

        const batchStartedAt = performance.now();
        const winners = await Promise.all(
          Array.from({ length: PARALLEL_SIMULATION_COUNT }, (_, index) =>
            runSingleParallelSimulation(
              index + batch * 1000,
              renderSnapshotWithTactics,
              championMapByPlayerId,
              runSeedBase,
            ),
          ),
        );
        const batchDurationSeconds = (performance.now() - batchStartedAt) / 1_000;
        const approxSimsPerHour = batchDurationSeconds > 0
          ? Math.round((winners.length * 3_600) / batchDurationSeconds)
          : 0;

        const blueWins = winners.filter((winner) => winner === "blue").length;
        const redWins = winners.filter((winner) => winner === "red").length;
        const unresolved = winners.length - blueWins - redWins;

        totalSims += winners.length;
        totalBlueWins += blueWins;
        totalRedWins += redWins;
        totalUnresolved += unresolved;

        saveParallelSimsCheckpoint({
          runId,
          atIso: new Date().toISOString(),
          batch,
          batchDurationSec: Number(batchDurationSeconds.toFixed(3)),
          simsPerHour: approxSimsPerHour,
          blueWins,
          redWins,
          unresolved,
          totalSims,
          totalBlueWins,
          totalRedWins,
          totalUnresolved,
        });

        setParallelSimsFeedback(
          t("match.parallelSimsDone", {
            defaultValue: `Lote ${batch} listo en ${batchDurationSeconds.toFixed(1)}s · Azul ${blueWins} / Rojo ${redWins} / Sin ganador ${unresolved} · ~${approxSimsPerHour} sims/h · Total ${totalSims}`,
          }),
        );
      }

      setParallelSimsFeedback(
        t("match.parallelSimsStopped", {
          defaultValue: "Simulaciones detenidas.",
        }),
      );
    } catch (error) {
      console.error("[MatchSimulation] parallelSims:failed", error);
      setParallelSimsFeedback(
        t("match.parallelSimsFailed", {
          defaultValue: "No se pudieron ejecutar las 8 simulaciones.",
        }),
      );
    } finally {
      parallelSimsLoopActiveRef.current = false;
      setIsRunningParallelSims(false);
    }
  }, [
    championSelections?.away,
    championSelections?.home,
    isRunningParallelSims,
    renderSnapshotWithTactics,
    runSingleParallelSimulation,
    t,
  ]);

  const finalizeMatch = useCallback(async (lolReport?: LolSimV1MatchReportInput): Promise<boolean> => {
    if (hasFinalizedMatch) {
      return true;
    }

    try {
      console.info("[MatchSimulation] finalizeMatch:start");
      const response =
        await invoke<FinishLiveMatchResponse>("finish_live_match", { lolReport });
      console.info("[MatchSimulation] finalizeMatch:success", {
        hasRoundSummary: !!response.round_summary,
        hasUpdatedGame: !!response.game,
      });
      setGameState(response.game);
      setHasFinalizedMatch(true);
      return true;
    } catch (err) {
      console.error("Failed to finish match:", err);
      return false;
    }
  }, [hasFinalizedMatch, setGameState]);

  const handleFullTime = useCallback((finalRuntimeState: LolSimV1RuntimeState, meta?: { source: "live" | "skip" }) => {
    console.info("[MatchSimulation] handleFullTime");
    const source = meta?.source ?? "live";
    setFinalRuntimeState(finalRuntimeState);
    const mappedEvents: MatchEvent[] = (finalRuntimeState.events ?? []).map((event) => ({
      minute: Math.max(0, Math.floor((event.t ?? 0) / 60)),
      event_type: event.type,
      side: event.text?.toUpperCase().includes("RED") ? "Away" : "Home",
      zone: "mid",
      player_id: null,
      secondary_player_id: null,
    }));
    setImportantEvents(mappedEvents);

    const safeDraftPayload = normalizeDraftPayload(draftPayload, championSelections, renderSnapshotWithTactics ?? null);

    const snapshotForResult = renderSnapshotWithTactics ?? snapshot;
    if (!snapshotForResult) {
      void (async () => {
        const finalized = await finalizeMatch(buildLolMatchReport(finalRuntimeState));
        if (finalized) {
          setStage("draft_result");
        }
      })();
      return;
    }

    const runtimeBasedResult = buildDraftResultFromRuntime({
      runtime: finalRuntimeState,
      snapshot: snapshotForResult,
      championSelections,
    });

    let resultToPersist = runtimeBasedResult;

    let simulatedForSkip: DraftMatchResult | null = null;

    if (source === "skip" && safeDraftPayload && renderSnapshotWithTactics && gameState) {
      try {
        const simulated = simulateDraftMatchResult({
          snapshot: renderSnapshotWithTactics,
          gameState,
          draft: safeDraftPayload,
        });
        setDraftResultSimulation(simulated);
        resultToPersist = simulated;
        simulatedForSkip = simulated;
      } catch (error) {
        console.error("[MatchSimulation] draftResultFallback:failed", error);
        setDraftResultSimulation(null);
      }
    } else {
      setDraftResultSimulation(null);
    }

    if (currentFixture?.id) {
      const targetSeriesWins = seriesLength === 1 ? 1 : seriesLength === 3 ? 2 : 3;
      const existingHomeWins = Math.min(
        targetSeriesWins,
        readSeriesWins(currentFixture.result?.home_wins) ||
          readSeriesWins(currentFixture.result?.home_goals),
      );
      const existingAwayWins = Math.min(
        targetSeriesWins,
        readSeriesWins(currentFixture.result?.away_wins) ||
          readSeriesWins(currentFixture.result?.away_goals),
      );

      const winnerTeamId =
        resultToPersist.winnerSide === "blue"
          ? snapshotForResult.home_team.id
          : snapshotForResult.away_team.id;

      const homeSeriesWins = Math.min(
        targetSeriesWins,
        winnerTeamId === currentFixture.home_team_id
          ? existingHomeWins + 1
          : existingHomeWins,
      );
      const awaySeriesWins = Math.min(
        targetSeriesWins,
        winnerTeamId === currentFixture.away_team_id
          ? existingAwayWins + 1
          : existingAwayWins,
      );
      const managerTeamId = gameState?.manager.team_id ?? null;
      const userSeriesWins =
        managerTeamId === currentFixture.home_team_id
          ? homeSeriesWins
          : managerTeamId === currentFixture.away_team_id
            ? awaySeriesWins
            : 0;
      const opponentSeriesWins =
        managerTeamId === currentFixture.home_team_id
          ? awaySeriesWins
          : managerTeamId === currentFixture.away_team_id
            ? homeSeriesWins
            : 0;

      persistFixtureDraftResult(currentFixture.id, {
        snapshot: snapshotForResult,
        controlledSide: userSelectedSide,
        result: resultToPersist,
        seriesLength,
        seriesGameIndex: homeSeriesWins + awaySeriesWins,
        userSeriesWins,
        opponentSeriesWins,
        homeSeriesWins,
        awaySeriesWins,
      });
    }

    const runtimeForFinalize = simulatedForSkip
      ? {
        ...finalRuntimeState,
        winner: simulatedForSkip.winnerSide,
        timeSec: simulatedForSkip.durationMinutes * 60,
        stats: {
          ...(finalRuntimeState.stats ?? {
            blue: { kills: 0, towers: 0, dragons: 0, barons: 0, gold: 0 },
            red: { kills: 0, towers: 0, dragons: 0, barons: 0, gold: 0 },
          }),
          blue: {
            ...(finalRuntimeState.stats?.blue ?? { kills: 0, towers: 0, dragons: 0, barons: 0, gold: 0 }),
            kills: simulatedForSkip.blueKills,
          },
          red: {
            ...(finalRuntimeState.stats?.red ?? { kills: 0, towers: 0, dragons: 0, barons: 0, gold: 0 }),
            kills: simulatedForSkip.redKills,
          },
        },
      }
      : finalRuntimeState;

    void (async () => {
      const finalized = await finalizeMatch(buildLolMatchReport(runtimeForFinalize));
      if (finalized) {
        setStage("draft_result");
      }
    })();
  }, [
    championSelections,
    currentFixture?.id,
    draftPayload,
    finalizeMatch,
    gameState,
    renderSnapshotWithTactics,
    seriesLength,
    snapshot,
    userSelectedSide,
  ]);

  const handlePressConference = useCallback(() => {
    console.info("[MatchSimulation] handlePressConference");
    setStage("press");
  }, []);

  const handleFinishMatch = useCallback(async () => {
    console.info("[MatchSimulation] handleFinishMatch:start");
    const finalized = await finalizeMatch();
    if (finalized) {
      navigate("/dashboard");
    }
  }, [finalizeMatch, navigate]);

  const handleSnapshotUpdate = useCallback((snap: MatchSnapshot) => {
    console.info("[MatchSimulation] handleSnapshotUpdate", {
      awayPlayers: snap.away_team.players.length,
      currentMinute: snap.current_minute,
      homePlayers: snap.home_team.players.length,
      phase: snap.phase,
    });
    setSnapshot(snap);
  }, []);

  const handleImportantEvent = useCallback((evt: MatchEvent) => {
    console.info("[MatchSimulation] handleImportantEvent", {
      eventType: evt.event_type,
      minute: evt.minute,
      side: evt.side,
    });
    setImportantEvents((prev) => [...prev, evt]);
  }, []);

  // Loading state
  if (!snapshot || !gameState) {
    return (
      <div className="min-h-screen bg-gray-100 dark:bg-navy-900 flex items-center justify-center transition-colors duration-300">
        <div className="flex flex-col items-center gap-3">
          <div className="w-8 h-8 border-4 border-primary-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider text-sm">
            {t("dashboard.loading")}
          </span>
        </div>
      </div>
    );
  }

  // Render the current stage
  switch (stage) {
    case "prematch":
      return (
        <PreMatchSetup
          snapshot={snapshot}
          gameState={gameState}
          currentFixture={currentFixture}
          userSide={userSide || "Home"}
          onStart={handleStartMatch}
          onUpdateSnapshot={handleSnapshotUpdate}
        />
      );

    case "draft":
      return (
        <ChampionDraft
          snapshot={renderSnapshotWithTactics ?? renderSnapshot ?? snapshot}
          onComplete={handleDraftComplete}
          controlledSide={userSelectedSide}
          seriesLength={seriesLength}
          blueSeriesWins={0}
          redSeriesWins={0}
          gameState={gameState}
        />
      );

    case "tactics":
      return (
        <MatchTacticsStage
          gameState={gameState}
          onGameUpdate={setGameState}
          onContinue={handleContinueFromTactics}
          onRunParallelSims={handleRunParallelSims}
          isRunningParallelSims={isRunningParallelSims}
          parallelSimsFeedback={parallelSimsFeedback}
        />
      );

    case "draft_result":
      {
        const runtimeBasedResult = finalRuntimeState
          ? buildDraftResultFromRuntime({
            runtime: finalRuntimeState,
            snapshot: renderSnapshotWithTactics ?? snapshot,
            championSelections,
          })
          : null;
        const draftScreenResult = draftResultSimulation ?? runtimeBasedResult;

        if (draftScreenResult) {
          return (
            <DraftResultScreen
              snapshot={renderSnapshotWithTactics ?? snapshot}
              controlledSide={userSelectedSide}
              result={draftScreenResult}
              seriesLength={seriesLength}
              onPressConference={handlePressConference}
              onContinue={(nextUserSide) => {
                if (nextUserSide) {
                  setUserSelectedSide(nextUserSide);
                }
                void handleFinishMatch();
              }}
            />
          );
        }

        return (
          <LolResultScreen
            snapshot={renderSnapshotWithTactics ?? snapshot}
            gameState={gameState}
            currentFixture={currentFixture}
            userSide={userSide}
            importantEvents={importantEvents}
            finalRuntimeState={finalRuntimeState}
            onPressConference={handlePressConference}
            onFinish={handleFinishMatch}
          />
        );
      }

    case "postmatch":
      return (
        <LolResultScreen
          snapshot={snapshot}
          gameState={gameState}
          currentFixture={currentFixture}
          userSide={userSide}
          importantEvents={importantEvents}
          finalRuntimeState={finalRuntimeState}
          onPressConference={handlePressConference}
          onFinish={handleFinishMatch}
        />
      );

    case "first_half":
      return (
        <LolMatchLive
          key={stage}
          snapshot={renderSnapshotWithTactics ?? snapshot}
          championSelections={championSelections}
          onSnapshotUpdate={handleSnapshotUpdate}
          onImportantEvent={handleImportantEvent}
          onFullTime={handleFullTime}
        />
      );

    case "press":
      return (
        <PressConference
          snapshot={snapshot}
          gameState={gameState}
          userSide={userSide || "Home"}
          onFinish={handleFinishMatch}
          onGameUpdate={setGameState}
        />
      );

    default:
      return null;
  }
}
