import type { PlayerData, TeamData } from "./types";

export function isAcademyTeam(team: TeamData): boolean {
  return team.team_kind === "Academy";
}

export function isMainTeam(team: TeamData): boolean {
  return team.team_kind !== "Academy";
}

export function getMainTeams(teams: TeamData[]): TeamData[] {
  return teams.filter(isMainTeam);
}

export function findAcademyTeamForParent(
  teams: TeamData[],
  parentTeamId: string | null | undefined,
): TeamData | null {
  if (!parentTeamId) {
    return null;
  }

  const parent = teams.find((team) => team.id === parentTeamId);
  const parentAcademyId = parent?.academy_team_id ?? null;

  return (
    teams.find(
      (team) =>
        isAcademyTeam(team) &&
        (team.parent_team_id === parentTeamId || team.id === parentAcademyId),
    ) ?? null
  );
}

export function getTeamAcademyRoster(
  teams: TeamData[],
  players: PlayerData[],
  parentTeamId: string | null | undefined,
): PlayerData[] {
  const academy = findAcademyTeamForParent(teams, parentTeamId);

  if (!academy) {
    return [];
  }

  return players.filter((player) => player.team_id === academy.id);
}
