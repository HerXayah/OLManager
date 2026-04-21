import { BASE_POSITION, JUNGLE_ROUTE, LANE_PATH_BLUE, OBJECTIVES, ROLE_OFFSET, getStructures } from "../assets/map";
import type { MatchSnapshot } from "../../types";
import { JUNGLE_CAMPS_LAYOUT } from "../../../../lib/lolMapLayout";
import { NavGrid } from "./navigation";
import type {
  ChampionState,
  LaneId,
  MatchState,
  MinionState,
  NeutralTimerKey,
  NeutralTimerState,
  RoleId,
  SimEvent,
  StructureState,
  TeamId,
  Vec2,
} from "./types";

const CHAMPION_KILL_GOLD = 300;
const CHAMPION_ASSIST_GOLD_TOTAL = 150;
const CHAMPION_KILL_XP = 220;
const ASSIST_RADIUS = 0.11;
const MINION_GOLD = { melee: 22, ranged: 16 } as const;
const MINION_XP = { melee: 58, ranged: 32 } as const;
const MINION_PROFILE = {
  melee: { maxHp: 46, moveSpeed: 0.068, attackRange: 0.035, attackDamage: 7, attackCadence: 0.98 },
  ranged: { maxHp: 30, moveSpeed: 0.071, attackRange: 0.055, attackDamage: 9, attackCadence: 1.06 },
} as const;
const MINION_FIRST_WAVE_AT = 30;
const LOCAL_COMBAT_ENGAGE_RADIUS = 0.16;
const LOCAL_STRUCTURE_ENGAGE_RADIUS = 0.12;
const LANE_STRUCTURE_PRESSURE_RADIUS = 0.08;
const JUNGLE_CAMP_ENGAGE_RADIUS = 0.09;
const STUCK_PROGRESS_WINDOW_SEC = 1.4;
const STUCK_MIN_PROGRESS = 0.004;
const STUCK_MIN_REMAINING_DELTA = 0.004;
const STUCK_REPATH_COOLDOWN_SEC = 2.2;

const JUNGLE_CAMP_KEYS: NeutralTimerKey[] = [
  "blue-buff-blue",
  "blue-buff-red",
  "red-buff-blue",
  "red-buff-red",
  "wolves-blue",
  "wolves-red",
  "raptors-blue",
  "raptors-red",
  "gromp-blue",
  "gromp-red",
  "krugs-blue",
  "krugs-red",
];

const JUNGLE_ROUTE_BY_TIMER: Record<TeamId, NeutralTimerKey[]> = {
  blue: ["blue-buff-blue", "gromp-blue", "wolves-blue", "raptors-blue", "red-buff-blue", "krugs-blue", "scuttle-top", "scuttle-bot"],
  red: ["red-buff-red", "krugs-red", "raptors-red", "wolves-red", "blue-buff-red", "gromp-red", "scuttle-bot", "scuttle-top"],
};

const JUNGLE_CAMP_PROFILE: Partial<Record<NeutralTimerKey, { maxHp: number; gold: number; xp: number }>> = {
  "blue-buff-blue": { maxHp: 470, gold: 95, xp: 150 },
  "blue-buff-red": { maxHp: 470, gold: 95, xp: 150 },
  "red-buff-blue": { maxHp: 500, gold: 95, xp: 155 },
  "red-buff-red": { maxHp: 500, gold: 95, xp: 155 },
  "wolves-blue": { maxHp: 380, gold: 70, xp: 110 },
  "wolves-red": { maxHp: 380, gold: 70, xp: 110 },
  "raptors-blue": { maxHp: 390, gold: 72, xp: 115 },
  "raptors-red": { maxHp: 390, gold: 72, xp: 115 },
  "gromp-blue": { maxHp: 520, gold: 82, xp: 128 },
  "gromp-red": { maxHp: 520, gold: 82, xp: 128 },
  "krugs-blue": { maxHp: 560, gold: 86, xp: 132 },
  "krugs-red": { maxHp: 560, gold: 86, xp: 132 },
  "scuttle-top": { maxHp: 560, gold: 70, xp: 110 },
  "scuttle-bot": { maxHp: 560, gold: 70, xp: 110 },
};

function isJungleCampKey(key: NeutralTimerKey) {
  return JUNGLE_CAMP_KEYS.includes(key);
}

function neutralDefaultMaxHp(key: NeutralTimerKey) {
  if (key === "dragon") return 3600;
  if (key === "baron") return 9000;
  if (key === "herald") return 5500;
  if (key === "voidgrubs") return 2800;
  if (key === "elder") return 7200;
  return JUNGLE_CAMP_PROFILE[key]?.maxHp ?? 1200;
}

const CAMP_POSITIONS = new Map(JUNGLE_CAMPS_LAYOUT.map((camp) => [camp.id, { x: camp.x, y: camp.y }]));
const VOIDGRUBS_SOFT_CLOSE_AT = 14 * 60 + 45;
const VOIDGRUBS_HARD_CLOSE_AT = 14 * 60 + 55;
const HERALD_SOFT_CLOSE_AT = 19 * 60 + 45;
const HERALD_HARD_CLOSE_AT = 19 * 60 + 55;

