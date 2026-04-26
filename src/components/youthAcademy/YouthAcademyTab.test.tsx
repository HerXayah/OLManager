import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData, TeamData } from "../../store/gameStore";
import YouthAcademyTab from "./YouthAcademyTab";

const getAcademyAcquisitionOptions = vi.fn();
const acquireAcademyTeam = vi.fn();

vi.mock("../../services/academyService", () => ({
  getAcademyAcquisitionOptions: (...args: unknown[]) => getAcademyAcquisitionOptions(...args),
  acquireAcademyTeam: (...args: unknown[]) => acquireAcademyTeam(...args),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "youthAcademy.title") return "Youth Academy";
      if (key === "youthAcademy.playersUnder21") return `${params?.count} youth players`;
      if (key === "youthAcademy.youthPlayers") return "Youth Players";
      if (key === "youthAcademy.avgOvr") return "Avg OVR";
      if (key === "youthAcademy.avgPotential") return "Avg Potential";
      if (key === "youthAcademy.highPotential") return "High Potential";
      if (key === "youthAcademy.youthCoach") return "Youth Coach";
      if (key === "youthAcademy.noYouthPlayers") return "No youth players";
      if (key === "youthAcademy.player") return "Player";
      if (key === "youthAcademy.pos") return "Pos";
      if (key === "youthAcademy.age") return "Age";
      if (key === "youthAcademy.ovr") return "OVR";
      if (key === "youthAcademy.potential") return "Potential";
      if (key === "youthAcademy.growth") return "Growth";
      if (key === "youthAcademy.traits") return "Traits";
      if (key === "youthAcademy.condition") return "Condition";
      if (key.startsWith("common.posAbbr.")) return key.replace("common.posAbbr.", "");
      return key;
    },
    i18n: { language: "en" },
  }),
}));

vi.mock("../TraitBadge", () => ({
  TraitList: () => <span>Traits</span>,
}));

beforeEach(() => {
  getAcademyAcquisitionOptions.mockReset();
  acquireAcademyTeam.mockReset();
});

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
    starting_xi_ids: [] as string[],
    form: [],
    history: [],
    ...overrides,
  };
}

function createGameState(teamOverrides: Partial<TeamData> = {}): GameStateData {
  const team = createTeam(teamOverrides);

  return {
    clock: { current_date: "2026-08-10T00:00:00Z", start_date: "2026-07-01T00:00:00Z" },
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
      career_stats: { matches_managed: 0, wins: 0, draws: 0, losses: 0, trophies: 0, best_finish: null },
      career_history: [],
    },
    teams: [team],
    players: [],
    staff: [],
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

describe("YouthAcademyTab", () => {
  it("renders acquisition options from the backend service", async () => {
    getAcademyAcquisitionOptions.mockResolvedValueOnce({
      parent_team_id: "team-1",
      acquisition_allowed: true,
      blocked_reason: null,
      options: [
        {
          source_team_id: "mkoi-fenix",
          source_team_name: "Movistar KOI Fénix",
          source_team_short_name: "MKOI F",
          source_team_logo_url: "https://cdn.example/logo.png",
          source_identity: {
            source_team_id: "mkoi-fenix",
            original_name: "Movistar KOI Fénix",
            original_short_name: "MKOI F",
            original_logo_url: "https://cdn.example/logo.png",
          },
          erl_league_id: "nlc",
          league_name: "NLC",
          country: "GB",
          region: "UK",
          assignment_rule: "Domestic",
          fallback_reason: null,
          reputation: 4,
          development_level: 5,
          acquisition_cost: 260000,
          rebrand_allowed: true,
        },
      ],
    });

    render(<YouthAcademyTab gameState={createGameState()} onSelectPlayer={vi.fn()} />);

    expect(getAcademyAcquisitionOptions).toHaveBeenCalledWith("team-1");
    await waitFor(() => expect(screen.getByText("Movistar KOI Fénix")).toBeInTheDocument());
    expect(screen.getByText("260000")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Adquirir" })).toBeInTheDocument();
  });

  it("invokes the acquisition action for the selected backend option", async () => {
    getAcademyAcquisitionOptions.mockResolvedValueOnce({
      parent_team_id: "team-1",
      acquisition_allowed: true,
      blocked_reason: null,
      options: [
        {
          source_team_id: "solary",
          source_team_name: "Solary",
          source_team_short_name: "SOL",
          source_team_logo_url: null,
          source_identity: {
            source_team_id: "solary",
            original_name: "Solary",
            original_short_name: "SOL",
            original_logo_url: null,
          },
          erl_league_id: "lfl",
          league_name: "LFL",
          country: "FR",
          region: "France",
          assignment_rule: "Fallback",
          fallback_reason: "Nearby ERL fallback",
          reputation: 5,
          development_level: 4,
          acquisition_cost: 300000,
          rebrand_allowed: false,
        },
      ],
    });

    acquireAcademyTeam.mockResolvedValueOnce(createGameState());

    render(<YouthAcademyTab gameState={createGameState()} onSelectPlayer={vi.fn()} />);

    await waitFor(() => expect(screen.getByText("Solary")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: "Adquirir" }));

    await waitFor(() =>
      expect(acquireAcademyTeam).toHaveBeenCalledWith({
        parent_team_id: "team-1",
        source_team_id: "solary",
        custom_name: undefined,
        custom_short_name: undefined,
        custom_logo_url: undefined,
      }),
    );
  });

  it("shows a backend blocked state when acquisition is not available", async () => {
    getAcademyAcquisitionOptions.mockResolvedValueOnce({
      parent_team_id: "team-1",
      acquisition_allowed: false,
      blocked_reason: "No eligible ERL teams within budget",
      options: [],
    });

    render(<YouthAcademyTab gameState={createGameState()} onSelectPlayer={vi.fn()} />);

    await waitFor(() => expect(screen.getByText("No eligible ERL teams within budget")).toBeInTheDocument());
    expect(screen.queryByRole("button", { name: "Adquirir" })).not.toBeInTheDocument();
  });

  it("does not crash when the current team cannot be resolved in state", () => {
    const gameState = createGameState();
    gameState.manager.team_id = "missing-team";

    render(<YouthAcademyTab gameState={gameState} onSelectPlayer={vi.fn()} />);

    expect(screen.getByText("No youth players")).toBeInTheDocument();
  });
});
