import { useEffect, useState, useCallback, useMemo } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { useGameStore, GameStateData } from "../store/gameStore";
import {
  MatchSnapshot,
  MatchEvent,
  MatchDayStage,
  RoundSummary,
} from "../components/match/types";
import { resolveMatchFixture } from "../components/match/helpers";
import PreMatchSetup from "../components/match/PreMatchSetup";
import ChampionDraft from "../components/match/ChampionDraft";
import type { ChampionDraftResultPayload } from "../components/match/ChampionDraft";
import MatchLive from "../components/match/MatchLive";
import HalfTimeBreak from "../components/match/HalfTimeBreak";
import PostMatchScreen from "../components/match/PostMatchScreen";
import PressConference from "../components/match/PressConference";
import DraftResultScreen from "../components/match/DraftResultScreen";
import {
  simulateDraftMatchResult,
  type DraftMatchResult,
} from "../components/match/draftResultSimulator";

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
  round_summary?: RoundSummary | null;
}

export default function MatchSimulation() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const routeState = (location.state as MatchRouteState | null) ?? null;
  const matchMode = routeState?.mode || "live";
  const { gameState, setGameState } = useGameStore();
  const [snapshot, setSnapshot] = useState<MatchSnapshot | null>(
    routeState?.snapshot ?? null,
  );
  const [stage, setStage] = useState<MatchDayStage>("prematch");
  const [importantEvents, setImportantEvents] = useState<MatchEvent[]>([]);
  const [userSide, setUserSide] = useState<"Home" | "Away" | null>(null);
  const [isSpectator, setIsSpectator] = useState(matchMode === "spectator");
  const [roundSummary, setRoundSummary] = useState<RoundSummary | null>(null);
  const [draftResult, setDraftResult] = useState<DraftMatchResult | null>(null);
  const [hasFinalizedMatch, setHasFinalizedMatch] = useState(false);
  const [seriesGameIndex, setSeriesGameIndex] = useState(1);
  const [userSeriesWins, setUserSeriesWins] = useState(0);
  const [opponentSeriesWins, setOpponentSeriesWins] = useState(0);

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
      setStage("first_half");
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

  const seriesTargetWins =
    seriesLength === 5 ? 3 : seriesLength === 3 ? 2 : 1;

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

  useEffect(() => {
    setSeriesGameIndex(1);
    setUserSeriesWins(0);
    setOpponentSeriesWins(0);
    setDraftResult(null);
  }, [routeState?.fixtureIndex]);

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

  const blueSeriesWinsForUi =
    userSelectedSide === "blue" ? userSeriesWins : opponentSeriesWins;
  const redSeriesWinsForUi =
    userSelectedSide === "red" ? userSeriesWins : opponentSeriesWins;

  const loserIsUserInCurrentGame =
    !!draftResult &&
    !!managerTeamId &&
    !!activeSnapshot &&
    ((draftResult.winnerSide === "blue"
      ? activeSnapshot.away_team.id
      : activeSnapshot.home_team.id) === managerTeamId);

  // Callbacks for stage transitions
  const handleStartMatch = useCallback(() => {
    console.info("[MatchSimulation] handleStartMatch");
    setStage("draft");
  }, []);

  const handleDraftComplete = useCallback((payload?: ChampionDraftResultPayload) => {
    console.info("[MatchSimulation] handleDraftComplete");
    if (!payload || !gameState || !activeSnapshot) {
      setStage("first_half");
      return;
    }

    const result = simulateDraftMatchResult({
      snapshot: activeSnapshot,
      gameState,
      draft: payload,
    });
    setDraftResult(result);
    setStage("draft_result");
  }, [activeSnapshot, gameState]);

  const handleHalfTime = useCallback(() => {
    console.info("[MatchSimulation] handleHalfTime");
    setStage("halftime");
  }, []);

  const handleResumeFromHalfTime = useCallback(() => {
    console.info("[MatchSimulation] handleResumeFromHalfTime");
    setStage("second_half");
  }, []);

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
      setRoundSummary(response.round_summary ?? null);
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
        setStage("postmatch");
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

  const handleContinueFromDraftResult = useCallback((nextUserSide?: "blue" | "red") => {
    if (!draftResult || !managerTeamId || !activeSnapshot) return;

    const winnerTeamId =
      draftResult.winnerSide === "blue"
        ? activeSnapshot.home_team.id
        : activeSnapshot.away_team.id;
    const loserTeamId =
      draftResult.winnerSide === "blue"
        ? activeSnapshot.away_team.id
        : activeSnapshot.home_team.id;

    const projectedUserWins =
      userSeriesWins + (winnerTeamId === managerTeamId ? 1 : 0);
    const projectedOpponentWins =
      opponentSeriesWins + (winnerTeamId === managerTeamId ? 0 : 1);

    setUserSeriesWins(projectedUserWins);
    setOpponentSeriesWins(projectedOpponentWins);

    const seriesFinished =
      projectedUserWins >= seriesTargetWins ||
      projectedOpponentWins >= seriesTargetWins;

    if (!seriesFinished) {
      const aiSidePreference =
        Math.abs(draftResult.power.diff) < 1.5
          ? (Math.random() < 0.5 ? "blue" : "red")
          : draftResult.power.winProbBlue >= 50
            ? "blue"
            : "red";

      const computedNextSide =
        loserTeamId === managerTeamId
          ? (nextUserSide ?? userSelectedSide)
          : aiSidePreference === "blue"
            ? "red"
            : "blue";

      setUserSelectedSide(computedNextSide);
      setSeriesGameIndex((prev) => prev + 1);
      setDraftResult(null);
      setStage("draft");
      return;
    }

    void (async () => {
      try {
        await invoke<MatchEvent[]>("step_live_match", { minutes: 200 });
      } catch (err) {
        console.warn("[MatchSimulation] draft_result step_live_match failed", err);
      }

      const finalized = await finalizeMatch();
      if (!finalized) return;

      if (currentFixture?.id) {
        try {
          localStorage.setItem(
            `fixture-draft-result:${currentFixture.id}`,
            JSON.stringify({
              snapshot: activeSnapshot,
              controlledSide: userSelectedSide,
              result: draftResult,
              seriesLength,
              seriesGameIndex,
              userSeriesWins: projectedUserWins,
              opponentSeriesWins: projectedOpponentWins,
            }),
          );
        } catch {
          // ignore persistence failures
        }

        const winnerTeamId =
          draftResult.winnerSide === "blue"
            ? activeSnapshot.home_team.id
            : activeSnapshot.away_team.id;

        const championPicks = draftResult.playerResults
          .filter((entry) => !!entry.championId)
          .map((entry) => ({
            playerId: entry.playerId,
            championId: entry.championId as string,
          }));

        if (championPicks.length > 0) {
          try {
            const updated = await invoke<GameStateData>("record_fixture_champion_picks", {
              fixtureId: currentFixture.id,
              winnerTeamId,
              picks: championPicks,
            });
            setGameState(updated);
          } catch (err) {
            console.warn("[MatchSimulation] record_fixture_champion_picks failed", err);
          }
        }
      }

      navigate("/dashboard");
    })();
  }, [
    activeSnapshot,
    currentFixture?.id,
    draftResult,
    finalizeMatch,
    managerTeamId,
    navigate,
    opponentSeriesWins,
    seriesTargetWins,
    setGameState,
    userSelectedSide,
    userSeriesWins,
  ]);

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

  const renderSnapshot = activeSnapshot ?? snapshot;

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
          snapshot={renderSnapshot}
          onComplete={handleDraftComplete}
          controlledSide={userSelectedSide}
          seriesLength={seriesLength}
          blueSeriesWins={blueSeriesWinsForUi}
          redSeriesWins={redSeriesWinsForUi}
          gameState={gameState}
        />
      );

    case "draft_result":
      if (!draftResult) return null;
      return (
        <DraftResultScreen
          snapshot={renderSnapshot}
          controlledSide={userSelectedSide}
          result={draftResult}
          seriesLength={seriesLength}
          seriesGameIndex={seriesGameIndex}
          userSeriesWins={userSeriesWins}
          opponentSeriesWins={opponentSeriesWins}
          canUserChooseSide={
            seriesLength > 1 &&
            loserIsUserInCurrentGame &&
            userSeriesWins < seriesTargetWins &&
            opponentSeriesWins < seriesTargetWins
          }
          onContinue={handleContinueFromDraftResult}
        />
      );

    case "first_half":
    case "second_half":
      return (
        <MatchLive
          key={stage}
          snapshot={snapshot}
          gameState={gameState}
          userSide={userSide}
          isSpectator={isSpectator}
          importantEvents={importantEvents}
          onSnapshotUpdate={handleSnapshotUpdate}
          onImportantEvent={handleImportantEvent}
          onHalfTime={handleHalfTime}
          onFullTime={handleFullTime}
        />
      );

    case "halftime":
      return (
        <HalfTimeBreak
          snapshot={snapshot}
          gameState={gameState}
          userSide={userSide || "Home"}
          isSpectator={isSpectator}
          importantEvents={importantEvents}
          onResume={handleResumeFromHalfTime}
          onUpdateSnapshot={handleSnapshotUpdate}
        />
      );

    case "postmatch":
      return (
        <PostMatchScreen
          snapshot={snapshot}
          gameState={gameState}
          currentFixture={currentFixture}
          userSide={userSide}
          isSpectator={isSpectator}
          importantEvents={importantEvents}
          roundSummary={roundSummary}
          onPressConference={handlePressConference}
          onFinish={handleFinishMatch}
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
