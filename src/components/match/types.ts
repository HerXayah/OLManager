// Shared types for match simulation components — mirrors Rust engine types

import type { TFunction } from "i18next";

export interface MatchEvent {
  minute: number;
  event_type: string;
  side: "Home" | "Away";
  zone: string;
  player_id: string | null;
  secondary_player_id: string | null;
}

export type LolDragonKind =
  | "Infernal"
  | "Ocean"
  | "Mountain"
  | "Cloud"
  | "Hextech"
  | "Chemtech"
  | "Elder";

export interface LolObjectiveState {
  alive: boolean;
  next_spawn_minute: number | null;
  last_taken_by: "Home" | "Away" | null;
}

export interface LolDragonState {
  alive: boolean;
  next_spawn_minute: number | null;
  current_kind: LolDragonKind | null;
  first_kind: LolDragonKind | null;
  second_kind: LolDragonKind | null;
  soul_rift_kind: LolDragonKind | null;
  soul_claimed_by: "Home" | "Away" | null;
  home_stacks: number;
  away_stacks: number;
  last_taken_by: "Home" | "Away" | null;
}

export interface LolGrubsState {
  alive: boolean;
  next_spawn_minute: number | null;
  waves_taken: number;
  last_taken_by: "Home" | "Away" | null;
}

export interface LolObjectivesState {
  dragon: LolDragonState;
  baron: LolObjectiveState;
  herald: LolObjectiveState;
  grubs: LolGrubsState;
}

export interface LolLaneState {
  outer_alive: boolean;
  outer_hp: number;
  inner_alive: boolean;
  inner_hp: number;
  inhibitor_alive: boolean;
  inhibitor_hp: number;
  inhibitor_respawn_minute: number | null;
}

export interface LolTeamStructuresState {
  top: LolLaneState;
  mid: LolLaneState;
  bot: LolLaneState;
  nexus_tower_top_alive: boolean;
  nexus_tower_top_hp: number;
  nexus_tower_bot_alive: boolean;
  nexus_tower_bot_hp: number;
  nexus_alive: boolean;
  nexus_hp: number;
}

export type LolRole = "Top" | "Jungle" | "Mid" | "Adc" | "Support";

export type LolTask = "MoveToLane" | "JungleClear" | "HoldLane" | "RotateObjective" | "Recall";

export interface LolUnitState {
  player_id: string;
  side: "Home" | "Away";
  role: LolRole;
  task: LolTask;
  x: number;
  y: number;
  target_x: number;
  target_y: number;
  path_index: number;
  recall_available_minute?: number;
  alive: boolean;
  respawn_minute: number | null;
  hp?: number;
  kills: number;
  deaths: number;
}

export interface LolMapState {
  objectives: LolObjectivesState;
  blue: LolTeamStructuresState;
  red: LolTeamStructuresState;
  destroyed_nexus_by: "Home" | "Away" | null;
  units: LolUnitState[];
}

export interface EnginePlayerData {
  id: string;
  name: string;
  position: string;
  condition: number;
  pace: number;
  stamina: number;
  strength: number;
  agility: number;
  passing: number;
  shooting: number;
  tackling: number;
  dribbling: number;
  defending: number;
  positioning: number;
  vision: number;
  decisions: number;
  composure: number;
  aggression: number;
  teamwork: number;
  leadership: number;
  handling: number;
  reflexes: number;
  aerial: number;
  traits: string[];
}

export interface EngineTeamData {
  id: string;
  name: string;
  formation: string;
  play_style: string;
  players: EnginePlayerData[];
}

export interface SetPieceTakers {
  free_kick_taker: string | null;
  corner_taker: string | null;
  penalty_taker: string | null;
  captain: string | null;
}

export interface SubstitutionRecord {
  minute: number;
  side: "Home" | "Away";
  player_off_id: string;
  player_on_id: string;
}

