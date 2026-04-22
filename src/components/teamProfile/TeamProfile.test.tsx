import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import TeamProfile from "./TeamProfile";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  initReactI18next: {
    type: "3rdParty",
    init: () => {},
  },
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        "common.back": "Back",
        "teams.avgOvr": "Avg OVR",
        "teams.rep": "Rep",
        "teams.squad": "Squad",
        "teams.est": "Est.",
        "manager.reputation": "Reputation",
        "teamProfile.leaguePos": "League Pos",
        "teamProfile.managerLabel": "Manager:",
        "teamProfile.clubInfo": "Club Information",
        "teamProfile.stadium": "Stadium",
        "teamProfile.capacity": "Capacity",
        "dashboard.finances": "Finances",
        "teamProfile.balance": "Balance",
        "teamProfile.totalWages": "Total Wages",
        "teamProfile.squadOverview": "Squad Overview",
        "teamProfile.squadSize": "Squad Size",
        "teamProfile.leagueStanding": "League Standing",
        "teamProfile.advancedStats": "Team Stats",
        "teamProfile.matchesPlayed": "Matches",
        "teamProfile.wins": "Wins",
        "teamProfile.losses": "Losses",
        "teamProfile.kills": "Kills",
        "teamProfile.deaths": "Deaths",
        "teamProfile.goldEarned": "Gold Earned",
        "teamProfile.damageToChampions": "Damage To Champions",
        "teamProfile.objectives": "Objectives",
        "teamProfile.averageGameDuration": "Average Game Duration",
        "teamProfile.perMatch": "Per Match",
        "teamProfile.kda": "K / D / A",
        "teamProfile.economy": "Gold / Objectives",
        "teamProfile.side": "Side",
        "finances.wageBudget": "Wage Budget",
        "finances.transferBudget": "Transfer Budget",
        "finances.squadValue": "Squad Value",
        "finances.seasonIncome": "Season Income",
        "finances.perWeekSuffix": "/wk",
        "tactics.formation": "Formation",
        "tactics.playStyle": "Play Style",
        "common.played": "P",
        "common.won": "W",
        "common.drawn": "D",
        "common.lost": "L",
        "common.gf": "GF",
        "common.ga": "GA",
        "common.gd": "GD",
        "common.pts": "Pts",
        "common.position": "Pos",
        "common.name": "Name",
        "common.age": "Age",
        "common.nationality": "Nationality",
        "common.value": "Value",
        "common.ovr": "OVR",
      };

      return map[key] ?? key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../../lib/countries", () => ({
  countryName: () => "England",
  isValidCountryCode: () => true,
  normaliseNationality: (value: string) => value,
  resolveCountryFlagCode: () => "GB",
}));

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "GB",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 500000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 250000,
    season_income: 100000,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "General",
    training_intensity: "Balanced",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000000", secondary: "#ffffff" },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

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
      pace: 60,
      stamina: 60,
      strength: 60,
      agility: 60,
      passing: 60,
      shooting: 60,
      tackling: 60,
      dribbling: 60,
      defending: 60,
      positioning: 60,
      vision: 60,
      decisions: 60,
      composure: 60,
      aggression: 60,
      teamwork: 60,
      leadership: 60,
      handling: 20,
      reflexes: 20,
      aerial: 60,
    },
    condition: 80,
    morale: 75,
    injury: null,
    team_id: "team-1",
    contract_end: null,
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

function createGameState(team: TeamData): GameStateData {
  return {
    clock: {
      current_date: "2026-08-10T00:00:00Z",
      start_date: "2026-07-01T00:00:00Z",
    },
    manager: {
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: team.id,
      career_stats: {
        matches_managed: 0,
        wins: 0,
        draws: 0,
        losses: 0,
        trophies: 0,
        best_finish: null,
      },
      career_history: [],
    },
    teams: [team],
    players: [createPlayer()],
    staff: [],
    messages: [],
    news: [],
    league: {
      id: "league-1",
      name: "League",
      season: 1,
      fixtures: [],
      standings: [
        {
          team_id: team.id,
          played: 2,
          won: 1,
          drawn: 1,
          lost: 0,
          goals_for: 5,
          goals_against: 1,
          points: 4,
        },
      ],
    },
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("TeamProfile", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(null);
  });

  it("loads and renders team stats overview from the backend", async () => {
    const team = createTeam();
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "get_team_stats_overview") {
        return {
          matchesPlayed: 12,
          wins: 8,
          losses: 3,
          metrics: {
            kills: { total: 160, perMatch: 13.3 },
            deaths: { total: 68, perMatch: 5.7 },
            goldEarned: { total: 5400, perMatch: 450 },
            damageToChampions: { total: 6300, perMatch: 525 },
            objectives: { total: 48, perMatch: 4 },
            averageGameDurationSeconds: { total: 25200, perMatch: 2100 },
          },
        };
      }

      return null;
    });

    render(
      <TeamProfile
        team={team}
        gameState={createGameState(team)}
        isOwnTeam
        onClose={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_team_stats_overview", {
        teamId: "team-1",
      });
      expect(screen.getByText("Team Stats")).toBeInTheDocument();
      expect(screen.getByText("8")).toBeInTheDocument();
      expect(screen.getByText("160")).toBeInTheDocument();
      expect(screen.getByText("5400")).toBeInTheDocument();
      expect(screen.getByText("2100s")).toBeInTheDocument();
    });
  });

  it("loads and renders recent team match history from the backend", async () => {
    const team = createTeam();
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === "get_team_match_history") {
        return [
          {
            fixtureId: "fixture-2",
            date: "2026-08-01",
            competition: "League",
            matchday: 1,
            opponentTeamId: "team-2",
            opponentName: "Bravo FC",
            side: "Red",
            result: "Loss",
            gameDurationSeconds: 2100,
            kills: 16,
            deaths: 7,
            goldEarned: 62000,
            damageToChampions: 88000,
            objectives: 3,
          },
        ];
      }

      if (command === "get_team_stats_overview") {
        return null;
      }

      return null;
    });

    render(
      <TeamProfile
        team={team}
        gameState={createGameState(team)}
        isOwnTeam
        onClose={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_team_match_history", {
        teamId: "team-1",
        limit: 5,
      });
      expect(screen.getByText("Bravo FC")).toBeInTheDocument();
      expect(screen.getByText("Red · Loss")).toBeInTheDocument();
      expect(screen.getByText("16 / 7 / 3")).toBeInTheDocument();
      expect(screen.getByText("62000 / 88000")).toBeInTheDocument();
    });
  });
});
