import { describe, expect, it } from "vitest";

import type { PlayerData } from "../store/gameStore";
import { calculateLolOvr, getLolVisibleStatValue } from "./lolPlayerStats";
import { TRAINING_FOCUS_ATTRS } from "./trainingFocus";

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
    nationality: "GB",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 62,
      stamina: 66,
      strength: 58,
      agility: 74,
      passing: 70,
      shooting: 73,
      tackling: 40,
      dribbling: 78,
      defending: 35,
      positioning: 72,
      vision: 75,
      decisions: 76,
      composure: 71,
      aggression: 48,
      teamwork: 69,
      leadership: 64,
      handling: 20,
      reflexes: 22,
      aerial: 30,
    },
    condition: 80,
    morale: 75,
    injury: null,
    team_id: "team-1",
    contract_end: "2027-06-30",
    wage: 12000,
    market_value: 350000,
    stats: {
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

describe("lolPlayerStats", () => {
  it("derives the visible LoL stats used in profile and training UI", () => {
    const player = createPlayer();

    expect(getLolVisibleStatValue(player, "mechanics")).toBe(71);
    expect(getLolVisibleStatValue(player, "macro")).toBe(73);
    expect(getLolVisibleStatValue(player, "championPool")).toBe(74);
    expect(getLolVisibleStatValue(player, "mentalResilience")).toBe(68);
  });

  it("computes roster OVR from the LoL-facing visible stats", () => {
    expect(calculateLolOvr(createPlayer())).toBe(72);
  });

  it("advertises training gains with LoL-visible stats instead of legacy football attributes", () => {
    expect(TRAINING_FOCUS_ATTRS.Scrims).toEqual(["teamfighting", "macro", "consistency"]);
    expect(TRAINING_FOCUS_ATTRS.IndividualCoaching).toEqual([
      "mechanics",
      "laning",
      "consistency",
    ]);
    expect(TRAINING_FOCUS_ATTRS.ChampionPoolPractice).toEqual([
      "mechanics",
      "championPool",
      "laning",
    ]);
  });
});
