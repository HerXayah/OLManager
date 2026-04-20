import { invoke } from "@tauri-apps/api/core";

import type { GameStateData } from "../store/gameStore";

export async function sendScout(
  scoutId: string,
  playerId: string,
): Promise<GameStateData> {
  return invoke<GameStateData>("send_scout", {
    scoutId,
    playerId,
  });
}