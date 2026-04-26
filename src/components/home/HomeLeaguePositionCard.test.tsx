import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import HomeLeaguePositionCard from "./HomeLeaguePositionCard";
import type { LeagueData, TeamData } from "../../store/gameStore";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "home.standings") return "Standings";
      if (key === "schedule.playoffs") return "Playoffs";
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
    expect(screen.getAllByText("1").length).toBeGreaterThan(0);
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

  it("renders playoff bracket in place of standings during playoffs", () => {
    const league: LeagueData = {
      id: "league-1",
      name: "LEC Winter",
      season: 1,
      fixtures: [
        {
          id: "fixture-aa11",
          matchday: 10,
          date: "2025-03-20",
          home_team_id: "lec-team-1",
          away_team_id: "lec-team-2",
          competition: "Playoffs",
          status: "Scheduled",
          best_of: 3,
          result: null,
        },
      ],
      standings: [],
    };

    render(
      <HomeLeaguePositionCard
        isPreseason={false}
        phase="Playoffs"
        seasonStartLabel={null}
        league={league}
        sortedStandings={[]}
        teams={[
          ...teams,
          {
            ...teams[0],
            id: "lec-team-2",
            name: "Team Two",
            short_name: "T2",
          },
        ]}
        myTeamId="lec-team-1"
      />,
    );

    expect(screen.getAllByText("Playoffs").length).toBeGreaterThan(0);
    expect(screen.getByText("home.nextMatch")).toBeInTheDocument();
    expect(screen.getByText("T1 vs T2")).toBeInTheDocument();
    expect(screen.getByText(/BO3/)).toBeInTheDocument();
    expect(screen.queryByText(/M10-/i)).not.toBeInTheDocument();
  });
});
