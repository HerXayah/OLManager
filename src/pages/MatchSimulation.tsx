import { useEffect, useState, useCallback, useMemo, useRef } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { useGameStore, GameStateData } from "../store/gameStore";
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
import PressConference from "../components/match/PressConference";
import {
  lolSimV2ClearTelemetryFiles,
  lolSimV2RunToCompletion,
} from "../components/match/lol-prototype/backend/tauri-client";
import type { LolSimV1PolicyConfig, LolSimV1RuntimeState } from "../components/match/lol-prototype/backend/contract-v1";

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

  // Callbacks for stage transitions
  const handleStartMatch = useCallback(() => {
    console.info("[MatchSimulation] handleStartMatch");
    setStage("draft");
  }, []);

  const handleDraftComplete = useCallback((_payload: ChampionDraftResultPayload) => {
    console.info("[MatchSimulation] handleDraftComplete");
    const payload = _payload;
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
    if (!renderSnapshot) {
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
              renderSnapshot,
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
    renderSnapshot,
    runSingleParallelSimulation,
    t,
  ]);

  const finalizeMatch = useCallback(async (): Promise<boolean> => {
    if (hasFinalizedMatch) {
      return true;
    }

    try {
      console.info("[MatchSimulation] finalizeMatch:start");
      const response =
        await invoke<FinishLiveMatchResponse>("finish_live_match");
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

  const handleFullTime = useCallback(() => {
    console.info("[MatchSimulation] handleFullTime");
    void (async () => {
      const finalized = await finalizeMatch();
      if (finalized) {
        setStage("draft_result");
      }
    })();
  }, [finalizeMatch]);

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
          snapshot={renderSnapshot ?? snapshot}
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
    case "postmatch":
      return (
        <LolResultScreen
          snapshot={snapshot}
          gameState={gameState}
          currentFixture={currentFixture}
          userSide={userSide}
          importantEvents={importantEvents}
          onPressConference={handlePressConference}
          onFinish={handleFinishMatch}
        />
      );

    case "first_half":
      return (
        <LolMatchLive
          key={stage}
          snapshot={snapshot}
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
