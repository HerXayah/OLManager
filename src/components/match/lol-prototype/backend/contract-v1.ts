import type { MatchSnapshot } from "../../types";
import type { ChampionCombatProfile } from "../engine/simulation";
import type { MatchState, TeamId } from "../engine/types";

/**
 * Simulation Contract v1 for LoL simulator migration (TS <-> Tauri Rust).
 *
 * Contract-first goal: preserve current UI behavior while moving runtime ownership
 * to Rust incrementally.
 */

export type LolSimV1EventType = "kill" | "tower" | "dragon" | "baron" | "nexus" | "spawn" | "recall" | "info";

export interface LolSimV1SimEvent {
  t: number;
  text: string;
  type: LolSimV1EventType;
}

export interface LolSimV1TeamStats {
  kills: number;
  towers: number;
  dragons: number;
  barons: number;
  gold: number;
}

/**
 * Runtime state subset currently consumed by LolMatchLive + render/panels.
 * NOTE: Keep aligned with fields actually read by UI, not full engine internals.
 */
export type LolSimV1RuntimeState = Pick<
  MatchState,
  "timeSec" | "running" | "winner" | "showWalls" | "champions" | "minions" | "structures" | "objectives" | "neutralTimers" | "stats" | "events"
> & {
  speed: number;
};

export interface LolSimV1InitRequest {
  sessionId: string;
  seed: string;
  snapshot: MatchSnapshot;
  championByPlayerId: Record<string, string>;
  championProfilesById: Record<string, ChampionCombatProfile>;
  /**
   * Legacy bootstrap field kept for backwards compatibility.
   * Rust v2 now creates state natively and ignores this payload.
   */
  initialState?: LolSimV1RuntimeState;
}

export interface LolSimV1StateResponse {
  sessionId: string;
  state: LolSimV1RuntimeState;
}

export interface LolSimV1TickRequest {
  sessionId: string;
  dtSec: number;
  running: boolean;
  speed: number;
}

export interface LolSimV1ResetRequest {
  sessionId: string;
  seed: string;
  initialState?: LolSimV1RuntimeState;
}

export interface LolSimV1DisposeRequest {
  sessionId: string;
}

export interface LolSimV1DisposeResponse {
  sessionId: string;
  disposed: boolean;
}

export interface LolSimV1ControlSnapshot {
  sessionId: string;
  seed: string;
  running: boolean;
  speed: number;
  winner: TeamId | null;
}
