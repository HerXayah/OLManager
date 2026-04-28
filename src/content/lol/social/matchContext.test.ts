import { describe, expect, it } from "vitest";
import { extractMatchContext } from "./matchContext";
import type { DraftMatchResult } from "../../../components/match/draftResultSimulator";

const baseResult = (overrides: Partial<DraftMatchResult> = {}): DraftMatchResult => ({
  winnerSide: "blue",
  durationMinutes: 31,
  blueKills: 18,
  redKills: 9,
  mvp: {
    side: "blue",
    playerId: "blue-mid",
    playerName: "Blue Mid",
    role: "MID",
    championId: "orianna",
    kills: 8,
    deaths: 1,
    assists: 7,
    gold: 14500,
    rating: 9.1,
  },
  playerResults: [
    {
      side: "blue",
      playerId: "blue-top",
      playerName: "Blue Top",
      role: "TOP",
      championId: "ksante",
      kills: 2,
      deaths: 1,
      assists: 8,
      gold: 11300,
      rating: 7.4,
    },
    {
      side: "blue",
      playerId: "blue-mid",
      playerName: "Blue Mid",
      role: "MID",
      championId: "orianna",
      kills: 8,
      deaths: 1,
      assists: 7,
      gold: 14500,
      rating: 9.1,
    },
    {
      side: "blue",
      playerId: "blue-adc",
      playerName: "Blue ADC",
      role: "ADC",
      championId: "zeri",
      kills: 6,
      deaths: 2,
      assists: 6,
      gold: 13800,
      rating: 8.3,
    },
    {
      side: "red",
      playerId: "red-adc",
      playerName: "Red ADC",
      role: "ADC",
      championId: "jinx",
      kills: 1,
      deaths: 7,
      assists: 2,
      gold: 8900,
      rating: 4.2,
    },
  ],
  goldDiffTimeline: [
    { minute: 10, diff: -1800 },
    { minute: 20, diff: 900 },
    { minute: 30, diff: 6200 },
  ],
  timelineEvents: [
    { minute: 6, side: "red", type: "first_blood", label: "Red first blood" },
    { minute: 23, side: "blue", type: "baron", label: "Blue Baron" },
  ],
  objectives: {
    blue: {
      voidgrubs: 4,
      dragons: 3,
      dragonSoul: false,
      elderDragons: 0,
      heralds: 1,
      barons: 1,
      towers: 8,
      inhibitors: 1,
    },
    red: {
      voidgrubs: 2,
      dragons: 1,
      dragonSoul: false,
      elderDragons: 0,
      heralds: 0,
      barons: 0,
      towers: 2,
      inhibitors: 0,
    },
  },
  power: {
    blue: 84,
    red: 77,
    diff: 7,
    autoWin: false,
    winProbBlue: 0.64,
  },
  ...overrides,
});

