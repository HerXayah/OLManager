import { invoke } from "@tauri-apps/api/core";
import type { GameStateData } from "../store/gameStore";

export async function startPotentialResearch(
  playerId: string,
): Promise<GameStateData> {
  return invoke<GameStateData>("start_potential_research", { playerId });
}

export async function setPlayerChampionTrainingTarget(
  playerId: string,
  priorityIndex: number,
  championId: string | null,
): Promise<GameStateData> {
  return invoke<GameStateData>("set_player_champion_training_target", {
    playerId,
    priorityIndex,
    championId,
  });
}
