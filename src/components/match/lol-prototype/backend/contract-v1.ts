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
export type LolSimV1AiMode = "rules" | "hybrid";

export interface LolSimV1TelemetryConfig {
  enabled?: boolean;
  sampleEveryTicks?: number;
  outcomeWindowTicks?: number;
  decisionChangeOnly?: boolean;
  outputPath?: string;
}

export interface LolChampionUltimateProfile {
  archetype: string;
  icon: string;
}

export interface LolSimV1PolicyConfig {
  noDiveHpMin?: number;
  tradeRetreatHpRatio?: number;
  tradeHpDisadvantageAllowance?: number;
  laneChaseLeashRadius?: number;
  hybridOpenTradeConfidenceHigh?: number;
  hybridDisengageConfidenceLow?: number;
}

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
  goldDiffTimeline?: Array<{ minute: number; diff: number }>;
};

export interface LolSimV1InitRequest {
  sessionId: string;
  seed: string;
  aiMode?: LolSimV1AiMode;
  policy?: LolSimV1PolicyConfig;
  telemetry?: LolSimV1TelemetryConfig;
  snapshot: MatchSnapshot;
  championByPlayerId: Record<string, string>;
  championProfilesById: Record<string, ChampionCombatProfile>;
  championUltimatesById?: Record<string, LolChampionUltimateProfile>;
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
  aiMode?: LolSimV1AiMode;
  policy?: LolSimV1PolicyConfig;
  telemetry?: LolSimV1TelemetryConfig;
  initialState?: LolSimV1RuntimeState;
}

export interface LolSimV1DisposeRequest {
  sessionId: string;
}

export interface LolSimV1DisposeResponse {
  sessionId: string;
  disposed: boolean;
}

export interface LolSimV1RunToCompletionRequest {
  seed: string;
  aiMode?: LolSimV1AiMode;
  policy?: LolSimV1PolicyConfig;
  telemetry?: LolSimV1TelemetryConfig;
  snapshot: MatchSnapshot;
  championByPlayerId: Record<string, string>;
  championProfilesById: Record<string, ChampionCombatProfile>;
  championUltimatesById?: Record<string, LolChampionUltimateProfile>;
  dtSec: number;
  speed: number;
  maxTicks: number;
}

export interface LolSimV1RunToCompletionResponse {
  winner: TeamId | null;
  ticks: number;
  elapsedSimulatedSec: number;
}

export interface LolSimV1SkipToEndRequest {
  sessionId: string;
  dtSec?: number;
  speed?: number;
  maxTicks?: number;
}

export interface LolSimV1SkipToEndResponse {
  sessionId: string;
  state: LolSimV1RuntimeState;
  winner: TeamId | null;
  ticks: number;
  elapsedSimulatedSec: number;
}

export interface LolSimV1MatchReportEventInput {
  t: number;
  text: string;
  type: LolSimV1EventType;
}

export interface LolSimV1MatchReportTeamStatsInput {
  kills: number;
  deaths: number;
  gold: number;
  towers: number;
  dragons: number;
  barons: number;
}

export interface LolSimV1MatchReportChampionInput {
  id: string;
  name: string;
  team: TeamId;
  role: "TOP" | "JGL" | "MID" | "ADC" | "SUP";
  kills: number;
  deaths: number;
  assists: number;
  cs: number;
  gold: number;
  spentGold: number;
}

export interface LolSimV1MatchReportInput {
  winner: TeamId | null;
  timeSec: number;
  events: LolSimV1MatchReportEventInput[];
  stats: {
    blue: LolSimV1MatchReportTeamStatsInput;
    red: LolSimV1MatchReportTeamStatsInput;
  };
  champions: LolSimV1MatchReportChampionInput[];
}

export interface LolSimV1ControlSnapshot {
  sessionId: string;
  seed: string;
  running: boolean;
  speed: number;
  winner: TeamId | null;
}
