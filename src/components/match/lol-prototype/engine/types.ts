export type TeamId = "blue" | "red";
export type RoleId = "TOP" | "JGL" | "MID" | "ADC" | "SUP";
export type LaneId = "top" | "mid" | "bot";

export interface Vec2 {
  x: number;
  y: number;
}

export interface ChampionState {
  id: string;
  name: string;
  team: TeamId;
  role: RoleId;
  lane: LaneId;
  pos: Vec2;
  hp: number;
  maxHp: number;
  alive: boolean;
  respawnAt: number;
  attackCdUntil: number;
  moveSpeed: number;
  attackRange: number;
  attackDamage: number;
  targetPath: Vec2[];
  targetPathIndex: number;
  nextDecisionAt: number;
  kills: number;
  deaths: number;
  assists: number;
  gold: number;
  xp: number;
  level: number;
  lastDamagedByChampionId: string | null;
  lastDamagedAt: number;
  state: "lane" | "fight" | "objective" | "recall";
}

export interface MinionState {
  id: string;
  team: TeamId;
  lane: LaneId;
  pos: Vec2;
  hp: number;
  maxHp: number;
  alive: boolean;
  kind: "melee" | "ranged";
  lastHitByChampionId: string | null;
  attackCdUntil: number;
  moveSpeed: number;
  attackRange: number;
  attackDamage: number;
  path: Vec2[];
  pathIndex: number;
}

export interface StructureState {
  id: string;
  team: TeamId;
  lane: LaneId | "base";
  kind: "tower" | "inhib" | "nexus";
  pos: Vec2;
  hp: number;
  maxHp: number;
  alive: boolean;
  attackCdUntil: number;
}

export interface ObjectiveState {
  key: "dragon" | "baron";
  pos: Vec2;
  alive: boolean;
  nextSpawnAt: number;
}

export type NeutralTimerKey =
  | "blue-buff-blue"
  | "blue-buff-red"
  | "red-buff-blue"
  | "red-buff-red"
  | "wolves-blue"
  | "wolves-red"
  | "raptors-blue"
  | "raptors-red"
  | "gromp-blue"
  | "gromp-red"
  | "krugs-blue"
  | "krugs-red"
  | "scuttle-top"
  | "scuttle-bot"
  | "dragon"
  | "voidgrubs"
  | "herald"
  | "baron"
  | "elder";

export interface NeutralTimerState {
  key: NeutralTimerKey;
  label: string;
  alive: boolean;
  hp: number;
  maxHp: number;
  nextSpawnAt: number | null;
  firstSpawnAt: number;
  respawnDelaySec: number | null;
  oneShot: boolean;
  windowCloseAt: number | null;
  combatGraceUntil: number | null;
  unlocked: boolean;
  lastSpawnAt: number | null;
  lastTakenAt: number | null;
  timesSpawned: number;
  timesTaken: number;
  pos: Vec2;
}

export interface NeutralTimersState {
  dragonSoulUnlocked: boolean;
  elderUnlocked: boolean;
  entities: Record<NeutralTimerKey, NeutralTimerState>;
}

export interface TeamStats {
  kills: number;
  towers: number;
  dragons: number;
  barons: number;
  gold: number;
}

export interface SimEvent {
  t: number;
  text: string;
  type:
    | "kill"
    | "tower"
    | "dragon"
    | "baron"
    | "nexus"
    | "spawn"
    | "recall"
    | "info";
}

export interface MatchState {
  timeSec: number;
  running: boolean;
  winner: TeamId | null;
  champions: ChampionState[];
  minions: MinionState[];
  structures: StructureState[];
  objectives: Record<"dragon" | "baron", ObjectiveState>;
  neutralTimers: NeutralTimersState;
  stats: Record<TeamId, TeamStats>;
  events: SimEvent[];
  showWalls: boolean;
}
