import { calculateLolOvr } from "../../lib/lolPlayerStats";
import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import { getLolRoleForPlayer } from "../squad/SquadTab.helpers";

import type { LeagueStanding, TeamProfileViewModel } from "./TeamProfile.types";

const ROLE_ORDER: Record<string, number> = {
  TOP: 1,
  JUNGLE: 2,
  MID: 3,
  ADC: 4,
  SUPPORT: 5,
};

function sortRoster(players: PlayerData[]): PlayerData[] {
  return [...players].sort((leftPlayer, rightPlayer) => {
    const leftRole = getLolRoleForPlayer(leftPlayer);
    const rightRole = getLolRoleForPlayer(rightPlayer);
    return (
      (ROLE_ORDER[leftRole] || 99) -
      (ROLE_ORDER[rightRole] || 99) ||
      leftPlayer.full_name.localeCompare(rightPlayer.full_name)
    );
  });
}

function calculateAverageOvr(roster: PlayerData[]): number {
  if (roster.length === 0) {
    return 0;
  }

  return Math.round(
    roster.reduce((sum, player) => {
      return sum + calculateLolOvr(player);
    }, 0) / roster.length,
  );
}

function getSortedStandings(gameState: GameStateData): LeagueStanding[] {
  if (!gameState.league?.standings) {
    return [];
  }

  return [...gameState.league.standings].sort(
    (leftEntry, rightEntry) =>
      rightEntry.points - leftEntry.points ||
      rightEntry.goals_for -
        rightEntry.goals_against -
        (leftEntry.goals_for - leftEntry.goals_against) ||
      rightEntry.goals_for - leftEntry.goals_for,
  );
}

export function buildTeamProfileViewModel(
  team: TeamData,
  gameState: GameStateData,
): TeamProfileViewModel {
  const roster = sortRoster(
    gameState.players.filter((player) => player.team_id === team.id),
  );
  const allStandings = getSortedStandings(gameState);

  return {
    roster,
    avgOvr: calculateAverageOvr(roster),
    totalWages: roster.reduce((sum, player) => sum + player.wage, 0),
    totalValue: roster.reduce((sum, player) => sum + player.market_value, 0),
    manager: gameState.manager.team_id === team.id ? gameState.manager : null,
    leaguePos: allStandings.findIndex((entry) => entry.team_id === team.id) + 1,
    standings:
      gameState.league?.standings.find((entry) => entry.team_id === team.id) ?? null,
  };
}