const NEUTRAL_TIMER_TEMPLATE: Record<NeutralTimerKey, Omit<NeutralTimerState, "alive" | "hp" | "nextSpawnAt" | "lastSpawnAt" | "lastTakenAt" | "timesSpawned" | "timesTaken">> = {
  "blue-buff-blue": {
    key: "blue-buff-blue",
    label: "Blue Blue Buff",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("blue-buff-blue"),
    respawnDelaySec: 5 * 60,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("blue-blue-buff") ?? { x: 0.25, y: 0.46 },
  },
  "blue-buff-red": {
    key: "blue-buff-red",
    label: "Red Blue Buff",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("blue-buff-red"),
    respawnDelaySec: 5 * 60,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("red-blue-buff") ?? { x: 0.48, y: 0.26 },
  },
  "red-buff-blue": {
    key: "red-buff-blue",
    label: "Blue Red Buff",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("red-buff-blue"),
    respawnDelaySec: 5 * 60,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("blue-red-buff") ?? { x: 0.53, y: 0.74 },
  },
  "red-buff-red": {
    key: "red-buff-red",
    label: "Red Red Buff",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("red-buff-red"),
    respawnDelaySec: 5 * 60,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("red-red-buff") ?? { x: 0.75, y: 0.54 },
  },
  "wolves-blue": {
    key: "wolves-blue",
    label: "Blue Wolves",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("wolves-blue"),
    respawnDelaySec: 135,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("blue-wolves") ?? { x: 0.26, y: 0.56 },
  },
  "wolves-red": {
    key: "wolves-red",
    label: "Red Wolves",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("wolves-red"),
    respawnDelaySec: 135,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("red-wolves") ?? { x: 0.53, y: 0.35 },
  },
  "raptors-blue": {
    key: "raptors-blue",
    label: "Blue Raptors",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("raptors-blue"),
    respawnDelaySec: 135,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("blue-raptors") ?? { x: 0.48, y: 0.64 },
  },
  "raptors-red": {
    key: "raptors-red",
    label: "Red Raptors",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("raptors-red"),
    respawnDelaySec: 135,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("red-raptors") ?? { x: 0.75, y: 0.44 },
  },
  "gromp-blue": {
    key: "gromp-blue",
    label: "Blue Gromp",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("gromp-blue"),
    respawnDelaySec: 135,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("blue-gromp") ?? { x: 0.15, y: 0.43 },
  },
  "gromp-red": {
    key: "gromp-red",
    label: "Red Gromp",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("gromp-red"),
    respawnDelaySec: 135,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("red-gromp") ?? { x: 0.44, y: 0.17 },
  },
  "krugs-blue": {
    key: "krugs-blue",
    label: "Blue Krugs",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("krugs-blue"),
    respawnDelaySec: 135,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("blue-krugs") ?? { x: 0.57, y: 0.83 },
  },
  "krugs-red": {
    key: "krugs-red",
    label: "Red Krugs",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("krugs-red"),
    respawnDelaySec: 135,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("red-krugs") ?? { x: 0.85, y: 0.57 },
  },
  "scuttle-top": {
    key: "scuttle-top",
    label: "Scuttle Top",
    firstSpawnAt: 175,
    maxHp: neutralDefaultMaxHp("scuttle-top"),
    respawnDelaySec: 150,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("river-scuttle-top") ?? { x: 0.285, y: 0.348 },
  },
  "scuttle-bot": {
    key: "scuttle-bot",
    label: "Scuttle Bot",
    firstSpawnAt: 175,
    maxHp: neutralDefaultMaxHp("scuttle-bot"),
    respawnDelaySec: 150,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: CAMP_POSITIONS.get("river-scuttle-bot") ?? { x: 0.7, y: 0.642 },
  },
  dragon: {
    key: "dragon",
    label: "Dragon",
    firstSpawnAt: 5 * 60,
    maxHp: neutralDefaultMaxHp("dragon"),
    respawnDelaySec: 5 * 60,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: OBJECTIVES.dragon,
  },
  voidgrubs: {
    key: "voidgrubs",
    label: "Voidgrubs",
    firstSpawnAt: 8 * 60,
    maxHp: neutralDefaultMaxHp("voidgrubs"),
    respawnDelaySec: null,
    oneShot: true,
    windowCloseAt: VOIDGRUBS_SOFT_CLOSE_AT,
    combatGraceUntil: VOIDGRUBS_HARD_CLOSE_AT,
    unlocked: true,
    pos: OBJECTIVES.baron,
  },
  herald: {
    key: "herald",
    label: "Rift Herald",
    firstSpawnAt: 15 * 60,
    maxHp: neutralDefaultMaxHp("herald"),
    respawnDelaySec: null,
    oneShot: true,
    windowCloseAt: HERALD_SOFT_CLOSE_AT,
    combatGraceUntil: HERALD_HARD_CLOSE_AT,
    unlocked: true,
    pos: OBJECTIVES.baron,
  },
  baron: {
    key: "baron",
    label: "Baron",
    firstSpawnAt: 20 * 60,
    maxHp: neutralDefaultMaxHp("baron"),
    respawnDelaySec: 6 * 60,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: true,
    pos: OBJECTIVES.baron,
  },
  elder: {
    key: "elder",
    label: "Elder Dragon",
    firstSpawnAt: 0,
    maxHp: neutralDefaultMaxHp("elder"),
    respawnDelaySec: 6 * 60,
    oneShot: false,
    windowCloseAt: null,
    combatGraceUntil: null,
    unlocked: false,
    pos: OBJECTIVES.dragon,
  },
};