export interface MatchSnapshot {
  phase: string;
  current_minute: number;
  home_score: number;
  away_score: number;
  possession: "Home" | "Away";
  ball_zone: string;
  home_team: EngineTeamData;
  away_team: EngineTeamData;
  home_bench: EnginePlayerData[];
  away_bench: EnginePlayerData[];
  home_possession_pct: number;
  away_possession_pct: number;
  events: MatchEvent[];
  home_subs_made: number;
  away_subs_made: number;
  max_subs: number;
  home_set_pieces: SetPieceTakers;
  away_set_pieces: SetPieceTakers;
  substitutions: SubstitutionRecord[];
  allows_extra_time: boolean;
  home_yellows: Record<string, number>;
  away_yellows: Record<string, number>;
  sent_off: string[];
  lol_map?: LolMapState;
}

export interface MinuteResult {
  minute: number;
  phase: string;
  events: MatchEvent[];
  home_score: number;
  away_score: number;
  possession: "Home" | "Away";
  ball_zone: string;
  is_finished: boolean;
}

export interface RoundResultSummary {
  fixture_id: string;
  home_team_id: string;
  home_team_name: string;
  away_team_id: string;
  away_team_name: string;
  home_goals: number;
  away_goals: number;
}

export interface StandingDelta {
  team_id: string;
  team_name: string;
  previous_position: number;
  current_position: number;
  points: number;
  points_delta: number;
}

export interface NotableUpset {
  fixture_id: string;
  favorite_team_id: string;
  favorite_team_name: string;
  favorite_strength: number;
  underdog_team_id: string;
  underdog_team_name: string;
  underdog_strength: number;
  strength_gap: number;
  home_goals: number;
  away_goals: number;
}

export interface TopScorerDelta {
  player_id: string;
  player_name: string;
  team_id: string;
  previous_rank: number;
  current_rank: number;
  previous_goals: number;
  current_goals: number;
}

export interface RoundSummary {
  matchday: number;
  is_complete: boolean;
  pending_fixture_count: number;
  completed_results: RoundResultSummary[];
  standings_delta: StandingDelta[];
  notable_upset: NotableUpset | null;
  top_scorer_delta: TopScorerDelta[];
}

export type SimSpeed = "paused" | "slow" | "normal" | "fast" | "instant";

export type MatchDayStage =
  | "prematch"
  | "draft"
  | "tactics"
  | "draft_result"
  | "first_half"
  | "halftime"
  | "second_half"
  | "postmatch"
  | "press";

export type TeamTalkTone =
  | "calm"
  | "motivational"
  | "assertive"
  | "aggressive"
  | "praise"
  | "disappointed";

export interface TeamTalkOption {
  id: TeamTalkTone;
  label: string;
  description: string;
  icon: string;
}

const TEAM_TALK_OPTION_DEFINITIONS: Array<{
  id: TeamTalkTone;
  icon: string;
}> = [
  { id: "calm", icon: "calm" },
  { id: "motivational", icon: "motivational" },
  { id: "assertive", icon: "assertive" },
  { id: "aggressive", icon: "aggressive" },
  { id: "praise", icon: "praise" },
  { id: "disappointed", icon: "disappointed" },
];

export function getTeamTalkOptions(t: TFunction): TeamTalkOption[] {
  return TEAM_TALK_OPTION_DEFINITIONS.map(({ id, icon }) => ({
    id,
    icon,
    label: t(`match.teamTalkOptions.${id}.label`),
    description: t(`match.teamTalkOptions.${id}.description`),
  }));
}

export const SPEED_MS: Record<SimSpeed, number> = {
  paused: 0,
  slow: 2000,
  normal: 800,
  fast: 200,
  instant: 10,
};

export const FORMATIONS = ["4-4-2", "4-3-3", "3-5-2", "4-5-1", "4-2-3-1", "3-4-3"];

export const PLAY_STYLES = [
  { id: "Balanced", label: "Balanced" },
  { id: "Attacking", label: "Attacking" },
  { id: "Defensive", label: "Defensive" },
  { id: "Possession", label: "Possession" },
  { id: "Counter", label: "Counter" },
  { id: "HighPress", label: "High Press" },
];
