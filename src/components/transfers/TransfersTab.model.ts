import type { GameStateData, PlayerData } from "../../store/gameStore";
import { getLolRoleForPlayer } from "../squad/SquadTab.helpers";

export type TransferTabView = "my_list" | "market" | "loans" | "offers";

export interface TransferCollections {
  myTransferList: PlayerData[];
  myLoanList: PlayerData[];
  marketPlayers: PlayerData[];
  loanPlayers: PlayerData[];
  playersWithOffers: PlayerData[];
}

export function deriveTransferCollections(
  gameState: GameStateData,
  userTeamId: string | null,
): TransferCollections {
  return {
    myTransferList: gameState.players.filter(
      (player) => player.team_id === userTeamId && player.transfer_listed,
    ),
    myLoanList: gameState.players.filter(
      (player) => player.team_id === userTeamId && player.loan_listed,
    ),
    marketPlayers: gameState.players.filter(
      (player) => player.transfer_listed && player.team_id !== userTeamId,
    ),
    loanPlayers: gameState.players.filter(
      (player) => player.loan_listed && player.team_id !== userTeamId,
    ),
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
