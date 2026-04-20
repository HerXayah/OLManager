import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import HomeLeaguePositionCard from "./HomeLeaguePositionCard";
import type { TeamData } from "../../store/gameStore";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "home.standings") return "Standings";
      if (key === "home.leaguePosition") return "League Position";
      if (key === "season.phases.Preseason") return "Preseason";
      if (key === "season.startsOn") return `Starts on ${params?.date}`;
      if (key === "season.noOpener") return "No opener";
      if (key === "season.standingsLocked") return "Standings are locked before kickoff.";
      if (key === "common.place.2") return "2nd place";
      if (key === "home.winningStreak") return "Winning Streak";
      if (key === "home.noLeague") return "No league data";
      return key;
    },
  }),
}));

describe("HomeLeaguePositionCard", () => {
  const teams: TeamData[] = [
    {
      id: "lec-team-1",
      name: "Team One",
      short_name: "T1",
      country: "ES",
      city: "Madrid",
      stadium_name: "Arena",
      stadium_capacity: 10000,
      finance: 0,
      manager_id: null,
      reputation: 70,
      wage_budget: 0,
      transfer_budget: 0,
      season_income: 0,
      season_expenses: 0,
      formation: "4-3-3",
      play_style: "Balanced",
      training_focus: "Balanced",
      training_intensity: "Normal",
      training_schedule: "Default",
      founded_year: 2020,
      colors: { primary: "#000", secondary: "#fff" },
      starting_xi_ids: [],
      form: [],
      history: [],
    },
  ];

  it("renders preseason standings lock messaging", () => {
    render(
      <HomeLeaguePositionCard
        isPreseason={true}
        phase="Preseason"
        seasonStartLabel="Jan 12"
        sortedStandings={[]}
        teams={teams}
        myTeamId={null}
      />,
    );

    expect(screen.getByText("Preseason")).toBeInTheDocument();
    expect(screen.getByText("Starts on Jan 12")).toBeInTheDocument();
    expect(screen.getByText("Standings are locked before kickoff.")).toBeInTheDocument();
  });

  it("renders league table summary and form streak data", () => {
    render(
      <HomeLeaguePositionCard
        isPreseason={false}
        phase="RegularSeason"
        seasonStartLabel={null}
        sortedStandings={[
          {
            team_id: "lec-team-1",
            played: 5,
            won: 3,
            drawn: 1,
            lost: 1,
            goals_for: 9,
            goals_against: 4,
            points: 10,
          },
        ]}
        teams={teams}
        myTeamId="lec-team-1"
      />,
    );

    expect(screen.getByText("League Position")).toBeInTheDocument();
    expect(screen.getByText("T1")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();
    expect(screen.getByText("75%")).toBeInTheDocument();
  });

  it("renders empty-state when no standings exist", () => {
    render(
      <HomeLeaguePositionCard
        isPreseason={false}
        phase="RegularSeason"
        seasonStartLabel={null}
        sortedStandings={[]}
        teams={teams}
        myTeamId={null}
      />,
    );

    expect(screen.getByText("No league data")).toBeInTheDocument();
  });
});
