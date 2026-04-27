import playersSeed from "../../data/lec/draft/players.json";
import championsSeed from "../../data/lec/draft/champions.json";
import type { PlayerData } from "../store/gameStore";
import { canonicalPosition } from "./playerRating";

export type LolRoleTag = "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT";

interface PlayerSeedEntry {
  ign: string;
  role?: string;
}

function normalizeKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z]/g, "");
}

const ROLE_BY_IGN = new Map(
  [
    ...(((playersSeed as { data?: { rostered_seeds?: PlayerSeedEntry[] } }).data?.rostered_seeds ?? []) as PlayerSeedEntry[]),
    ...(((playersSeed as { data?: { free_agent_seeds?: PlayerSeedEntry[] } }).data?.free_agent_seeds ?? []) as PlayerSeedEntry[]),
  ].map((entry) => [normalizeKey(entry.ign), normalizeKey(entry.role ?? "")]),
);

const ROLE_TO_CANONICAL: Record<string, LolRoleTag> = {
  top: "TOP",
  toplaner: "TOP",
  jungle: "JUNGLE",
  jungler: "JUNGLE",
  mid: "MID",
  middle: "MID",
  midlaner: "MID",
  adc: "ADC",
  bot: "ADC",
  bottom: "ADC",
  support: "SUPPORT",
  sup: "SUPPORT",
};

function mapPositionToRole(position: string): LolRoleTag {
  const direct = normalizeKey(position);
  if (direct === "top") return "TOP";
  if (direct === "jungle") return "JUNGLE";
  if (direct === "mid") return "MID";
  if (direct === "adc" || direct === "bot" || direct === "bottom") return "ADC";
  if (direct === "support" || direct === "sup") return "SUPPORT";

  const normalized = canonicalPosition(position || "");
  if (
    normalized === "Defender" ||
    normalized === "RightBack" ||
    normalized === "LeftBack" ||
    normalized === "CenterBack" ||
    normalized === "RightWingBack" ||
    normalized === "LeftWingBack"
  ) {
    return "TOP";
  }
  if (
    normalized === "AttackingMidfielder" ||
    normalized === "RightMidfielder" ||
    normalized === "LeftMidfielder"
  ) {
    return "MID";
  }
  if (
    normalized === "Forward" ||
    normalized === "Striker" ||
    normalized === "RightWinger" ||
    normalized === "LeftWinger"
  ) {
    return "ADC";
  }
  if (normalized === "DefensiveMidfielder" || normalized === "Goalkeeper") {
    return "SUPPORT";
  }
  return "JUNGLE";
}

export function resolvePlayerLolRole(player: PlayerData): LolRoleTag {
  const hasPositionData = Boolean((player.natural_position || player.position || "").trim());
  if (!hasPositionData) {
    const fromSeed = ROLE_TO_CANONICAL[ROLE_BY_IGN.get(normalizeKey(player.match_name || "")) ?? ""];
    if (fromSeed) return fromSeed;
  }
  return mapPositionToRole(player.natural_position || player.position || "");
}

const CHAMPION_ROLE_MAP =
  ((championsSeed as { data?: { roles?: Record<string, string[]> } }).data?.roles ?? {}) as Record<string, string[]>;

const CHAMPION_POOL_BY_ROLE: Record<LolRoleTag, string[]> = {
  TOP: [],
  JUNGLE: [],
  MID: [],
  ADC: [],
  SUPPORT: [],
};

Object.entries(CHAMPION_ROLE_MAP).forEach(([champion, roles]) => {
  roles.forEach((role) => {
    const normalized = normalizeKey(role);
    if (normalized === "top") CHAMPION_POOL_BY_ROLE.TOP.push(champion);
    if (normalized === "jungle") CHAMPION_POOL_BY_ROLE.JUNGLE.push(champion);
    if (normalized === "mid") CHAMPION_POOL_BY_ROLE.MID.push(champion);
    if (normalized === "adc" || normalized === "bot" || normalized === "bottom") CHAMPION_POOL_BY_ROLE.ADC.push(champion);
    if (normalized === "support" || normalized === "sup") CHAMPION_POOL_BY_ROLE.SUPPORT.push(champion);
  });
});

function stableHash(value: string): number {
  return value
    .split("")
    .reduce((acc, ch) => (acc * 33 + ch.charCodeAt(0)) >>> 0, 5381);
}

export function fallbackChampionForRole(playerId: string, role: LolRoleTag): string | null {
  const pool = CHAMPION_POOL_BY_ROLE[role] ?? [];
  if (pool.length === 0) return null;
  const idx = stableHash(`${playerId}:${role}`) % pool.length;
  return pool[idx] ?? null;
}
