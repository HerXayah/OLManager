import { describe, expect, it } from "vitest";

import {
  findAcademyTeamForParent,
  getMainTeams,
  getTeamAcademyRoster,
} from "./academySelectors";
import type { PlayerData, TeamData } from "./types";

function team(overrides: Partial<TeamData>): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "GB",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 500000,
    manager_id: null,
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 250000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "General",
    training_intensity: "Balanced",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000", secondary: "#fff" },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

function player(overrides: Partial<PlayerData>): PlayerData {
  return {
    id: "player-1",
    match_name: "Rookie",
    full_name: "Rookie One",
    date_of_birth: "2008-01-01",
    nationality: "GB",
    position: "Mid",
    natural_position: "Mid",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 50,
      stamina: 50,
      strength: 50,
      agility: 50,
      passing: 50,
      shooting: 50,
      tackling: 50,
      dribbling: 50,
      defending: 50,
      positioning: 50,
      vision: 50,
      decisions: 50,
      composure: 50,
      aggression: 50,
      teamwork: 50,
      leadership: 50,
      handling: 50,
      reflexes: 50,
      aerial: 50,
    },
    condition: 80,
    morale: 80,
    injury: null,
    team_id: "academy-1",
    contract_end: "2028-01-01",
    wage: 1000,
    market_value: 10000,
    stats: { appearances: 0, goals: 0, assists: 0, clean_sheets: 0, yellow_cards: 0, red_cards: 0, avg_rating: 0, minutes_played: 0 },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

describe("academySelectors", () => {
  it("returns only main teams when academy teams share the teams collection", () => {
    const main = team({ id: "team-1", team_kind: "Main" });
    const academy = team({ id: "academy-1", team_kind: "Academy", parent_team_id: "team-1" });
    const legacyMain = team({ id: "legacy-team", team_kind: undefined });

    expect(getMainTeams([main, academy, legacyMain]).map((entry) => entry.id)).toEqual([
      "team-1",
      "legacy-team",
    ]);
  });

  it("finds an academy through explicit parent linkage or parent academy id", () => {
    const parent = team({ id: "team-1", team_kind: "Main", academy_team_id: "academy-1" });
    const linkedFromAcademy = team({ id: "academy-1", team_kind: "Academy", parent_team_id: "team-1" });
    const linkedFromParentOnly = team({ id: "academy-1", team_kind: "Academy" });

    expect(findAcademyTeamForParent([parent, linkedFromAcademy], "team-1")?.id).toBe("academy-1");
    expect(findAcademyTeamForParent([parent, linkedFromParentOnly], "team-1")?.id).toBe("academy-1");
  });

  it("returns the real academy roster without including parent roster players", () => {
    const parent = team({ id: "team-1", team_kind: "Main", academy_team_id: "academy-1" });
    const academy = team({ id: "academy-1", team_kind: "Academy", parent_team_id: "team-1" });
    const academyPlayer = player({ id: "academy-player", team_id: "academy-1" });
    const parentPlayer = player({ id: "parent-player", team_id: "team-1" });

    expect(getTeamAcademyRoster([parent, academy], [academyPlayer, parentPlayer], "team-1")).toEqual([
      academyPlayer,
    ]);
  });
});
