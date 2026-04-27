import type { GameStateData, PlayerData } from "../../store/gameStore";
import { getLolRoleForPlayer } from "../squad/SquadTab.helpers";

function normalizePlayerKey(player: PlayerData): string {
  const normalizeToken = (value: string): string =>
    value
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]/g, "");

  const matchName = normalizeToken(player.match_name || "");
  const fullName = normalizeToken(player.full_name || "");
  const identity = matchName || fullName || player.id;
  const dob = player.date_of_birth || "unknown-dob";
  const nationality = normalizeToken(player.nationality || "unknown-nat");
  return `${identity}|${dob}|${nationality}`;
}

function dedupePlayersPreferAcademy(
  players: PlayerData[],
  academyTeamIds: Set<string>,
): PlayerData[] {
  const byKey = new Map<string, PlayerData>();

  players.forEach((player) => {
    const key = normalizePlayerKey(player);
    const existing = byKey.get(key);
    if (!existing) {
      byKey.set(key, player);
      return;
    }

    const currentIsAcademy = academyTeamIds.has(player.team_id ?? "");
    const existingIsAcademy = academyTeamIds.has(existing.team_id ?? "");
    if (currentIsAcademy && !existingIsAcademy) {
      byKey.set(key, player);
    }
  });

  return [...byKey.values()];
}

export type TransferTabView = "my_list" | "market" | "erl" | "loans" | "offers";

export interface TransferCollections {
  myTransferList: PlayerData[];
  myLoanList: PlayerData[];
  marketPlayers: PlayerData[];
  erlPlayers: PlayerData[];
  loanPlayers: PlayerData[];
  playersWithOffers: PlayerData[];
}

export function deriveTransferCollections(
  gameState: GameStateData,
  userTeamId: string | null,
): TransferCollections {
  const academyTeamIds = new Set(
    gameState.teams
      .filter((team) => team.team_kind === "Academy")
      .map((team) => team.id),
  );
  const academyIdentityKeys = new Set(
    gameState.players
      .filter((player) => academyTeamIds.has(player.team_id ?? ""))
      .map((player) => normalizePlayerKey(player)),
  );
  const userClubTeamIds = new Set<string>();
  if (userTeamId) {
    userClubTeamIds.add(userTeamId);
    gameState.teams
      .filter(
        (team) =>
          team.team_kind === "Academy" &&
          team.parent_team_id === userTeamId,
      )
      .forEach((team) => userClubTeamIds.add(team.id));
  }

  const marketPlayers = dedupePlayersPreferAcademy(
    gameState.players.filter(
      (player) =>
        (player.team_id === null || player.transfer_listed) &&
        player.team_id !== userTeamId &&
        !academyIdentityKeys.has(normalizePlayerKey(player)),
    ),
    academyTeamIds,
  );

  const loanPlayers = dedupePlayersPreferAcademy(
    gameState.players.filter(
      (player) =>
        player.loan_listed &&
        player.team_id !== userTeamId &&
        !academyIdentityKeys.has(normalizePlayerKey(player)),
    ),
    academyTeamIds,
  );

  const erlPlayers = dedupePlayersPreferAcademy(
    gameState.players.filter((player) => {
      const teamId = player.team_id ?? "";
      return academyTeamIds.has(teamId) && !userClubTeamIds.has(teamId);
    }),
    academyTeamIds,
  );

  return {
    myTransferList: gameState.players.filter(
      (player) => player.team_id === userTeamId && player.transfer_listed,
    ),
    myLoanList: gameState.players.filter(
      (player) => player.team_id === userTeamId && player.loan_listed,
    ),
    marketPlayers,
    erlPlayers,
    loanPlayers,
    playersWithOffers: gameState.players.filter(
      (player) =>
        player.transfer_offers.length > 0 &&
        (player.team_id === userTeamId ||
          player.transfer_offers.some(
            (offer) => offer.from_team_id === userTeamId,
          )),
    ),
  };
}

export function getCurrentTransferList(
  view: TransferTabView,
  collections: TransferCollections,
): PlayerData[] {
  switch (view) {
    case "my_list":
      return [...collections.myTransferList, ...collections.myLoanList];
    case "market":
      return collections.marketPlayers;
    case "erl":
      return collections.erlPlayers;
    case "loans":
      return collections.loanPlayers;
    case "offers":
    default:
      return collections.playersWithOffers;
  }
}

export function filterTransferPlayers(
  players: PlayerData[],
  search: string,
  posFilter: string | null,
): PlayerData[] {
  return players.filter((player) => {
    if (posFilter && getLolRoleForPlayer(player) !== posFilter) {
      return false;
    }

    if (search.length >= 2) {
      const query = search.toLowerCase();

      if (
        !player.match_name.toLowerCase().includes(query) &&
        !player.full_name.toLowerCase().includes(query) &&
        !player.nationality.toLowerCase().includes(query)
      ) {
        return false;
      }
    }

    return true;
  });
}
