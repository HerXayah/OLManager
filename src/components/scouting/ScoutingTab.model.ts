import type {
  PlayerData,
  ScoutingAssignment,
  TeamData,
} from "../../store/gameStore";
import { getTeamName } from "../../lib/helpers";
import { calculateLolOvr } from "../../lib/lolPlayerStats";
import { getLolRoleForPlayer } from "../squad/SquadTab.helpers";

interface FilterScoutablePlayersParams {
  players: PlayerData[];
  teams: TeamData[];
  myTeamId: string;
  posFilter: string;
  searchQuery: string;
}

export function filterScoutablePlayers({
  players,
  teams,
  myTeamId,
  posFilter,
  searchQuery,
}: FilterScoutablePlayersParams): PlayerData[] {
  return players
    .filter((player) => player.team_id !== myTeamId)
    .filter(
      (player) =>
        posFilter === "All" ||
        getLolRoleForPlayer(player) === posFilter,
    )
    .filter((player) => {
      if (!searchQuery) {
        return true;
      }

      const query = searchQuery.toLowerCase();

      return (
        player.match_name.toLowerCase().includes(query) ||
        player.full_name.toLowerCase().includes(query) ||
        player.nationality.toLowerCase().includes(query) ||
        (player.team_id &&
          getTeamName(teams, player.team_id).toLowerCase().includes(query))
      );
    })
    .sort(
      (left, right) =>
        calculateLolOvr(right) - calculateLolOvr(left),
    );
}

export function paginateScoutablePlayers(
  players: PlayerData[],
  page: number,
  pageSize: number,
) {
  const totalPages = Math.max(1, Math.ceil(players.length / pageSize));
  const safePage = Math.min(page, totalPages - 1);

  return {
    totalPages,
    safePage,
    players: players.slice(safePage * pageSize, (safePage + 1) * pageSize),
  };
}

export function buildAlreadyScoutingIds(assignments: ScoutingAssignment[]) {
  return new Set(assignments.map((assignment) => assignment.player_id));
}
