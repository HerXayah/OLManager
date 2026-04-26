import { invoke } from "@tauri-apps/api/core";

import type {
  AcademyAcquisitionOptionsResponseData,
  AcquireAcademyTeamRequestData,
  GameStateData,
} from "../store/gameStore";

function normalizeAcademyAcquisitionOptionsResponse(
  response: AcademyAcquisitionOptionsResponseData,
): AcademyAcquisitionOptionsResponseData {
  return {
    parent_team_id: response.parent_team_id,
    acquisition_allowed: Boolean(response.acquisition_allowed),
    blocked_reason: response.blocked_reason ?? null,
    options: Array.isArray(response.options) ? response.options : [],
  };
}

export async function getAcademyAcquisitionOptions(
  parentTeamId: string,
): Promise<AcademyAcquisitionOptionsResponseData> {
  const response = await invoke<AcademyAcquisitionOptionsResponseData>(
    "get_academy_acquisition_options",
    {
      parentTeamId,
    },
  );

  return normalizeAcademyAcquisitionOptionsResponse(response);
}

export async function acquireAcademyTeam(
  request: AcquireAcademyTeamRequestData,
): Promise<GameStateData> {
  return invoke<GameStateData>("acquire_academy_team", {
    parentTeamId: request.parent_team_id,
    sourceTeamId: request.source_team_id,
    customName: request.custom_name ?? undefined,
    customShortName: request.custom_short_name ?? undefined,
    customLogoUrl: request.custom_logo_url ?? undefined,
  });
}

export async function getAcademyCreationOptions(parentTeamId: string) {
  return getAcademyAcquisitionOptions(parentTeamId);
}

export async function createAcademy(parentTeamId: string, sourceTeamId: string) {
  return acquireAcademyTeam({
    parent_team_id: parentTeamId,
    source_team_id: sourceTeamId,
  });
}
