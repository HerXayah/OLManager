import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import DraftResultScreen, { type DraftResultSeriesGame } from "./DraftResultScreen";
import type { DraftMatchResult } from "./draftResultSimulator";
import type { MatchSnapshot } from "./types";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: string | { defaultValue?: string }) => {
      if (typeof options === "string") {
        return options;
      }

      if (options && typeof options === "object" && "defaultValue" in options) {
        return options.defaultValue ?? key;
      }

      return key;
    },
  }),
}));

function createResult(overrides: Partial<DraftMatchResult> = {}): DraftMatchResult {
  return {
    winnerSide: "blue",
    durationMinutes: 32,
    blueKills: 15,
    redKills: 9,
    mvp: {
      side: "blue",
      playerId: "blue-top",
      playerName: "Blue Top",
      role: "TOP",
      championId: "Aatrox",
      kills: 7,
      deaths: 1,
      assists: 6,
      gold: 14000,
      rating: 9.2,
    },
    playerResults: [],
    goldDiffTimeline: [],
    timelineEvents: [],
    objectives: {
      blue: {
        voidgrubs: 0,
        dragons: 2,
        dragonSoul: false,
        elderDragons: 0,
        heralds: 1,
        barons: 1,
        towers: 8,
        inhibitors: 2,
      },
      red: {
        voidgrubs: 0,
        dragons: 1,
        dragonSoul: false,
        elderDragons: 0,
        heralds: 0,
        barons: 0,
        towers: 3,
        inhibitors: 0,
      },
    },
    power: {
      blue: 70,
      red: 55,
      diff: 15,
      autoWin: false,
      winProbBlue: 61,
    },
    ...overrides,
  };
}

const snapshot = {
  home_team: { id: "team-1", name: "Alpha FC", players: [] },
  away_team: { id: "team-2", name: "Beta FC", players: [] },
} as MatchSnapshot;

describe("DraftResultScreen", () => {
  it("renders game tabs and switches displayed game result", () => {
    const gameOne = createResult({
      blueKills: 22,
      redKills: 14,
      mvp: {
        side: "blue",
        playerId: "alpha-mid",
        playerName: "Alpha Mid",
        role: "MID",
        championId: "Ahri",
        kills: 12,
        deaths: 2,
        assists: 7,
        gold: 16800,
        rating: 9.7,
      },
    });

    const gameTwo = createResult({
      blueKills: 9,
      redKills: 3,
      mvp: {
        side: "blue",
        playerId: "alpha-jungle",
        playerName: "Alpha Jungle",
        role: "JUNGLE",
        championId: "LeeSin",
        kills: 6,
        deaths: 1,
        assists: 8,
        gold: 13200,
        rating: 9.1,
      },
    });

    const seriesGames: DraftResultSeriesGame[] = [
      { gameIndex: 1, result: gameOne, winnerSide: gameOne.winnerSide },
      { gameIndex: 2, result: gameTwo, winnerSide: gameTwo.winnerSide },
    ];

    render(
      <DraftResultScreen
        snapshot={snapshot}
        controlledSide="blue"
        result={gameTwo}
        seriesGames={seriesGames}
        seriesLength={3}
        seriesGameIndex={2}
        userSeriesWins={2}
        opponentSeriesWins={0}
        onContinue={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: "Game 1" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Game 2" })).toBeInTheDocument();
    expect(screen.getAllByText("Alpha Jungle").length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "Game 1" }));

    expect(screen.getAllByText("Alpha Mid").length).toBeGreaterThan(0);
  });
});