function clamp(v: number, min: number, max: number) {
  return Math.max(min, Math.min(max, v));
}
function dist(a: Vec2, b: Vec2) {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

function normalize(v: Vec2) {
  const m = Math.hypot(v.x, v.y);
  if (m < 1e-6) return { x: 1, y: 0 };
  return { x: v.x / m, y: v.y / m };
}

function hashSeed(seed: string) {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < seed.length; i += 1) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function mulberry32(a: number) {
  return () => {
    let t = (a += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export class PrototypeSimulation {
  state: MatchState;
  private rng: () => number;
  private waveSpawnAt = MINION_FIRST_WAVE_AT;
  private nextMinionId = 0;
  private championStuckState = new Map<string, { sampleAt: number; samplePos: Vec2; sampleRemaining: number; cooldownUntil: number }>();
  private jungleCampFocusByChampion = new Map<string, NeutralTimerKey>();
  private jungleRouteIndexByChampion = new Map<string, number>();

  constructor(private nav: NavGrid, private snapshot: MatchSnapshot, seed = "default-seed") {
    this.rng = mulberry32(hashSeed(seed));
    this.state = this.createInitialState();
  }

  reset(seed: string) {
    this.rng = mulberry32(hashSeed(seed));
    this.waveSpawnAt = MINION_FIRST_WAVE_AT;
    this.nextMinionId = 0;
    this.championStuckState.clear();
    this.jungleCampFocusByChampion.clear();
    this.jungleRouteIndexByChampion.clear();
    this.state = this.createInitialState();
  }

  setRunning(running: boolean) {
    this.state.running = running;
  }

  toggleWalls() {
    this.state.showWalls = !this.state.showWalls;
  }

  private laneOf(role: RoleId): LaneId {
    if (role === "TOP") return "top";
    if (role === "MID") return "mid";
    return "bot";
  }

  private xpToNextLevel(level: number) {
    return 110 + level * 70;
  }

  private addGold(ch: ChampionState, amount: number) {
    if (amount <= 0) return;
    ch.gold += amount;
    this.state.stats[ch.team].gold += amount;
  }

  private addXp(ch: ChampionState, amount: number) {
    if (amount <= 0) return;
    ch.xp += amount;
    while (ch.level < 18 && ch.xp >= this.xpToNextLevel(ch.level)) {
      ch.xp -= this.xpToNextLevel(ch.level);
      ch.level += 1;
      ch.maxHp += 18;
      ch.attackDamage += 2.2;
      ch.hp = Math.min(ch.maxHp, ch.hp + ch.maxHp * 0.22);
      this.log(`${ch.name} reached level ${ch.level}`, "info");
    }
  }

  private nearbyChampions(team: TeamId, pos: Vec2, radius: number, aliveOnly = true) {
    return this.state.champions.filter((c) => c.team === team && (!aliveOnly || c.alive) && dist(c.pos, pos) <= radius);
  }

  private distributeMinionXp(dead: MinionState) {
    const nearby = this.nearbyChampions(dead.team === "blue" ? "red" : "blue", dead.pos, 0.12);
    if (!nearby.length) return;
    const each = MINION_XP[dead.kind] / nearby.length;
    nearby.forEach((ch) => this.addXp(ch, each));
  }

  private registerMinionDeath(minion: MinionState, killerChampionId: string | null) {
    if (!minion.alive) return;
    minion.alive = false;
    minion.lastHitByChampionId = killerChampionId;
    this.distributeMinionXp(minion);
    if (!killerChampionId) return;
    const killer = this.state.champions.find((c) => c.id === killerChampionId && c.alive);
    if (!killer) return;
    this.addGold(killer, MINION_GOLD[minion.kind]);
  }

  private shouldAvoidDive(ch: ChampionState, targetPos: Vec2) {
    const tower = this.state.structures.find(
      (s) => s.alive && s.team !== ch.team && s.kind === "tower" && dist(s.pos, targetPos) <= 0.095,
    );
    if (!tower) return false;
    const alliedMinionsNearTower = this.state.minions.filter((m) => m.alive && m.team === ch.team && dist(m.pos, tower.pos) <= 0.085).length;
    const allyNearby = this.nearbyChampions(ch.team, targetPos, 0.12).length;
    const enemyNearby = this.nearbyChampions(ch.team === "blue" ? "red" : "blue", targetPos, 0.12).length;
    const lowHp = ch.hp / ch.maxHp < 0.48;
    return lowHp || alliedMinionsNearTower === 0 || allyNearby < enemyNearby;
  }

  private isLocalCombatTarget(
    ch: ChampionState,
    target:
      | { kind: "champion"; target: ChampionState }
      | { kind: "minion"; target: MinionState }
      | { kind: "structure"; target: StructureState }
      | { kind: "neutral"; target: NeutralTimerState },
  ) {
    const targetDistance = dist(ch.pos, target.target.pos);
    if (targetDistance > LOCAL_COMBAT_ENGAGE_RADIUS) return false;
    if (target.kind === "structure" && targetDistance > LOCAL_STRUCTURE_ENGAGE_RADIUS) return false;
    if (target.kind === "neutral" && targetDistance > JUNGLE_CAMP_ENGAGE_RADIUS) return false;
    return true;
  }

  private pickCombatTarget(ch: ChampionState, now: number):
    | { kind: "champion"; target: ChampionState }
    | { kind: "minion"; target: MinionState }
    | { kind: "structure"; target: StructureState }
    | { kind: "neutral"; target: NeutralTimerState }
    | null {
    const enemyTeam: TeamId = ch.team === "blue" ? "red" : "blue";

    if (ch.role === "JGL") {
      const nearbyCamp = this.nearbyNeutralCamp(ch);
      if (nearbyCamp) return { kind: "neutral", target: nearbyCamp };
      const nearbyEnemyChampion = this.state.champions
        .filter((enemy) => enemy.alive && enemy.team === enemyTeam && dist(ch.pos, enemy.pos) <= 0.13)
        .sort((a, b) => dist(ch.pos, a.pos) - dist(ch.pos, b.pos))[0];
      if (nearbyEnemyChampion) return { kind: "champion", target: nearbyEnemyChampion };
      return null;
    }

    const threateningEnemy = this.state.champions
      .filter((enemy) => {
        if (!enemy.alive || enemy.team !== enemyTeam || dist(ch.pos, enemy.pos) > 0.12) return false;
        return this.state.champions.some(
          (ally) =>
            ally.alive &&
            ally.team === ch.team &&
            dist(ally.pos, ch.pos) <= 0.12 &&
            ally.lastDamagedByChampionId === enemy.id &&
            now - ally.lastDamagedAt <= 2.4,
        );
      })
      .sort((a, b) => dist(ch.pos, a.pos) - dist(ch.pos, b.pos))[0];
    if (threateningEnemy) return { kind: "champion", target: threateningEnemy };

    const laneSkirmishEnemy = this.state.champions
      .filter(
        (enemy) =>
          enemy.alive &&
          enemy.team === enemyTeam &&
          enemy.lane === ch.lane &&
          dist(ch.pos, enemy.pos) <= 0.145 &&
          ch.hp / ch.maxHp >= 0.35,
      )
      .sort((a, b) => dist(ch.pos, a.pos) - dist(ch.pos, b.pos))[0];
    if (laneSkirmishEnemy) return { kind: "champion", target: laneSkirmishEnemy };

    const skirmishEnemy = this.state.champions
      .filter((enemy) => {
        if (!enemy.alive || enemy.team !== enemyTeam || enemy.lane !== ch.lane) return false;
        if (dist(ch.pos, enemy.pos) > 0.11) return false;
        const hpAdvantage = ch.hp / ch.maxHp >= enemy.hp / enemy.maxHp;
        const alliedLaneUnitsNearby = this.state.minions.filter((m) => m.alive && m.team === ch.team && m.lane === ch.lane && dist(m.pos, ch.pos) <= 0.075).length;
        return hpAdvantage || alliedLaneUnitsNearby >= 2;
      })
      .sort((a, b) => dist(ch.pos, a.pos) - dist(ch.pos, b.pos))[0];
    if (skirmishEnemy) return { kind: "champion", target: skirmishEnemy };

    const lastHit = this.state.minions
      .filter(
        (m) =>
          m.alive &&
          m.team === enemyTeam &&
          m.lane === ch.lane &&
          dist(ch.pos, m.pos) <= 0.1 &&
          m.hp <= ch.attackDamage * 1.05,
      )
      .sort((a, b) => a.hp - b.hp || dist(ch.pos, a.pos) - dist(ch.pos, b.pos))[0];
    if (lastHit) return { kind: "minion", target: lastHit };

    const nearestStructure = this.state.structures
      .filter((s) => {
        if (!s.alive || s.team !== enemyTeam || (s.lane !== ch.lane && s.kind !== "nexus")) return false;
        // Laners should pressure structures only when truly nearby and with allied wave support.
        if (ch.role !== "JGL") {
          if (dist(ch.pos, s.pos) > LANE_STRUCTURE_PRESSURE_RADIUS) return false;
          const hasAlliedWaveAtStructure = this.state.minions.some(
            (m) => m.alive && m.team === ch.team && m.lane === ch.lane && dist(m.pos, s.pos) <= 0.09,
          );
          if (!hasAlliedWaveAtStructure) return false;
        }
        return true;
      })
      .sort((a, b) => dist(ch.pos, a.pos) - dist(ch.pos, b.pos))[0];
    const nearestMinion = this.state.minions
      .filter((m) => m.alive && m.team === enemyTeam && m.lane === ch.lane)
      .sort((a, b) => dist(ch.pos, a.pos) - dist(ch.pos, b.pos))[0];
    const nearestEnemyChampion = this.state.champions
      .filter((enemy) => enemy.alive && enemy.team === enemyTeam && dist(ch.pos, enemy.pos) <= 0.19)
      .sort((a, b) => dist(ch.pos, a.pos) - dist(ch.pos, b.pos))[0];
    const nearbyCamp = this.nearbyNeutralCamp(ch);

    const candidates: Array<{
      kind: "champion" | "minion" | "structure" | "neutral";
      target: ChampionState | MinionState | StructureState | NeutralTimerState;
      d: number;
    }> = [];
    if (nearbyCamp) candidates.push({ kind: "neutral", target: nearbyCamp, d: dist(ch.pos, nearbyCamp.pos) - 0.015 });
    if (nearestEnemyChampion) candidates.push({ kind: "champion", target: nearestEnemyChampion, d: dist(ch.pos, nearestEnemyChampion.pos) });
    if (nearestMinion) candidates.push({ kind: "minion", target: nearestMinion, d: dist(ch.pos, nearestMinion.pos) });
    if (nearestStructure) candidates.push({ kind: "structure", target: nearestStructure, d: dist(ch.pos, nearestStructure.pos) });

    if (!candidates.length) return null;
    const selected = candidates.sort((a, b) => a.d - b.d)[0];
    if (selected.kind === "champion") return { kind: "champion", target: selected.target as ChampionState };
    if (selected.kind === "minion") return { kind: "minion", target: selected.target as MinionState };
    if (selected.kind === "neutral") return { kind: "neutral", target: selected.target as NeutralTimerState };
    return { kind: "structure", target: selected.target as StructureState };
  }

  private createInitialState(): MatchState {
    const champions: ChampionState[] = [];
    const seedTeam = (team: TeamId, players: MatchSnapshot["home_team"]["players"]) => {
      const ordered: RoleId[] = ["TOP", "JGL", "MID", "ADC", "SUP"];
      players.slice(0, 5).forEach((p, i) => {
        const role = ordered[i];
        const b = BASE_POSITION[team];
        const o = ROLE_OFFSET[role];
        champions.push({
          id: p.id,
          name: p.name,
          team,
          role,
          lane: this.laneOf(role),
          pos: { x: b.x + o.x, y: b.y + o.y },
          hp: 140,
          maxHp: 140,
          alive: true,
          respawnAt: 0,
          attackCdUntil: 0,
          moveSpeed: 0.043 + this.rng() * 0.008,
          attackRange: 0.05,
          attackDamage: 14 + this.rng() * 5,
          targetPath: [],
          targetPathIndex: 0,
          nextDecisionAt: 0,
          kills: 0,
          deaths: 0,
          assists: 0,
          gold: 500,
          xp: 0,
          level: 1,
          lastDamagedByChampionId: null,
          lastDamagedAt: -999,
          state: "lane",
        });
      });
    };
    seedTeam("blue", this.snapshot.home_team.players);
    seedTeam("red", this.snapshot.away_team.players);

    const structures: StructureState[] = getStructures().map((s) => ({
      id: s.id,
      team: s.team,
      lane: s.lane,
      kind: s.kind as StructureState["kind"],
      pos: s.pos,
      hp: s.kind === "nexus" ? 2300 : s.kind === "inhib" ? 1500 : 1400,
      maxHp: s.kind === "nexus" ? 2300 : s.kind === "inhib" ? 1500 : 1400,
      alive: true,
      attackCdUntil: 0,
    }));

    const pushEvent = (events: SimEvent[], text: string, type: SimEvent["type"]) => {
      events.unshift({ t: 0, text, type });
    };

    const events: SimEvent[] = [];
    pushEvent(events, "Match started", "info");
    const neutralTimers = this.buildNeutralTimersState();

    return {
      timeSec: 0,
      running: true,
      winner: null,
      champions,
      minions: [],
      structures,
      objectives: {
        dragon: { key: "dragon", pos: OBJECTIVES.dragon, alive: false, nextSpawnAt: 5 * 60 },
        baron: { key: "baron", pos: OBJECTIVES.baron, alive: false, nextSpawnAt: 20 * 60 },
      },
      neutralTimers,
      stats: {
        blue: { kills: 0, towers: 0, dragons: 0, barons: 0, gold: 2500 },
        red: { kills: 0, towers: 0, dragons: 0, barons: 0, gold: 2500 },
      },
      events,
      showWalls: false,
    };
  }

  private log(text: string, type: SimEvent["type"]) {
    this.state.events.unshift({ t: this.state.timeSec, text, type });
    this.state.events = this.state.events.slice(0, 80);
  }

  private lanePath(team: TeamId, lane: LaneId) {
    return team === "blue" ? LANE_PATH_BLUE[lane] : [...LANE_PATH_BLUE[lane]].reverse();
  }

  private setChampionPath(ch: ChampionState, target: Vec2, minTargetDelta = 0.018, force = false) {
    const currentTarget = ch.targetPath[ch.targetPath.length - 1];
    const shouldRepath =
      force ||
      !currentTarget ||
      ch.targetPath.length === 0 ||
      ch.targetPathIndex >= ch.targetPath.length - 1 ||
      dist(currentTarget, target) > minTargetDelta;
    if (!shouldRepath) return;
    const rawPath = this.nav.findPath(ch.pos, target);
    const path = [...rawPath];

    // Drop trivial first node equal to current position to avoid repath-reset loops.
    while (path.length > 1 && dist(path[0], ch.pos) < 0.0095) {
      path.shift();
    }

    // Safety fallback: if nav collapses to a single node while target is still far,
    // force a direct step so champions don't freeze due grid snapping anomalies.
    if (path.length <= 1 && dist(ch.pos, target) > 0.012) {
      ch.targetPath = [target];
      ch.targetPathIndex = 0;
      return;
    }

    ch.targetPath = path;
    ch.targetPathIndex = 0;
  }

  private antiDiveFallbackAnchor(ch: ChampionState) {
    const lanePath = this.lanePath(ch.team, ch.lane);
    const anchorIdx = this.chooseLaneAnchorIndex(ch, lanePath);
    const safetyOffset = ch.role === "ADC" || ch.role === "SUP" ? 1 : 0;
    const idx = clamp(anchorIdx - safetyOffset, 1, lanePath.length - 2);
    return lanePath[idx];
  }

  private updateStuckRecovery(ch: ChampionState, now: number) {
    const finalTarget = ch.targetPath[ch.targetPath.length - 1];
    const hasActivePath = Boolean(finalTarget) && ch.targetPathIndex < ch.targetPath.length;
    if (!hasActivePath || !finalTarget) {
      this.championStuckState.delete(ch.id);
      return;
    }

    const remaining = dist(ch.pos, finalTarget);
    if (remaining <= 0.02) {
      this.championStuckState.delete(ch.id);
      return;
    }

    const previous = this.championStuckState.get(ch.id);
    if (!previous) {
      this.championStuckState.set(ch.id, { sampleAt: now, samplePos: { ...ch.pos }, sampleRemaining: remaining, cooldownUntil: 0 });
      return;
    }

    const elapsed = now - previous.sampleAt;
    if (elapsed < STUCK_PROGRESS_WINDOW_SEC) return;

    const progress = dist(previous.samplePos, ch.pos);
    const remainingDelta = previous.sampleRemaining - remaining;
    if (progress < STUCK_MIN_PROGRESS && remainingDelta < STUCK_MIN_REMAINING_DELTA && now >= previous.cooldownUntil) {
      this.setChampionPath(ch, finalTarget, 0, true);
      this.championStuckState.set(ch.id, {
        sampleAt: now,
        samplePos: { ...ch.pos },
        sampleRemaining: remaining,
        cooldownUntil: now + STUCK_REPATH_COOLDOWN_SEC,
      });
      return;
    }

    this.championStuckState.set(ch.id, {
      sampleAt: now,
      samplePos: { ...ch.pos },
      sampleRemaining: remaining,
      cooldownUntil: previous.cooldownUntil,
    });
  }

  private spawnFormationPosition(path: Vec2[], kind: MinionState["kind"], slot: number): Vec2 {
    const origin = path[0];
    const next = path[1] ?? path[0];
    const direction = normalize({ x: next.x - origin.x, y: next.y - origin.y });
    const perpendicular = { x: -direction.y, y: direction.x };
    const row = kind === "melee" ? 0 : 1;
    const column = slot - 1;
    const depth = row * 0.0105 + Math.abs(column) * 0.002;
    const lateral = column * 0.0048;
    return {
      x: clamp(origin.x - direction.x * depth + perpendicular.x * lateral, 0.01, 0.99),
      y: clamp(origin.y - direction.y * depth + perpendicular.y * lateral, 0.01, 0.99),
    };
  }

  private styleAggro(team: TeamId) {
    const style = team === "blue" ? this.snapshot.home_team.play_style : this.snapshot.away_team.play_style;
    switch (style) {
      case "HighPress": return 1.12;
      case "Attacking": return 1.08;
      case "Possession": return 1.04;
      case "Defensive": return 0.9;
      case "Counter": return 0.96;
      default: return 1.0;
    }
  }

  private chooseLaneAnchorIndex(ch: ChampionState, lanePath: Vec2[]) {
    const alliedLaneMinions = this.state.minions
      .filter((m) => m.alive && m.team === ch.team && m.lane === ch.lane)
      .sort((a, b) => b.pathIndex - a.pathIndex);

    // Follow own wave when available, keeping a small safety offset behind.
    if (alliedLaneMinions.length) {
      const front = alliedLaneMinions[0];
      const offset = ch.role === "TOP" || ch.role === "MID" ? 1 : 2;
      return clamp(front.pathIndex - offset, 1, lanePath.length - 1);
    }

    // If no allied wave is present, move toward enemy lane units to force lane confrontation.
    const enemyLaneUnits = this.state.minions
      .filter((m) => m.alive && m.team !== ch.team && m.lane === ch.lane)
      .sort((a, b) => dist(ch.pos, a.pos) - dist(ch.pos, b.pos));
    if (enemyLaneUnits.length) {
      const enemyIndex = this.closestLanePathIndex(enemyLaneUnits[0].pos, lanePath);
      const offset = ch.role === "TOP" || ch.role === "MID" ? 1 : 2;
      return clamp(enemyIndex - offset, 1, lanePath.length - 1);
    }

    // If lane is temporarily empty, keep advancing dynamically to avoid static anchors.
    const currentIndex = this.closestLanePathIndex(ch.pos, lanePath);
    const laneState = this.styleAggro(ch.team);
    const step = laneState > 1.04 ? 2 : 1;
    return clamp(currentIndex + step, 1, lanePath.length - 1);
  }

  private closestLanePathIndex(pos: Vec2, lanePath: Vec2[]) {
    let bestIdx = 1;
    let bestDist = Number.POSITIVE_INFINITY;
    for (let i = 1; i < lanePath.length; i += 1) {
      const d = dist(pos, lanePath[i]);
      if (d < bestDist) {
        bestDist = d;
        bestIdx = i;
      }
    }
    return bestIdx;
  }

  private waveIntervalSec(atTimeSec: number) {
    if (atTimeSec < 14 * 60) return 30;
    if (atTimeSec < 30 * 60) return 25;
    return 20;
  }

  private buildNeutralTimersState() {
    const entities = Object.fromEntries(
      (Object.keys(NEUTRAL_TIMER_TEMPLATE) as NeutralTimerKey[]).map((key) => {
        const template = NEUTRAL_TIMER_TEMPLATE[key];
        return [
          key,
          {
            ...template,
            alive: false,
            hp: template.maxHp,
            nextSpawnAt: template.unlocked ? template.firstSpawnAt : null,
            lastSpawnAt: null,
            lastTakenAt: null,
            timesSpawned: 0,
            timesTaken: 0,
          },
        ];
      }),
    ) as MatchState["neutralTimers"]["entities"];
    return {
      dragonSoulUnlocked: false,
      elderUnlocked: false,
      entities,
    } as MatchState["neutralTimers"];
  }

  private markNeutralTaken(key: NeutralTimerKey, killer?: ChampionState) {
    const timer = this.state.neutralTimers.entities[key];
    if (!timer || !timer.alive) return;
    timer.alive = false;
    timer.hp = 0;
    timer.lastTakenAt = this.state.timeSec;
    timer.timesTaken += 1;
    timer.nextSpawnAt = timer.oneShot || timer.respawnDelaySec == null ? null : this.state.timeSec + timer.respawnDelaySec;

    if (killer && isJungleCampKey(key)) {
      const reward = JUNGLE_CAMP_PROFILE[key];
      if (reward) {
        this.addGold(killer, reward.gold);
        this.addXp(killer, reward.xp);
      }
      this.log(`${killer.name} cleared ${timer.label}`, "info");
    }
  }

  private spawnNeutralTimerIfDue(key: NeutralTimerKey, now: number) {
    const timer = this.state.neutralTimers.entities[key];
    if (!timer || timer.alive || timer.nextSpawnAt == null || now < timer.nextSpawnAt || !timer.unlocked) return;
    timer.alive = true;
    timer.hp = timer.maxHp;
    timer.lastSpawnAt = timer.nextSpawnAt;
    timer.timesSpawned += 1;
    this.log(`${timer.label} spawned`, "spawn");
  }

  private pickJungleFarmPos(ch: ChampionState, now: number): Vec2 | null {
    const route = JUNGLE_ROUTE_BY_TIMER[ch.team];
    const currentFocus = this.jungleCampFocusByChampion.get(ch.id);
    if (currentFocus) {
      const currentTimer = this.state.neutralTimers.entities[currentFocus];
      if (currentTimer?.alive) return currentTimer.pos;
      this.jungleCampFocusByChampion.delete(ch.id);
      const prevIdx = route.findIndex((key) => key === currentFocus);
      if (prevIdx >= 0) {
        this.jungleRouteIndexByChampion.set(ch.id, (prevIdx + 1) % route.length);
      }
    }

    const baseIdx = this.jungleRouteIndexByChampion.get(ch.id) ?? 0;
    for (let i = 0; i < route.length; i += 1) {
      const idx = (baseIdx + i) % route.length;
      const key = route[idx];
      const timer = this.state.neutralTimers.entities[key];
      if (timer?.alive) {
        this.jungleCampFocusByChampion.set(ch.id, key);
        this.jungleRouteIndexByChampion.set(ch.id, idx);
        return timer.pos;
      }
    }

    // If all camps are down, path to next soonest spawn on route.
    const pending = route
      .map((key) => this.state.neutralTimers.entities[key])
      .filter((timer): timer is MatchState["neutralTimers"]["entities"][NeutralTimerKey] => Boolean(timer && timer.nextSpawnAt != null))
      .sort((a, b) => (a.nextSpawnAt ?? Number.POSITIVE_INFINITY) - (b.nextSpawnAt ?? Number.POSITIVE_INFINITY));
    const next = pending[0];
    if (!next) return null;
    this.jungleCampFocusByChampion.set(ch.id, next.key);
    const idx = route.findIndex((key) => key === next.key);
    if (idx >= 0) this.jungleRouteIndexByChampion.set(ch.id, idx);
    void now;
    return next.pos;
  }

  private nearbyNeutralCamp(ch: ChampionState) {
    if (ch.role !== "JGL") return null;
    const camps = (Object.values(this.state.neutralTimers.entities) as NeutralTimerState[])
      .filter((timer) => isJungleCampKey(timer.key) && timer.alive && dist(timer.pos, ch.pos) <= JUNGLE_CAMP_ENGAGE_RADIUS)
      .sort((a, b) => dist(a.pos, ch.pos) - dist(b.pos, ch.pos));
    return camps[0] ?? null;
  }

  private pickMacroObjectivePos(ch: ChampionState, now: number): Vec2 | null {
    if (ch.role !== "JGL") return null;
    const timers = this.state.neutralTimers.entities;
    const objectiveLeadTime = 35;
    const candidates: NeutralTimerKey[] = ["elder", "baron", "herald", "voidgrubs", "dragon", "scuttle-top", "scuttle-bot"];

    for (const key of candidates) {
      const timer = timers[key];
      if (!timer || !timer.unlocked) continue;
      if (timer.alive) return timer.pos;
      if (timer.nextSpawnAt != null && timer.nextSpawnAt >= now && timer.nextSpawnAt - now <= objectiveLeadTime) return timer.pos;
    }
    return null;
  }

  private spawnWave() {
    const makeMinion = (team: TeamId, lane: LaneId, kind: MinionState["kind"], slot: number): MinionState => {
      const path = this.lanePath(team, lane);
      const profile = MINION_PROFILE[kind];
      return {
        id: `m-${this.nextMinionId++}`,
        team,
        lane,
        pos: this.spawnFormationPosition(path, kind, slot),
        hp: profile.maxHp,
        maxHp: profile.maxHp,
        alive: true,
        kind,
        lastHitByChampionId: null,
        attackCdUntil: 0,
        moveSpeed: profile.moveSpeed,
        attackRange: profile.attackRange,
        attackDamage: profile.attackDamage,
        path,
        pathIndex: 1,
      };
    };
    for (const lane of ["top", "mid", "bot"] as LaneId[]) {
      for (let i = 0; i < 3; i += 1) {
        this.state.minions.push(makeMinion("blue", lane, "melee", i));
        this.state.minions.push(makeMinion("red", lane, "melee", i));
      }
      for (let i = 0; i < 3; i += 1) {
        this.state.minions.push(makeMinion("blue", lane, "ranged", i));
        this.state.minions.push(makeMinion("red", lane, "ranged", i));
      }
    }
    this.log("Minion wave spawned", "spawn");
  }

  private moveEntity(pos: Vec2, target: Vec2, speed: number, dt: number) {
    const dd = dist(pos, target);
    if (dd < 1e-5) return;
    const step = Math.min(speed * dt, dd);
    pos.x += ((target.x - pos.x) / dd) * step;
    pos.y += ((target.y - pos.y) / dd) * step;
  }

  private decideChampion(ch: ChampionState) {
    if (!ch.alive) return;
    if (ch.hp / ch.maxHp < 0.28) {
      ch.state = "recall";
    }

    if (ch.state === "recall") {
      const base = BASE_POSITION[ch.team];
      this.setChampionPath(ch, base, 0.01);
      return;
    }

    const now = this.state.timeSec;
    const macroObjectivePos = this.pickMacroObjectivePos(ch, now);
    if (macroObjectivePos) {
      ch.state = "objective";
      this.setChampionPath(ch, macroObjectivePos, 0.012);
      return;
    }

    ch.state = "lane";
    if (ch.role === "JGL") {
      const jungleTarget = this.pickJungleFarmPos(ch, now);
      if (jungleTarget) {
        this.setChampionPath(ch, jungleTarget, 0.014);
        return;
      }

      const route = JUNGLE_ROUTE[ch.team];
      const offset = ch.team === "blue" ? 0 : 2;
      const target = route[(Math.floor(this.state.timeSec / 12) + offset) % route.length];
      this.setChampionPath(ch, target, 0.016);
      return;
    }

    const lanePath = this.lanePath(ch.team, ch.lane);
    const idx = this.chooseLaneAnchorIndex(ch, lanePath);
    this.setChampionPath(ch, lanePath[idx], 0.014);
  }

  private tickChampions(dt: number) {
    const now = this.state.timeSec;
    for (const ch of this.state.champions) {
      if (!ch.alive) {
        this.championStuckState.delete(ch.id);
        if (now >= ch.respawnAt) {
          ch.alive = true;
          ch.hp = ch.maxHp;
          ch.pos = { ...BASE_POSITION[ch.team] };
          ch.state = "lane";
        }
        continue;
      }

      if (now >= ch.nextDecisionAt) {
        this.decideChampion(ch);
        ch.nextDecisionAt = now + 0.8;
      }

      const node = ch.targetPath[ch.targetPathIndex];
      if (node) {
        this.moveEntity(ch.pos, node, ch.moveSpeed, dt);
        if (dist(ch.pos, node) < 0.01 && ch.targetPathIndex < ch.targetPath.length - 1) {
          ch.targetPathIndex += 1;
        }
      }
      this.updateStuckRecovery(ch, now);

      if (ch.state === "recall" && dist(ch.pos, BASE_POSITION[ch.team]) < 0.03) {
        ch.hp = ch.maxHp;
        ch.state = "lane";
        this.log(`${ch.name} recalled`, "recall");
      }

      const target = this.pickCombatTarget(ch, now);
      if (!target) continue;
      if (!this.isLocalCombatTarget(ch, target)) continue;

      if (this.shouldAvoidDive(ch, target.target.pos)) {
        ch.state = "lane";
        this.setChampionPath(ch, this.antiDiveFallbackAnchor(ch), 0.008, true);
        continue;
      }

      if (dist(ch.pos, target.target.pos) > ch.attackRange) {
        this.setChampionPath(ch, target.target.pos, 0.01);
        continue;
      }

      if (now < ch.attackCdUntil) continue;

      if (target.kind === "champion") {
        target.target.hp -= ch.attackDamage;
        target.target.lastDamagedByChampionId = ch.id;
        target.target.lastDamagedAt = now;
        ch.attackCdUntil = now + 0.85;

        if (target.target.hp <= 0 && target.target.alive) {
          target.target.alive = false;
          target.target.deaths += 1;
          target.target.respawnAt = now + 12;
          ch.kills += 1;
          this.state.stats[ch.team].kills += 1;
          this.addGold(ch, CHAMPION_KILL_GOLD);
          this.addXp(ch, CHAMPION_KILL_XP);

          const assisters = this.state.champions.filter(
            (ally) => ally.alive && ally.team === ch.team && ally.id !== ch.id && dist(ally.pos, target.target.pos) <= ASSIST_RADIUS,
          );
          if (assisters.length) {
            const sharedGold = CHAMPION_ASSIST_GOLD_TOTAL / assisters.length;
            const sharedXp = CHAMPION_KILL_XP * 0.5 / assisters.length;
            assisters.forEach((assist) => {
              assist.assists += 1;
              this.addGold(assist, sharedGold);
              this.addXp(assist, sharedXp);
            });
          }

          const defenders = this.state.champions.filter(
            (enemyChampion) => enemyChampion.alive && enemyChampion.team === target.target.team && dist(enemyChampion.pos, target.target.pos) <= ASSIST_RADIUS,
          );
          if (defenders.length) {
            const consolationXp = CHAMPION_KILL_XP * 0.15 / defenders.length;
            defenders.forEach((defender) => this.addXp(defender, consolationXp));
          }

          this.log(`${ch.name} killed ${target.target.name}`, "kill");
        }
        continue;
      }

      if (target.kind === "minion") {
        target.target.hp -= ch.attackDamage;
        ch.attackCdUntil = now + 0.75;
        if (target.target.hp <= 0) this.registerMinionDeath(target.target, ch.id);
        continue;
      }

      if (target.kind === "neutral") {
        target.target.hp -= ch.attackDamage * 1.08;
        ch.attackCdUntil = now + 0.78;
        if (target.target.hp <= 0) {
          this.markNeutralTaken(target.target.key, ch);
        }
        continue;
      }

      target.target.hp -= ch.attackDamage;
      ch.attackCdUntil = now + 0.9;
      if (target.target.hp <= 0 && target.target.alive) {
        target.target.alive = false;
        if (target.target.kind === "tower") this.state.stats[ch.team].towers += 1;
        this.log(`${ch.name} destroyed ${target.target.id}`, target.target.kind === "nexus" ? "nexus" : "tower");
        if (target.target.kind === "nexus") {
          this.state.winner = ch.team;
          this.state.running = false;
        }
      }
    }
  }

  private nearestEnemyMinion(m: MinionState, range: number) {
    const candidates = this.state.minions.filter((e) => e.alive && e.team !== m.team && e.lane === m.lane && dist(m.pos, e.pos) < range);
    if (!candidates.length) return null;
    if (m.kind === "ranged") {
      return candidates.sort((a, b) => a.hp - b.hp || dist(m.pos, a.pos) - dist(m.pos, b.pos))[0];
    }
    return candidates.sort((a, b) => dist(m.pos, a.pos) - dist(m.pos, b.pos))[0];
  }

  private tickMinions(dt: number) {
    const now = this.state.timeSec;
    for (const m of this.state.minions) {
      if (!m.alive) continue;
      const enemyMinion = this.nearestEnemyMinion(m, 0.05);
      if (enemyMinion) {
        if (now >= m.attackCdUntil) {
          enemyMinion.hp -= m.attackDamage;
          m.attackCdUntil = now + MINION_PROFILE[m.kind].attackCadence;
          if (enemyMinion.hp <= 0) this.registerMinionDeath(enemyMinion, null);
        }
        continue;
      }

      const targetStruct = this.state.structures.find(
        (s) => s.alive && s.team !== m.team && (s.lane === m.lane || s.kind === "nexus"),
      );
      if (targetStruct && dist(m.pos, targetStruct.pos) <= 0.05) {
        if (now >= m.attackCdUntil) {
          targetStruct.hp -= m.attackDamage;
          m.attackCdUntil = now + MINION_PROFILE[m.kind].attackCadence;
          if (targetStruct.hp <= 0 && targetStruct.alive) {
            targetStruct.alive = false;
            if (targetStruct.kind === "tower") this.state.stats[m.team].towers += 1;
            this.log(`${m.team.toUpperCase()} destroyed ${targetStruct.id}`, targetStruct.kind === "nexus" ? "nexus" : "tower");
            if (targetStruct.kind === "nexus") {
              this.state.winner = m.team;
              this.state.running = false;
            }
          }
        }
        continue;
      }

      const next = m.path[m.pathIndex] ?? m.path[m.path.length - 1];
      this.moveEntity(m.pos, next, m.moveSpeed, dt);
      if (dist(m.pos, next) < 0.01 && m.pathIndex < m.path.length - 1) m.pathIndex += 1;
    }

    this.state.minions = this.state.minions.filter((m) => m.alive && m.pathIndex < m.path.length);
  }

  private tickStructures(dt: number) {
    const now = this.state.timeSec;
    for (const s of this.state.structures) {
      if (!s.alive || s.kind === "nexus") continue;
      if (now < s.attackCdUntil) continue;

      const minion = this.state.minions.find((m) => m.alive && m.team !== s.team && dist(m.pos, s.pos) < 0.08);
      if (minion) {
        minion.hp -= 24;
        s.attackCdUntil = now + 1.0;
        if (minion.hp <= 0) this.registerMinionDeath(minion, null);
        continue;
      }

      const champ = this.state.champions.find((c) => c.alive && c.team !== s.team && dist(c.pos, s.pos) < 0.08);
      if (champ) {
        champ.hp -= 32;
        if (champ.hp <= 0 && champ.alive) {
          champ.alive = false;
          champ.deaths += 1;
          champ.respawnAt = now + 12;
        }
        s.attackCdUntil = now + 1.0;
      }
    }
    void dt;
  }

  private tickNeutralTimers() {
    const now = this.state.timeSec;
    const timers = this.state.neutralTimers.entities;

    if (this.state.neutralTimers.elderUnlocked && !timers.elder.unlocked) {
      timers.elder.unlocked = true;
      timers.elder.nextSpawnAt = now + 6 * 60;
    }

    for (const key of Object.keys(timers) as NeutralTimerKey[]) {
      this.spawnNeutralTimerIfDue(key, now);
      const timer = timers[key];
      if (!timer.alive || timer.combatGraceUntil == null) continue;
      if (now >= timer.combatGraceUntil) {
        timer.alive = false;
        timer.hp = 0;
        timer.nextSpawnAt = null;
        this.log(`${timer.label} despawned`, "info");
      }
    }

    this.state.objectives.dragon.alive = timers.dragon.alive;
    this.state.objectives.dragon.nextSpawnAt = timers.dragon.nextSpawnAt ?? Number.POSITIVE_INFINITY;
    this.state.objectives.baron.alive = timers.baron.alive;
    this.state.objectives.baron.nextSpawnAt = timers.baron.nextSpawnAt ?? Number.POSITIVE_INFINITY;
  }

  private tickObjectives() {
    const timers = this.state.neutralTimers.entities;
    for (const key of ["dragon", "baron"] as const) {
      const timer = timers[key];
      if (!timer.alive) continue;

      const blue = this.state.champions.filter((c) => c.alive && c.team === "blue" && dist(c.pos, timer.pos) < 0.08).length;
      const red = this.state.champions.filter((c) => c.alive && c.team === "red" && dist(c.pos, timer.pos) < 0.08).length;
      if (blue >= 3 && blue >= red + 1) {
        this.markNeutralTaken(key);
        this.state.stats.blue[key === "dragon" ? "dragons" : "barons"] += 1;
        this.log(`BLUE secured ${key}`, key);
      } else if (red >= 3 && red >= blue + 1) {
        this.markNeutralTaken(key);
        this.state.stats.red[key === "dragon" ? "dragons" : "barons"] += 1;
        this.log(`RED secured ${key}`, key);
      }
    }

    if (!this.state.neutralTimers.dragonSoulUnlocked) {
      const blueSoul = this.state.stats.blue.dragons >= 4;
      const redSoul = this.state.stats.red.dragons >= 4;
      if (blueSoul || redSoul) {
        this.state.neutralTimers.dragonSoulUnlocked = true;
        this.state.neutralTimers.elderUnlocked = true;
        this.markNeutralTaken("dragon");
        timers.dragon.nextSpawnAt = null;
        this.log(`${blueSoul ? "BLUE" : "RED"} unlocked dragon soul`, "dragon");
      }
    }
  }

  tick(dtRaw: number, speed: number) {
    if (!this.state.running) return;
    const dt = clamp(dtRaw, 0, 0.05) * speed;
    this.state.timeSec += dt;

    while (this.state.timeSec >= this.waveSpawnAt) {
      this.spawnWave();
      this.waveSpawnAt += this.waveIntervalSec(this.waveSpawnAt);
    }

    this.tickChampions(dt);
    this.tickMinions(dt);
    this.tickStructures(dt);
    this.tickNeutralTimers();
    this.tickObjectives();
  }
}
