import { invoke } from "@tauri-apps/api/core";

import {
  normalizeOptionalTrainingFocus,
  normalizeTrainingFocus,
} from "../lib/trainingFocus";
import type { GameStateData } from "../store/gameStore";

export interface TrainingGroupData {
  id: string;
  name: string;
  focus: string;
  player_ids: string[];
}

export async function setTraining(
  focus: string,
  intensity: string,
): Promise<GameStateData> {
  return invoke<GameStateData>("set_training", {
    focus: normalizeTrainingFocus(focus),
    intensity,
  });
}

export async function setTrainingSchedule(
  schedule: string,
): Promise<GameStateData> {
  return invoke<GameStateData>("set_training_schedule", {
    schedule,
  });
}

export async function setTrainingGroups(
  groups: TrainingGroupData[],
): Promise<GameStateData> {
  return invoke<GameStateData>("set_training_groups", {
    groups: groups.map((group) => ({
      ...group,
      focus: normalizeTrainingFocus(group.focus),
    })),
  });
}

export async function setWeeklyScrims(
  opponentTeamIds: string[],
): Promise<GameStateData> {
  return invoke<GameStateData>("set_weekly_scrims", {
    opponentTeamIds,
  });
}

export async function setPlayerTrainingFocus(
  playerId: string,
  focus: string | null,
): Promise<GameStateData> {
  return invoke<GameStateData>("set_player_training_focus", {
    playerId,
    focus: normalizeOptionalTrainingFocus(focus),
  });
}
