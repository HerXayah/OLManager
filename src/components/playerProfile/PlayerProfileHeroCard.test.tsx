import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import PlayerProfileHeroCard from "./PlayerProfileHeroCard";
import type { PlayerData } from "../../store/gameStore";

function makePlayer(overrides?: Partial<PlayerData>): PlayerData {
  return {
    id: "lec-player-test",
    match_name: "Test",
    full_name: "Test Player",
    date_of_birth: "2000-01-01",
    nationality: "ES",
    position: "Midfielder",
    natural_position: "Midfielder",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 70,
      stamina: 70,
      strength: 70,
      agility: 70,
      passing: 70,
      shooting: 70,
      tackling: 70,
      dribbling: 70,
      defending: 70,
      positioning: 70,
      vision: 70,
      decisions: 70,
      composure: 70,
      aggression: 70,
      teamwork: 70,
      leadership: 70,
      handling: 20,
      reflexes: 20,
      aerial: 20,
    },
    condition: 90,
    morale: 80,
    injury: null,
    team_id: "team-1",
    contract_end: "2026-12-31",
    wage: 1000,
    market_value: 50000,
    stats: {
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
      shots: 0,
      shots_on_target: 0,
      passes_completed: 0,
      passes_attempted: 0,
      tackles_won: 0,
      interceptions: 0,
      fouls_committed: 0,
      fouls_suffered: 0,
      offsides: 0,
      saves: 0,
      goals_conceded: 0,
      penalties_scored: 0,
      penalties_missed: 0,
      penalties_saved: 0,
      motm: 0,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

const translate = (key: string, options?: Record<string, string | number>) => {
  const template = String(options?.defaultValue ?? key);
  if (!options) return template;

  return Object.entries(options).reduce((acc, [token, value]) => {
    if (token === "defaultValue") return acc;
    return acc.replace(`{{${token}}}`, String(value));
  }, template);
};

describe("PlayerProfileHeroCard potential UX", () => {
  it("shows hidden potential with start button", () => {
    const onStart = vi.fn();

    render(
      <PlayerProfileHeroCard
        player={makePlayer()}
        ovr={80}
        primaryPosition="Midfielder"
        age={24}
        teamName="Team"
        weeklySuffix="/wk"
        language="es"
        isOwnClub
        scoutAvailability={{
          canScout: false,
          availableScout: null,
          scouts: [],
          alreadyScouting: false,
          allBusy: false,
        }}
        scoutStatus="idle"
        scoutError={null}
        onScout={() => undefined}
        onStartPotentialResearch={onStart}
        t={translate}
      />,
    );

    expect(screen.getByText("??")).toBeInTheDocument();
    const button = screen.getByRole("button", { name: "Investigar potencial" });
    expect(button).toBeEnabled();
    fireEvent.click(button);
    expect(onStart).toHaveBeenCalledTimes(1);
  });

  it("shows active research progress state", () => {
    render(
      <PlayerProfileHeroCard
        player={makePlayer({ potential_research_eta_days: 5 })}
        ovr={80}
        primaryPosition="Midfielder"
        age={24}
        teamName="Team"
        weeklySuffix="/wk"
        language="es"
        isOwnClub
        scoutAvailability={{
          canScout: false,
          availableScout: null,
          scouts: [],
          alreadyScouting: false,
          allBusy: false,
        }}
        scoutStatus="idle"
        scoutError={null}
        onScout={() => undefined}
        onStartPotentialResearch={() => undefined}
        t={translate}
      />,
    );

    expect(screen.getByText("Investigando… 2/7")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Investigar potencial" })).not.toBeInTheDocument();
  });

  it("shows revealed potential value", () => {
    render(
      <PlayerProfileHeroCard
        player={makePlayer({ potential_revealed: 91 })}
        ovr={80}
        primaryPosition="Midfielder"
        age={24}
        teamName="Team"
        weeklySuffix="/wk"
        language="es"
        isOwnClub
        scoutAvailability={{
          canScout: false,
          availableScout: null,
          scouts: [],
          alreadyScouting: false,
          allBusy: false,
        }}
        scoutStatus="idle"
        scoutError={null}
        onScout={() => undefined}
        t={translate}
      />,
    );

    expect(screen.getByText("91")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Investigar potencial" })).not.toBeInTheDocument();
  });
});
