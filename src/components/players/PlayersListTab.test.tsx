import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type {
  GameStateData,
  PlayerData,
  TeamData,
} from "../../store/gameStore";
import PlayersListTab from "./PlayersListTab";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "players.searchPlaceholder") return "Search players";
      if (key === "players.allPos") return "All positions";
      if (key === "players.allTeams") return "All teams";
      if (key === "players.nPlayersFound") return `${params?.count} players`;
      if (key === "players.noMatch") return "No matches";
      if (key === "common.all") return "All";
      if (key === "common.position") return "Position";
      if (key === "common.name") return "Name";
      if (key === "common.age") return "Age";
      if (key === "common.nationality") return "Nationality";
      if (key === "common.team") return "Team";
      if (key === "common.value") return "Value";
      if (key === "common.ovr") return "OVR";
      if (key === "common.status") return "Status";
      if (key === "transfers.transfer") return "Transfer";
      if (key === "transfers.loan") return "Loan";
      if (key === "common.injured") return "Injured";
      if (key.startsWith("common.posAbbr.")) {
        return key.replace("common.posAbbr.", "");
      }
      return key;
    },
  }),
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
    season_income: 0,
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

function createGameState(): GameStateData {
  return {
    clock: {
      current_date: "2026-08-01T00:00:00Z",
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
      team_id: "team-1",
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
    teams: [
      createTeam(),
      createTeam({ id: "team-2", name: "Beta FC", short_name: "BET" }),
    ],
    players: [
      createPlayer(),
      createPlayer({
        id: "player-2",
        match_name: "A. Keeper",
        full_name: "Alex Keeper",
        position: "Goalkeeper",
        natural_position: "Goalkeeper",
        team_id: "team-2",
      }),
      createPlayer({
        id: "player-3",
        match_name: "D. Loan",
        full_name: "David Loan",
        position: "Defender",
        natural_position: "Defender",
        team_id: "team-2",
        loan_listed: true,
      }),
    ],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("PlayersListTab", () => {
  it("filters by search and position before selecting a player", () => {
    const onSelectPlayer = vi.fn();

    render(
      <PlayersListTab
        gameState={createGameState()}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByPlaceholderText("Search players"), {
      target: { value: "keeper" },
    });

    expect(screen.getByText("Alex Keeper")).toBeInTheDocument();
    expect(screen.queryByText("John Smith")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "SUPPORT" }));
    fireEvent.click(screen.getByText("Alex Keeper"));

    expect(onSelectPlayer).toHaveBeenCalledWith("player-2");
  });

  it("keeps team navigation separate from player row selection", () => {
    const onSelectPlayer = vi.fn();
    const onSelectTeam = vi.fn();

    render(
      <PlayersListTab
        gameState={createGameState()}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
      />,
    );

    fireEvent.click(screen.getAllByRole("button", { name: "Beta FC" })[0]);

    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
    expect(onSelectPlayer).not.toHaveBeenCalled();
  });
});
