import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type {
  GameStateData,
  PlayerData,
  ScoutingAssignment,
  StaffData,
  TeamData,
} from "../../store/gameStore";
import ScoutingTab from "./ScoutingTab";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "scouting.title") return "Scouting";
      if (key === "scouting.scouts") return "Scouts";
      if (key === "scouting.activeAssignments") return "Active Assignments";
      if (key === "scouting.freeSlots") return "Free Slots";
      if (key === "scouting.activeScoutingAssignments") return "Active Scouting Assignments";
      if (key === "scouting.yourScouts") return "Your Scouts";
      if (key === "scouting.noScouts") return "No scouts";
      if (key === "scouting.noScoutsHint") return "Hire a scout first";
      if (key === "scouting.findPlayers") return "Find Players";
      if (key === "scouting.searchPlaceholder") return "Search players";
      if (key === "scouting.player") return "Player";
      if (key === "scouting.pos") return "Pos";
      if (key === "scouting.age") return "Age";
      if (key === "scouting.team") return "Team";
      if (key === "scouting.value") return "Value";
      if (key === "scouting.action") return "Action";
      if (key === "scouting.scoutBtn") return "Scout";
      if (key === "scouting.scoutingInProgress") return "Scouting in progress";
      if (key === "scouting.noScoutsFree") return "No scouts free";
      if (key === "scouting.noPlayersFound") return "No players found";
      if (key === "scouting.slots") return "slots";
      if (key === "scouting.judgingAbility") return "Judging Ability";
      if (key === "scouting.judgingPotential") return "Judging Potential";
      if (key === "scouting.scoutLabel") return params?.name ? `Scout ${params.name}` : "Scout ";
      if (key === "scouting.daysLeft") return `${params?.days} days left`;
      if (key === "common.all") return "All";
      if (key === "common.freeAgent") return "Free Agent";
      return key;
    },
    i18n: { language: "en" },
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
    team_id: "team-2",
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

function createScout(overrides: Partial<StaffData> = {}): StaffData {
  return {
    id: "staff-1",
    first_name: "Sam",
    last_name: "Scout",
    date_of_birth: "1985-01-01",
    nationality: "GB",
    role: "Scout",
    attributes: {
      coaching: 20,
      judging_ability: 65,
      judging_potential: 70,
      physiotherapy: 10,
    },
    team_id: "team-1",
    specialization: null,
    wage: 1000,
    contract_end: "2027-06-30",
    ...overrides,
  };
}

function createGameState(options?: {
  scouts?: StaffData[];
  assignments?: ScoutingAssignment[];
  players?: PlayerData[];
}): GameStateData {
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
      createTeam({ id: "team-2", name: "Beta FC", short_name: "BET", manager_id: "manager-2" }),
    ],
    players: options?.players ?? [createPlayer()],
    staff: options?.scouts ?? [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: options?.assignments ?? [],
    board_objectives: [],
  };
}

describe("ScoutingTab", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("renders the no-scouts empty state", () => {
    render(
      <ScoutingTab gameState={createGameState()} onGameUpdate={vi.fn()} />,
    );

    expect(screen.getByText("No scouts")).toBeInTheDocument();
    expect(screen.getByText("Hire a scout first")).toBeInTheDocument();
  });

  it("sends a scout assignment and forwards the updated state", async () => {
    const updatedState = createGameState();
    const onGameUpdate = vi.fn();
    invokeMock.mockResolvedValue(updatedState);

    render(
      <ScoutingTab
        gameState={createGameState({ scouts: [createScout()] })}
        onGameUpdate={onGameUpdate}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /Scout/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("send_scout", {
        scoutId: "staff-1",
        playerId: "player-1",
      });
      expect(onGameUpdate).toHaveBeenCalledWith(updatedState);
    });
  });
});