describe("extractMatchContext", () => {
  it("extracts win, stomp, comeback, MVP, objective, role, draft, rivalry, and streak facts from simulated match output", () => {
    const context = extractMatchContext({
      match: baseResult(),
      userSide: "blue",
      leagueId: "lec",
      rivalry: true,
      streak: { type: "win", count: 3 },
      draft: { strongSide: "Bot", timing: "Late" },
    });

    expect(context.tags).toEqual([
      "win",
      "stomp",
      "objective_domination",
      "comeback",
      "neutral_objectives",
      "first_blood",
      "first_blood_against_us",
      "draft",
      "late_game",
      "mvp",
      "role_mid",
      "mvp_carry",
      "rivalry",
      "streak_win",
    ]);
    expect(context.facts).toMatchObject({
      leagueId: "lec",
      result: "win",
      durationMinutes: 31,
      killDiff: 9,
      killShare: 0.67,
      objectiveDiff: 12,
      comebackGoldDeficit: 1800,
      firstBloodSide: "red",
      mvpPlayerId: "blue-mid",
      mvpRole: "MID",
      strongSide: "Bot",
      streakCount: 3,
    });
  });

  it("extracts loss, underperformance, decisive mistake, early draft, and botlane tags without false comeback tags", () => {
    const loss = baseResult({
      winnerSide: "red",
      durationMinutes: 22,
      blueKills: 5,
      redKills: 19,
      goldDiffTimeline: [
        { minute: 10, diff: -2600 },
        { minute: 20, diff: -7800 },
      ],
      playerResults: [
        {
          side: "blue",
          playerId: "blue-adc",
          playerName: "Blue ADC",
          role: "ADC",
          championId: "zeri",
          kills: 0,
          deaths: 8,
          assists: 1,
          gold: 7600,
          rating: 3.2,
        },
        {
          side: "blue",
          playerId: "blue-support",
          playerName: "Blue Support",
          role: "SUPPORT",
          championId: "nautilus",
          kills: 0,
          deaths: 7,
          assists: 2,
          gold: 6100,
          rating: 3.9,
        },
      ],
      objectives: {
          blue: {
          voidgrubs: 0,
          dragons: 0,
          dragonSoul: false,
          elderDragons: 0,
          heralds: 0,
          barons: 0,
          towers: 1,
          inhibitors: 0,
        },
        red: {
          voidgrubs: 6,
          dragons: 3,
          dragonSoul: false,
          elderDragons: 0,
          heralds: 1,
          barons: 1,
          towers: 9,
          inhibitors: 2,
        },
      },
      timelineEvents: [
        { minute: 3, side: "blue", type: "first_blood", label: "Blue first blood" },
        { minute: 18, side: "red", type: "dragon", label: "Red Dragon" },
      ],
    });

    const context = extractMatchContext({
      match: loss,
      userSide: "blue",
      draft: { strongSide: "Bot", timing: "Early" },
    });

    expect(context.tags).toEqual([
      "loss",
      "stomped",
      "objective_control",
      "underperformance",
      "decisive_mistake",
      "first_blood",
      "first_blood_for_us",
      "draft",
      "early_game",
      "role_adc",
      "role_support",
      "botlane_underperformed",
    ]);
    expect(context.facts).toMatchObject({
      result: "loss",
      worstPlayerId: "blue-adc",
      worstRole: "ADC",
      durationMinutes: 22,
      strongSide: "Bot",
      timing: "Early",
      firstBloodSide: "blue",
    });
  });

  it("marks close games and preserves compact fact signals for tighter ranking", () => {
    const closeGame = baseResult({
      blueKills: 11,
      redKills: 10,
      goldDiffTimeline: [
        { minute: 8, diff: -400 },
        { minute: 16, diff: 100 },
        { minute: 30, diff: 700 },
      ],
      objectives: {
        blue: {
          voidgrubs: 1,
          dragons: 2,
          dragonSoul: false,
          elderDragons: 0,
          heralds: 0,
          barons: 0,
          towers: 4,
          inhibitors: 0,
        },
        red: {
          voidgrubs: 1,
          dragons: 2,
          dragonSoul: false,
          elderDragons: 0,
          heralds: 0,
          barons: 0,
          towers: 4,
          inhibitors: 0,
        },
      },
      timelineEvents: [
        { minute: 2, side: "red", type: "first_blood", label: "Red first blood" },
        { minute: 17, side: "blue", type: "herald", label: "Blue Herald" },
      ],
    });

    const context = extractMatchContext({ match: closeGame, userSide: "blue" });

    expect(context.tags).toEqual([
      "win",
      "close_game",
      "objective_control",
      "first_blood",
      "first_blood_against_us",
      "mvp",
      "role_mid",
      "mvp_carry",
    ]);
    expect(context.facts).toMatchObject({
      durationMinutes: 31,
      killDiff: 1,
      objectiveDiff: 0,
      firstBloodSide: "red",
      objectiveLead: 0,
    });
  });
});
