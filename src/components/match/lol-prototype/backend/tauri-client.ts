import { invoke } from "@tauri-apps/api/core";

import type {
  LolSimV1DisposeRequest,
  LolSimV1DisposeResponse,
  LolSimV1InitRequest,
  LolSimV1RunToCompletionRequest,
  LolSimV1RunToCompletionResponse,
  LolSimV1ResetRequest,
  LolSimV1StateResponse,
  LolSimV1TickRequest,
} from "./contract-v1";

export async function lolSimV2Init(request: LolSimV1InitRequest): Promise<LolSimV1StateResponse> {
  return invoke<LolSimV1StateResponse>("lol_sim_v2_init", { request });
}

export async function lolSimV2Tick(request: LolSimV1TickRequest): Promise<LolSimV1StateResponse> {
  return invoke<LolSimV1StateResponse>("lol_sim_v2_tick", { request });
}

export async function lolSimV2Reset(request: LolSimV1ResetRequest): Promise<LolSimV1StateResponse> {
  return invoke<LolSimV1StateResponse>("lol_sim_v2_reset", { request });
}

export async function lolSimV2Dispose(request: LolSimV1DisposeRequest): Promise<LolSimV1DisposeResponse> {
  return invoke<LolSimV1DisposeResponse>("lol_sim_v2_dispose", { request });
}

export async function lolSimV2RunToCompletion(
  request: LolSimV1RunToCompletionRequest,
): Promise<LolSimV1RunToCompletionResponse> {
  return invoke<LolSimV1RunToCompletionResponse>("lol_sim_v2_run_to_completion", { request });
}

export interface LolSimV2ClearTelemetryResponse {
  directory: string;
  deleted_files: number;
  skipped_entries: number;
  existed: boolean;
}

export async function lolSimV2ClearTelemetryFiles(): Promise<LolSimV2ClearTelemetryResponse> {
  return invoke<LolSimV2ClearTelemetryResponse>("lol_sim_v2_clear_telemetry_files");
}

function createSessionId() {
  return `lol-sim-v2-${Date.now()}-${Math.floor(Math.random() * 1_000_000)}`;
}

export class LolSimV2Client {
  readonly sessionId: string;

  constructor(sessionId?: string) {
    this.sessionId = sessionId ?? createSessionId();
  }

  async init(request: Omit<LolSimV1InitRequest, "sessionId">): Promise<LolSimV1StateResponse> {
    return lolSimV2Init({ ...request, sessionId: this.sessionId });
  }

  async tick(request: Omit<LolSimV1TickRequest, "sessionId">): Promise<LolSimV1StateResponse> {
    return lolSimV2Tick({ ...request, sessionId: this.sessionId });
  }

  async reset(request: Omit<LolSimV1ResetRequest, "sessionId">): Promise<LolSimV1StateResponse> {
    return lolSimV2Reset({ ...request, sessionId: this.sessionId });
  }

  async dispose(): Promise<LolSimV1DisposeResponse> {
    return lolSimV2Dispose({ sessionId: this.sessionId });
  }
}
