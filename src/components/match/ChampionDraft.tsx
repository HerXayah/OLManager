import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { MatchSnapshot } from "./types";
import type { GameStateData } from "../../store/gameStore";
import { getChampionTiming } from "../../lib/championTiming";
import { getLolStaffEffectsForTeam } from "../../lib/lolStaffEffects";
import { resolvePlayerPhoto } from "../../lib/playerPhotos";
import teamsSeed from "../../../data/lec/draft/teams.json";
import playersSeed from "../../../data/lec/draft/players.json";
import championsSeed from "../../../data/lec/draft/champions.json";
import aiConfigSeed from "../../../data/lec/draft/ai-config.json";

type Side = "blue" | "red";
type DraftActionType = "ban" | "pick";
type Role = "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT";

interface ChampionData {
  id: string;
  key: number;
  name: string;
  image: string;
  tags: string[];
  roleHints: Role[];
}

interface DraftAction {
  type: DraftActionType;
  side: Side;
  label: string;
}

interface DraftPick {
  role: Role;
  championId: string;
}

interface DraftSelection {
  championId: string;
}

interface DraftAdviceTip {
  sourceType: "coach" | "player";
  sourceName: string;
  sourceRole?: string;
  sourceImage: string;
  type: "ban" | "pick" | "warn";
  text: string;
  champion?: ChampionData;
}

interface DraftScoreBreakdown {
  mastery: number;
  synergy: number;
  counter: number;
  comfort: number;
  preparation: number;
  total: number;
}

export interface ChampionDraftResultPayload {
  blue: {
    picks: DraftPick[];
    bans: string[];
    score: DraftScoreBreakdown;
  };
  red: {
    picks: DraftPick[];
    bans: string[];
    score: DraftScoreBreakdown;
  };
  history: string[];
}

interface TeamSeed {
  id: string;
  name: string;
  shortName: string;
  logo?: string;
}

interface PlayerSeed {
  ign: string;
  teamId: string;
  rating?: number;
  role: string;
  photo?: string;
  champions: Array<Array<string | number>>;
}

type RivalMasteryKnowledgeSource = "insignia" | "scouting" | "staff";

interface RivalMasteryDisplayEntry {
  champion: ChampionData;
  mastery: number;
  playerName: string;
  playerRole: Role | null;
  source: RivalMasteryKnowledgeSource;
}

type RivalMasteryOption = Omit<RivalMasteryDisplayEntry, "source">;

export function selectRivalMasteryKnowledgeForPlayer(
  allKnownOptions: RivalMasteryOption[],
  usedChampionIds: Set<string>,
  selectedChampionIds: Set<string>,
  isScouted: boolean,
): {
  knownEntries: RivalMasteryDisplayEntry[];
  staffCandidates: RivalMasteryDisplayEntry[];
} {
  const sortedOptions = allKnownOptions.slice().sort((a, b) => b.mastery - a.mastery);
  const signature = sortedOptions[0];
  const signatureChampionId = signature?.champion.id ?? null;
  const knownEntries: RivalMasteryDisplayEntry[] = [];
  const localSelectedChampionIds = new Set(selectedChampionIds);

  if (
    signature &&
    !usedChampionIds.has(signature.champion.id) &&
    !localSelectedChampionIds.has(signature.champion.id)
  ) {
    knownEntries.push({ ...signature, source: "insignia" });
    localSelectedChampionIds.add(signature.champion.id);
  }

  const availableNonSignatureOptions = sortedOptions.filter((option) => {
    if (option.champion.id === signatureChampionId) return false;
    if (localSelectedChampionIds.has(option.champion.id)) return false;
    return true;
  });

  if (isScouted) {
    const scoutedExtra = availableNonSignatureOptions.find(
      (option) => !usedChampionIds.has(option.champion.id),
    );
    if (scoutedExtra) {
      knownEntries.push({ ...scoutedExtra, source: "scouting" });
      localSelectedChampionIds.add(scoutedExtra.champion.id);
    }
  }

  const staffCandidates = availableNonSignatureOptions
    .filter((option) => !localSelectedChampionIds.has(option.champion.id))
    .map((option) => ({ ...option, source: "staff" as const }));

  return { knownEntries, staffCandidates };
}

export function calculateStaffRevealBudget(metaDiscovery: number): number {
  const normalized = Math.max(0, Math.min(1, (metaDiscovery - 0.9) / 0.3));
  return Math.max(1, Math.min(5, 1 + Math.round(normalized * 4)));
}

export function selectStaffRevealEntries(
  candidates: RivalMasteryDisplayEntry[],
  budget: number,
  usedChampionIds: Set<string>,
): RivalMasteryDisplayEntry[] {
  return candidates
    .slice()
    .sort((a, b) => b.mastery - a.mastery)
    .slice(0, budget)
    .filter((candidate) => !usedChampionIds.has(candidate.champion.id));
}

interface ChampionsSeed {
  data?: {
    roles?: Record<string, string[]>;
    counterpicks?: Array<{ a: string; b: string; value: number }>;
  };
}

interface DraftAiConfigSeed {
  data?: {
    pick?: {
      masteryWeight?: number;
      metaWeight?: number;
      counterAdvantageWeight?: number;
      counterRiskWeight?: number;
    };
    ban?: {
      enemyMasteryWeight?: number;
      metaWeight?: number;
    };
    score?: {
      counterAdvantageWeight?: number;
      counterRiskWeight?: number;
    };
    timing?: {
      userTurnSeconds?: number;
      aiMinSeconds?: number;
      aiMaxSeconds?: number;
    };
  };
}

interface ChampionDraftProps {
  snapshot: MatchSnapshot;
  onComplete: (result: ChampionDraftResultPayload) => void;
  controlledSide?: Side;
  seriesLength?: 1 | 3 | 5;
  blueSeriesWins?: number;
  redSeriesWins?: number;
  lockedChampionIds?: string[];
  gameState?: GameStateData;
}


// This should be put in another place. Cruncky to edit and test. Should we get real mastery and meta scores? This will be removed anyway.
const META_CHAMPION_SCORES: Record<string, number> = {
  ahri: 18,
  ambessa: 20,
  ashe: 12,
  aurora: 17,
  azir: 18,
  camille: 16,
  corki: 14,
  elise: 12,
  ezreal: 13,
  gragas: 15,
  hwei: 17,
  jayce: 16,
  jhin: 13,
  jinx: 15,
  kalista: 14,
  kaisa: 16,
  ksante: 17,
  leblanc: 14,
  leesin: 12,
  lissandra: 13,
  lucian: 15,
  nautilus: 14,
  nidalee: 12,
  orianna: 15,
  poppy: 15,
  rell: 14,
  renekton: 14,
  rumble: 18,
  sejuani: 16,
  skarner: 17,
  smolder: 13,
  sylas: 17,
  taliyah: 14,
  varus: 16,
  vi: 14,
  xayah: 13,
  yone: 14,
};

const ROLE_ORDER: Role[] = ["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"];
const ASSISTANT_COACH_PLACEHOLDER = "/player-photos/103935359525547325.png";
const LEC_LOGO_URL = "/lec-logo.svg";
const EMPTY_LOCKED_CHAMPION_IDS: string[] = [];
const ROLE_ICON_URLS: Record<Role, string> = {
  TOP: "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-top.png",
  JUNGLE:
    "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-jungle.png",
  MID: "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-middle.png",
  ADC: "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-bottom.png",
  SUPPORT:
    "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-utility.png",
};

const DRAFT_SEQUENCE: DraftAction[] = [
  { type: "ban", side: "blue", label: "B1" },
  { type: "ban", side: "red", label: "R1" },
  { type: "ban", side: "blue", label: "B2" },
  { type: "ban", side: "red", label: "R2" },
  { type: "ban", side: "blue", label: "B3" },
  { type: "ban", side: "red", label: "R3" },
  { type: "pick", side: "blue", label: "B1" },
  { type: "pick", side: "red", label: "R1" },
  { type: "pick", side: "red", label: "R2" },
  { type: "pick", side: "blue", label: "B2" },
  { type: "pick", side: "blue", label: "B3" },
  { type: "pick", side: "red", label: "R3" },
  { type: "ban", side: "blue", label: "B4" },
  { type: "ban", side: "red", label: "R4" },
  { type: "ban", side: "blue", label: "B5" },
  { type: "ban", side: "red", label: "R5" },
  { type: "pick", side: "red", label: "R4" },
  { type: "pick", side: "blue", label: "B4" },
  { type: "pick", side: "blue", label: "B5" },
  { type: "pick", side: "red", label: "R5" },
];

const COACH_BAN_PHRASES: string[] = [
  "If we leave {{champion}} open, {{player}} punishes us: {{mastery}}% mastery.",
  "{{champion}} is pure comfort for {{player}} ({{mastery}}%). I would remove it in bans.",
  "Ban priority: {{champion}}. {{player}} plays it at a {{mastery}}% level.",
  "Do not give away {{champion}}; {{player}} already showed {{mastery}}% on that pick.",
  "Early ban on {{champion}}. That matchup is dangerous in {{player}}'s hands ({{mastery}}%).",
  "I am worried about {{champion}}: {{player}} looks very comfortable ({{mastery}}%).",
];

const PLAYER_COUNTER_PICK_PHRASES: string[] = [
  "Pick me {{champion}}. I win the matchup into {{enemy}}.",
  "Give me {{champion}}; I can break lane against {{enemy}}.",
  "With {{champion}} I get real advantage over {{enemy}}. Trust me.",
  "{{champion}} is the right answer into {{enemy}}. Leave it to me.",
  "If they go {{enemy}}, I want {{champion}}: I studied that lane.",
  "Comfort plus counter pick: {{champion}} into {{enemy}}. This is the moment.",
];

const PLAYER_SMART_COUNTER_PICK_PHRASES: string[] = [
  "Please pick {{champion}} for me; with my game read I can break this matchup into {{enemy}}.",
  "With {{champion}} I can outplay {{enemy}} from wave 1. Give me that pick.",
  "{{enemy}} gets exposed if you give me {{champion}}. I can snowball it.",
  "I have perfect timing to punish {{enemy}} with {{champion}}.",
  "Trust me: {{champion}} vs {{enemy}} is technical advantage for us.",
  "This matchup heavily favors me: {{champion}} into {{enemy}}. I want to play it.",
];

const PLAYER_COMFORT_PICK_PHRASES: string[] = [
  "I feel very solid on {{champion}} ({{mastery}}% mastery).",
  "If you give me {{champion}}, I can guarantee consistency ({{mastery}}%).",
  "{{champion}} is my most reliable pick right now ({{mastery}}%).",
  "I am comfortable carrying with {{champion}} ({{mastery}}% mastery).",
  "{{champion}} gives me early impact and control ({{mastery}}%).",
  "I want {{champion}}; I practiced it a lot ({{mastery}}%).",
];

const PLAYER_BAN_REQUEST_PHRASES: string[] = [
  "Can we ban {{threat}}? It is one of the few picks that shuts down my {{champion}}.",
  "If we remove {{threat}}, my {{champion}} becomes much freer.",
  "I need {{threat}} out; it hurts the plan too much with {{champion}}.",
  "Ban {{threat}} and I can play {{champion}} with much more pressure.",
  "{{threat}} is a hard counter to {{champion}}. Better remove it now.",
  "If you want me on {{champion}}, the best move is removing {{threat}}.",
];

function pickPhrase(
  pool: string[],
  seed: string,
  shift = 0,
): { index: number; template: string } {
  if (pool.length === 0) return { index: 0, template: "" };
  const index = (hashText(seed) + shift) % pool.length;
  return {
    index,
    template: pool[index] ?? pool[0] ?? "",
  };
}

function inferRoleHints(tags: string[]): Role[] {
  const set = new Set<Role>();
  if (tags.includes("Support")) set.add("SUPPORT");
  if (tags.includes("Marksman")) set.add("ADC");
  if (tags.includes("Mage")) set.add("MID");
  if (tags.includes("Assassin")) {
    set.add("MID");
    set.add("JUNGLE");
  }
  if (tags.includes("Tank") || tags.includes("Fighter")) {
    set.add("TOP");
    set.add("JUNGLE");
  }
  if (set.size === 0) set.add("MID");
  return [...set];
}

function splashUrl(championId: string): string {
  return `https://ddragon.leagueoflegends.com/cdn/img/champion/splash/${championId}_0.jpg`;
}

function loadingUrl(championId: string): string {
  return `https://ddragon.leagueoflegends.com/cdn/img/champion/loading/${championId}_0.jpg`;
}

const TEAM_BRAND_MAP: Record<string, { tricode: string; logo: string | null }> = {
  "g2 esports": { tricode: "G2", logo: "/team-logos/g2-esports.png" },
  fnatic: { tricode: "FNC", logo: "/team-logos/fnatic.png" },
  "team vitality": { tricode: "VIT", logo: "/team-logos/team-vitality.png" },
  vitality: { tricode: "VIT", logo: "/team-logos/team-vitality.png" },
  "team heretics": { tricode: "HRTS", logo: "/team-logos/team-heretics-lec.png" },
  "sk gaming": { tricode: "SK", logo: "/team-logos/sk-gaming.png" },
  "movistar koi": { tricode: "MKOI", logo: "/team-logos/mad-lions.png" },
  "mad lions koi": { tricode: "MKOI", logo: "/team-logos/mad-lions.png" },
  "team bds": { tricode: "SHFT", logo: "https://static.lolesports.com/teams/1765897071435_600px-Shifters_allmode.png" },
  giantx: { tricode: "GX", logo: "/team-logos/giantx-lec.png" },
  heretics: { tricode: "HRTS", logo: "/team-logos/team-heretics-lec.png" },
  shifters: { tricode: "SHFT", logo: "https://static.lolesports.com/teams/1765897071435_600px-Shifters_allmode.png" },
  "natus vincere": { tricode: "NAVI", logo: "/team-logos/natus-vincere.png" },
  "karmine corp": { tricode: "KC", logo: "/team-logos/karmine-corp.png" },
};

const TEAM_SEEDS: TeamSeed[] = ((teamsSeed as { data?: { teams?: TeamSeed[] } })
  .data?.teams ?? []) as TeamSeed[];
const PLAYER_SEEDS: PlayerSeed[] = [
  ...(((playersSeed as unknown as {
    data?: { rostered_seeds?: PlayerSeed[] };
  }).data?.rostered_seeds ?? []) as PlayerSeed[]),
  ...(((playersSeed as unknown as {
    data?: { free_agent_seeds?: PlayerSeed[] };
  }).data?.free_agent_seeds ?? []) as PlayerSeed[]),
];
const CHAMPIONS_SEED: ChampionsSeed = championsSeed as ChampionsSeed;
const AI_CONFIG_SEED: DraftAiConfigSeed = aiConfigSeed as DraftAiConfigSeed;

function numberOrDefault(value: number | undefined, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

const AI_WEIGHTS = {
  pick: {
    masteryWeight: numberOrDefault(AI_CONFIG_SEED.data?.pick?.masteryWeight, 1.15),
    metaWeight: numberOrDefault(AI_CONFIG_SEED.data?.pick?.metaWeight, 1.1),
    counterAdvantageWeight: numberOrDefault(AI_CONFIG_SEED.data?.pick?.counterAdvantageWeight, 4),
    counterRiskWeight: numberOrDefault(AI_CONFIG_SEED.data?.pick?.counterRiskWeight, 3),
  },
  ban: {
    enemyMasteryWeight: numberOrDefault(AI_CONFIG_SEED.data?.ban?.enemyMasteryWeight, 1.15),
    metaWeight: numberOrDefault(AI_CONFIG_SEED.data?.ban?.metaWeight, 0.9),
  },
  score: {
    counterAdvantageWeight: numberOrDefault(AI_CONFIG_SEED.data?.score?.counterAdvantageWeight, 2),
    counterRiskWeight: numberOrDefault(AI_CONFIG_SEED.data?.score?.counterRiskWeight, 2),
  },
};

const AI_TIMING = {
  userTurnMs: numberOrDefault(AI_CONFIG_SEED.data?.timing?.userTurnSeconds, 30) * 1000,
  aiMinMs: numberOrDefault(AI_CONFIG_SEED.data?.timing?.aiMinSeconds, 3) * 1000,
  aiMaxMs: numberOrDefault(AI_CONFIG_SEED.data?.timing?.aiMaxSeconds, 10) * 1000,
};

function normalizeKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]/g, "");
}

function mapSeedRoleToDraftRole(role: string): Role | null {
  const key = normalizeKey(role);
  if (key === "top") return "TOP";
  if (key === "jungle") return "JUNGLE";
  if (key === "mid" || key === "middle") return "MID";
  if (key === "bot" || key === "bottom" || key === "adc") return "ADC";
  if (key === "support" || key === "sup") return "SUPPORT";
  return null;
}

function tierToMetaScore(tier: string): number {
  const normalized = tier.toUpperCase();
  if (normalized === "S") return 20;
  if (normalized === "A") return 17;
  if (normalized === "B") return 14;
  if (normalized === "C") return 10;
  if (normalized === "D") return 7;
  return 12;
}

const CHAMPION_ROLE_HINTS = new Map<string, Role[]>();
Object.entries(CHAMPIONS_SEED.data?.roles ?? {}).forEach(([championName, roles]) => {
  const mappedRoles = (roles ?? [])
    .map((role) => mapSeedRoleToDraftRole(role))
    .filter((role): role is Role => role !== null);

  if (mappedRoles.length > 0) {
    const unique = Array.from(new Set(mappedRoles));
    CHAMPION_ROLE_HINTS.set(normalizeKey(championName), unique);
  }
});

const CHAMPION_COUNTER_VALUES = new Map<string, number>();
(CHAMPIONS_SEED.data?.counterpicks ?? []).forEach(({ a, b, value }) => {
  const left = normalizeKey(a);
  const right = normalizeKey(b);
  if (!left || !right) return;
  CHAMPION_COUNTER_VALUES.set(`${left}::${right}`, Number(value) || 0);
});

function inferRoleHintsFromSeed(championId: string, championName: string, tags: string[]): Role[] {
  const seedRolesById = CHAMPION_ROLE_HINTS.get(normalizeKey(championId));
  if (seedRolesById && seedRolesById.length > 0) return seedRolesById;

  const seedRolesByName = CHAMPION_ROLE_HINTS.get(normalizeKey(championName));
  if (seedRolesByName && seedRolesByName.length > 0) return seedRolesByName;

  return inferRoleHints(tags);
}

function hashText(value: string): number {
  let hash = 0;
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash * 31 + value.charCodeAt(i)) >>> 0;
  }
  return hash;
}

function planTempo(playStyle: string): "early" | "mid" | "late" {
  switch (playStyle) {
    case "Attacking":
    case "HighPress":
    case "Counter":
      return "early";
    case "Defensive":
      return "late";
    default:
      return "mid";
  }
}

function championTempo(championId: string): "early" | "mid" | "late" {
  const mod = hashText(championId) % 3;
  if (mod === 0) return "early";
  if (mod === 1) return "mid";
  return "late";
}

function hasSynergy(a: string, b: string): boolean {
  return hashText(`${a}++${b}`) % 7 === 0;
}

function counterValue(allyChampionId: string, enemyChampionId: string): number {
  const key = `${normalizeKey(allyChampionId)}::${normalizeKey(enemyChampionId)}`;
  return CHAMPION_COUNTER_VALUES.get(key) ?? 0;
}

function teamTriCode(name: string): string {
  const normalizedName = normalizeKey(name);
  const fromSeed = TEAM_SEEDS.find((team) => normalizeKey(team.name) === normalizedName);
  if (fromSeed?.shortName) return fromSeed.shortName.toUpperCase();

  const key = name.trim().toLowerCase();
  const known = TEAM_BRAND_MAP[key];
  if (known) return known.tricode;

  const cleaned = name.replace(/[^A-Za-z0-9\s]/g, " ").trim();
  if (!cleaned) return "TEAM";
  const words = cleaned.split(/\s+/).filter(Boolean);
  if (words.length >= 2) return words.map((word) => word[0]).join("").toUpperCase().slice(0, 4);
  return cleaned.slice(0, 4).toUpperCase();
}

function teamLogo(name: string): string | null {
  const key = name.trim().toLowerCase();
  const known = TEAM_BRAND_MAP[key]?.logo;
  if (known) return known;

  const normalizedName = normalizeKey(name);
  const fromSeed = TEAM_SEEDS.find((team) => normalizeKey(team.name) === normalizedName);
  if (fromSeed?.logo) {
    const logoFileName = fromSeed.logo.split("/").pop();
    const fallback = logoFileName ? `/team-logos/${logoFileName.toLowerCase()}` : null;
    return fallback;
  }

  return null;
}

function tricodeSizeClass(code: string): string {
  if (code.length >= 4) return "text-[26px]";
  if (code.length === 3) return "text-[30px]";
  return "text-[34px]";
}

function playerSeedPhotoUrl(photo?: string): string | null {
  if (!photo) return null;
  if (photo.startsWith("/images/")) return `/data/lec${photo}`;
  return photo;
}

export default function ChampionDraft({
  snapshot,
  onComplete,
  controlledSide = "blue",
  seriesLength = 1,
  blueSeriesWins = 0,
  redSeriesWins = 0,
  lockedChampionIds = EMPTY_LOCKED_CHAMPION_IDS,
  gameState,
}: ChampionDraftProps) {
  const { t } = useTranslation();
  const [champions, setChampions] = useState<ChampionData[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [roleFilter, setRoleFilter] = useState<"ALL" | Role>("ALL");
  const [masteryFilter, setMasteryFilter] = useState<number>(0);
  const [searchTerm, setSearchTerm] = useState("");
  const [stepIndex, setStepIndex] = useState(0);
  const [blueBans, setBlueBans] = useState<string[]>([]);
  const [redBans, setRedBans] = useState<string[]>([]);
  const [bluePicks, setBluePicks] = useState<DraftSelection[]>([]);
  const [redPicks, setRedPicks] = useState<DraftSelection[]>([]);
  const [blueRoleOrder, setBlueRoleOrder] = useState<number[] | null>(null);
  const [redRoleOrder, setRedRoleOrder] = useState<number[] | null>(null);
  const [pendingChampionId, setPendingChampionId] = useState<string | null>(null);
  const [swapSource, setSwapSource] = useState<{ side: Side; index: number } | null>(null);
  const [draftHistory, setDraftHistory] = useState<string[]>([]);
  const [turnDurationMs, setTurnDurationMs] = useState<number>(AI_TIMING.userTurnMs);
  const [turnStartedAt, setTurnStartedAt] = useState<number>(() => Date.now());
  const [turnRemainingMs, setTurnRemainingMs] = useState<number>(AI_TIMING.userTurnMs);
  const [showFinalRoleReassignFx, setShowFinalRoleReassignFx] = useState(false);
  const autoResolvedStepKeyRef = useRef<string | null>(null);
  const finalRoleReassignFxPlayedRef = useRef(false);

  const bluePlayerIds = snapshot.home_team.players.map((player) => player.id);
  const redPlayerIds = snapshot.away_team.players.map((player) => player.id);

  useEffect(() => {
    let cancelled = false;

    const loadChampions = async () => {
      setLoading(true);
      setError(null);
      try {
        const versionsResponse = await fetch(
          "https://ddragon.leagueoflegends.com/api/versions.json",
        );
        const versions = (await versionsResponse.json()) as string[];
        const version = versions[0];

        const championsResponse = await fetch(
          `https://ddragon.leagueoflegends.com/cdn/${version}/data/en_US/champion.json`,
        );
        const payload = (await championsResponse.json()) as {
          data: Record<
            string,
            { key: string; id: string; name: string; tags: string[]; image: { full: string } }
          >;
        };

        const list = Object.values(payload.data)
          .map((champion) => ({
            id: champion.id,
            key: Number(champion.key),
            name: champion.name,
            image: `https://ddragon.leagueoflegends.com/cdn/${version}/img/champion/${champion.image.full}`,
            tags: champion.tags,
            roleHints: inferRoleHintsFromSeed(champion.id, champion.name, champion.tags),
          }))
          .sort((a, b) => a.name.localeCompare(b.name));

        if (!cancelled) setChampions(list);
      } catch (err) {
        if (!cancelled) setError(String(err));
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    void loadChampions();
    return () => {
      cancelled = true;
    };
  }, []);

  const currentStep = DRAFT_SEQUENCE[stepIndex] ?? null;

  const usedChampionIds = useMemo(() => {
    const set = new Set<string>();
    lockedChampionIds.forEach((championId) => set.add(championId));
    [...blueBans, ...redBans].forEach((championId) => set.add(championId));
    [...bluePicks, ...redPicks].forEach((pick) => set.add(pick.championId));
    return set;
  }, [blueBans, bluePicks, lockedChampionIds, redBans, redPicks]);

  const championById = useMemo(() => {
    const map = new Map<string, ChampionData>();
    champions.forEach((champion) => map.set(champion.id, champion));
    return map;
  }, [champions]);

  const seriesLockedChampions = useMemo(() => {
    return lockedChampionIds
      .map((championId) => championById.get(championId) ?? null)
      .filter((champion): champion is ChampionData => champion !== null);
  }, [championById, lockedChampionIds]);

  const finished = stepIndex >= DRAFT_SEQUENCE.length;

  const handleSelectChampion = (champion: ChampionData, actor: "user" | "ai" = "user") => {
    if (!currentStep || finished) return;
    if (actor === "user" && currentStep.side !== controlledSide) return;

    if (currentStep.type === "ban") {
      if (currentStep.side === "blue") {
        setBlueBans((prev) => [...prev, champion.id]);
      } else {
        setRedBans((prev) => [...prev, champion.id]);
      }
      setDraftHistory((prev) => [...prev, champion.id]);
      setStepIndex((prev) => prev + 1);
      return;
    }

    if (currentStep.side === "blue") {
      setBluePicks((prev) => [...prev, { championId: champion.id }]);
      setBlueRoleOrder(null);
    } else {
      setRedPicks((prev) => [...prev, { championId: champion.id }]);
      setRedRoleOrder(null);
    }
    setDraftHistory((prev) => [...prev, champion.id]);
    setStepIndex((prev) => prev + 1);
    setPendingChampionId(null);
  };

  const handleChampionTileClick = (champion: ChampionData): void => {
    if (!currentStep || finished || !isUserTurn) return;
    if (usedChampionIds.has(champion.id)) return;
    setPendingChampionId((prev) => (prev === champion.id ? null : champion.id));
  };

  const handleConfirmPendingAction = (): void => {
    if (!pendingChampionId) return;
    const champion = championById.get(pendingChampionId);
    if (!champion) return;
    handleSelectChampion(champion, "user");
  };

  const isUserTurn = !!currentStep && currentStep.side === controlledSide && !finished;

  const totalSteps = DRAFT_SEQUENCE.length;
  const currentStepNumber = Math.min(stepIndex + 1, totalSteps);
  const actionLabel =
    currentStep?.type === "ban"
      ? t("match.draft.actions.ban")
      : t("match.draft.actions.pick");
  const sideLabel =
    currentStep?.side === "blue"
      ? t("match.draft.sides.blue")
      : t("match.draft.sides.red");

  const bluePlayers = useMemo(() => snapshot.home_team.players.slice(0, 5), [snapshot.home_team.players]);
  const redPlayers = useMemo(() => snapshot.away_team.players.slice(0, 5), [snapshot.away_team.players]);
  const bluePlayerLabels = useMemo(
    () => bluePlayers.map((player) => player.name.toUpperCase()),
    [bluePlayers],
  );
  const redPlayerLabels = useMemo(
    () => redPlayers.map((player) => player.name.toUpperCase()),
    [redPlayers],
  );

  const blueHeader = `${bluePlayerLabels[0] ?? "BLUE"} & ${bluePlayerLabels[1] ?? "STAFF"}`;
  const redHeader = `${redPlayerLabels[0] ?? "RED"} & ${redPlayerLabels[1] ?? "STAFF"}`;

  const roleMasteryForChampion = (side: Side, role: Role, championId: string): number => {
    const roleIndex = ROLE_ORDER.indexOf(role);
    if (roleIndex < 0) return 25;
    return resolvePlayerMastery(side, roleIndex, championId);
  };

  const roleAssignmentScore = (side: Side, role: Role, championId: string): number => {
    const champion = championById.get(championId);
    const roleBonus = champion?.roleHints.includes(role) ? 20 : 0;
    return roleMasteryForChampion(side, role, championId) + roleBonus;
  };

  const buildBestRoleOrder = (side: Side, selections: DraftSelection[]): number[] => {
    const availableIndices = selections.map((_, idx) => idx);
    if (availableIndices.length <= 1) return availableIndices;

    const roleSubset = ROLE_ORDER.slice(0, availableIndices.length);
    let bestOrder = [...availableIndices];
    let bestScore = Number.NEGATIVE_INFINITY;

    const permute = (arr: number[], l: number): void => {
      if (l === arr.length) {
        const score = arr.reduce((sum, idx, roleIdx) => {
          const role = roleSubset[roleIdx];
          if (!role) return sum;
          return sum + roleAssignmentScore(side, role, selections[idx].championId);
        }, 0);
        if (score > bestScore) {
          bestScore = score;
          bestOrder = [...arr];
        }
        return;
      }

      for (let i = l; i < arr.length; i += 1) {
        [arr[l], arr[i]] = [arr[i], arr[l]];
        permute(arr, l + 1);
        [arr[l], arr[i]] = [arr[i], arr[l]];
      }
    };

    permute([...availableIndices], 0);
    return bestOrder;
  };

  const buildOrderedPicks = (
    side: Side,
    selections: DraftSelection[],
    manualOrder: number[] | null,
  ): Array<DraftPick | null> => {
    const naturalOrder = selections.map((_, idx) => idx);
    const order = !finished
      ? naturalOrder
      : manualOrder && manualOrder.length === selections.length
        ? manualOrder
        : buildBestRoleOrder(side, selections);

    return ROLE_ORDER.map((role, roleIdx) => {
      const selectionIndex = order[roleIdx];
      if (selectionIndex === undefined) return null;
      const selection = selections[selectionIndex];
      if (!selection) return null;
      return { role, championId: selection.championId };
    });
  };

  const blueBanDisplay: Array<string | null> = [...blueBans, null, null, null, null].slice(0, 5);
  const redBanDisplay: Array<string | null> = [...redBans, null, null, null, null].slice(0, 5);

  const homeTeamSeed = useMemo(
    () => TEAM_SEEDS.find((team) => normalizeKey(team.name) === normalizeKey(snapshot.home_team.name)),
    [snapshot.home_team.name],
  );
  const awayTeamSeed = useMemo(
    () => TEAM_SEEDS.find((team) => normalizeKey(team.name) === normalizeKey(snapshot.away_team.name)),
    [snapshot.away_team.name],
  );
  const rivalTeamSeed = controlledSide === "blue" ? awayTeamSeed : homeTeamSeed;

  const championLookupByNormalizedName = useMemo(() => {
    const map = new Map<string, ChampionData>();
    champions.forEach((champion) => {
      map.set(normalizeKey(champion.name), champion);
      map.set(normalizeKey(champion.id), champion);
    });
    return map;
  }, [champions]);

  const playerMasteryMap = useMemo(() => {
    const map = new Map<string, Map<string, number>>();
    PLAYER_SEEDS.forEach((player) => {
      const key = `${player.teamId}:${normalizeKey(player.ign)}`;
      const championMap = new Map<string, number>();
      player.champions.forEach((entry) => {
        const championName = String(entry[0] ?? "");
        const mastery = Number(entry[1] ?? 0);
        if (championName.length === 0) return;
        championMap.set(normalizeKey(championName), mastery);
      });
      map.set(key, championMap);
    });
    return map;
  }, []);

  const runtimeMasteryByPlayerId = useMemo(() => {
    const map = new Map<string, Map<string, number>>();
    (gameState?.champion_masteries ?? []).forEach((entry) => {
      let championMap = map.get(entry.player_id);
      if (!championMap) {
        championMap = new Map<string, number>();
        map.set(entry.player_id, championMap);
      }
      championMap.set(normalizeKey(entry.champion_id), Number(entry.mastery ?? 25));
    });
    return map;
  }, [gameState?.champion_masteries]);

  const runtimeMetaScoreByChampion = useMemo(() => {
    const map = new Map<string, number>();
    (gameState?.champion_patch?.hidden_meta ?? []).forEach((entry) => {
      map.set(normalizeKey(entry.champion_id), tierToMetaScore(String(entry.tier ?? "B")));
    });
    return map;
  }, [gameState?.champion_patch?.hidden_meta]);

  const resolvePlayerMastery = (
    side: Side,
    pickIndex: number,
    championId: string,
  ): number => {
    const playerId = side === "blue" ? bluePlayerIds[pickIndex] : redPlayerIds[pickIndex];
    if (playerId) {
      const runtimeMap = runtimeMasteryByPlayerId.get(playerId);
      if (runtimeMap) {
        return runtimeMap.get(normalizeKey(championId)) ?? 25;
      }
    }

    const teamId = side === "blue" ? homeTeamSeed?.id : awayTeamSeed?.id;
    const playerName = side === "blue" ? bluePlayers[pickIndex]?.name : redPlayers[pickIndex]?.name;
    if (!teamId || !playerName) return 25;

    const masteryByChampion = playerMasteryMap.get(`${teamId}:${normalizeKey(playerName)}`);
    if (!masteryByChampion) return 25;
    return masteryByChampion.get(normalizeKey(championId)) ?? 25;
  };

  const resolveTeamChampionMastery = (side: Side, championId: string): number => {
    const teamId = side === "blue" ? homeTeamSeed?.id : awayTeamSeed?.id;
    const teamPlayers = side === "blue" ? bluePlayers : redPlayers;
    if (!teamId || teamPlayers.length === 0) return 25;

    let best = 25;
    teamPlayers.forEach((player) => {
      const masteryByChampion = playerMasteryMap.get(`${teamId}:${normalizeKey(player.name)}`);
      const value = masteryByChampion?.get(normalizeKey(championId)) ?? 25;
      if (value > best) best = value;
    });
    return best;
  };

  const blueOrderedPicks = buildOrderedPicks("blue", bluePicks, finished ? blueRoleOrder : null);
  const redOrderedPicks = buildOrderedPicks("red", redPicks, finished ? redRoleOrder : null);

  const metaScoreForChampion = (champion: ChampionData): number => {
    const runtime = runtimeMetaScoreByChampion.get(normalizeKey(champion.id));
    if (typeof runtime === "number") return runtime;
    const direct = META_CHAMPION_SCORES[normalizeKey(champion.id)] ?? META_CHAMPION_SCORES[normalizeKey(champion.name)];
    return direct ?? 0;
  };

  const enemySideFor = (side: Side): Side => (side === "blue" ? "red" : "blue");

  const visibleChampions = useMemo(() => {
    return champions.filter((champion) => {
      if (roleFilter !== "ALL" && !champion.roleHints.includes(roleFilter)) return false;
      if (searchTerm.trim().length > 0) {
        const query = searchTerm.trim().toLowerCase();
        if (!champion.name.toLowerCase().includes(query)) return false;
      }

      const sampleSide = currentStep?.side ?? controlledSide;
      const samplePickIndex = sampleSide === "blue" ? bluePicks.length : redPicks.length;
      const mastery = resolvePlayerMastery(sampleSide, samplePickIndex, champion.id);
      if (mastery < masteryFilter) return false;

      return true;
    });
  }, [
    bluePicks.length,
    champions,
    controlledSide,
    currentStep,
    masteryFilter,
    redPicks.length,
    roleFilter,
    resolvePlayerMastery,
    searchTerm,
  ]);

  const getAvailableChampions = (): ChampionData[] =>
    champions.filter((champion) => !usedChampionIds.has(champion.id));

  const selectAiChampionForCurrentStep = (): ChampionData | null => {
    if (!currentStep) return null;

    const available = getAvailableChampions();
    if (available.length === 0) return null;

    if (currentStep.type === "pick") {
      const aiSide = currentStep.side;
      const ownPicks = aiSide === "blue" ? bluePicks : redPicks;
      const enemyPicks = aiSide === "blue" ? redPicks : bluePicks;
      const coveredRoles = new Set<Role>();
      ownPicks.forEach((pick) => {
        const champion = championById.get(pick.championId);
        champion?.roleHints.forEach((role) => coveredRoles.add(role));
      });
      const missingRoles = ROLE_ORDER.filter((role) => !coveredRoles.has(role));

      const candidates = available;

      if (candidates.length === 0) return available[0] ?? null;

      let bestChampion: ChampionData | null = null;
      let bestScore = Number.NEGATIVE_INFINITY;

      candidates.forEach((champion) => {
        const mastery = resolveTeamChampionMastery(aiSide, champion.id);
        const meta = metaScoreForChampion(champion);
        const roleNeedBonus =
          missingRoles.length > 0 && champion.roleHints.some((role) => missingRoles.includes(role)) ? 12 : 0;
        let counter = 0;

        enemyPicks.forEach((enemyPick) => {
          counter += counterValue(champion.id, enemyPick.championId) * AI_WEIGHTS.pick.counterAdvantageWeight;
          counter -= counterValue(enemyPick.championId, champion.id) * AI_WEIGHTS.pick.counterRiskWeight;
        });

        const score =
          mastery * AI_WEIGHTS.pick.masteryWeight +
          meta * AI_WEIGHTS.pick.metaWeight +
          counter +
          roleNeedBonus;
        if (score > bestScore) {
          bestScore = score;
          bestChampion = champion;
        }
      });

      return bestChampion;
    }

    const targetSide = enemySideFor(currentStep.side);
    let bestBan: ChampionData | null = null;
    let bestScore = Number.NEGATIVE_INFINITY;

    available.forEach((champion) => {
      const enemyMastery = resolveTeamChampionMastery(targetSide, champion.id);
      const meta = metaScoreForChampion(champion);
      const score =
        enemyMastery * AI_WEIGHTS.ban.enemyMasteryWeight +
        meta * AI_WEIGHTS.ban.metaWeight;
      if (score > bestScore) {
        bestScore = score;
        bestBan = champion;
      }
    });

    return bestBan;
  };

  const handleSkipDraft = useCallback((): void => {
    if (loading || champions.length === 0 || finished) return;

    const nextBlueBans = [...blueBans];
    const nextRedBans = [...redBans];
    const nextBluePicks = [...bluePicks];
    const nextRedPicks = [...redPicks];
    const nextHistory = [...draftHistory];

    const used = new Set<string>();
    lockedChampionIds.forEach((championId) => used.add(championId));
    [...nextBlueBans, ...nextRedBans].forEach((championId) => used.add(championId));
    [...nextBluePicks, ...nextRedPicks].forEach((pick) => used.add(pick.championId));

    const chooseBestForStep = (step: DraftAction): ChampionData | null => {
      const available = champions.filter((champion) => !used.has(champion.id));
      if (available.length === 0) return null;

      if (step.type === "pick") {
        const side = step.side;
        const ownPicks = side === "blue" ? nextBluePicks : nextRedPicks;
        const enemyPicks = side === "blue" ? nextRedPicks : nextBluePicks;
        const coveredRoles = new Set<Role>();
        ownPicks.forEach((pick) => {
          const champion = championById.get(pick.championId);
          champion?.roleHints.forEach((role) => coveredRoles.add(role));
        });
        const missingRoles = ROLE_ORDER.filter((role) => !coveredRoles.has(role));

        let bestChampion: ChampionData | null = null;
        let bestScore = Number.NEGATIVE_INFINITY;

        available.forEach((champion) => {
          const mastery = resolveTeamChampionMastery(side, champion.id);
          const meta = metaScoreForChampion(champion);
          const roleNeedBonus =
            missingRoles.length > 0 && champion.roleHints.some((role) => missingRoles.includes(role)) ? 12 : 0;
          let counter = 0;

          enemyPicks.forEach((enemyPick) => {
            counter += counterValue(champion.id, enemyPick.championId) * AI_WEIGHTS.pick.counterAdvantageWeight;
            counter -= counterValue(enemyPick.championId, champion.id) * AI_WEIGHTS.pick.counterRiskWeight;
          });

          const score =
            mastery * AI_WEIGHTS.pick.masteryWeight +
            meta * AI_WEIGHTS.pick.metaWeight +
            counter +
            roleNeedBonus;
          if (score > bestScore) {
            bestScore = score;
            bestChampion = champion;
          }
        });

        return bestChampion;
      }

      const targetSide = enemySideFor(step.side);
      let bestBan: ChampionData | null = null;
      let bestScore = Number.NEGATIVE_INFINITY;

      available.forEach((champion) => {
        const enemyMastery = resolveTeamChampionMastery(targetSide, champion.id);
        const meta = metaScoreForChampion(champion);
        const score = enemyMastery * AI_WEIGHTS.ban.enemyMasteryWeight + meta * AI_WEIGHTS.ban.metaWeight;
        if (score > bestScore) {
          bestScore = score;
          bestBan = champion;
        }
      });

      return bestBan;
    };

    for (let i = stepIndex; i < DRAFT_SEQUENCE.length; i += 1) {
      const step = DRAFT_SEQUENCE[i];
      const selected = chooseBestForStep(step);
      if (!selected) break;

      used.add(selected.id);
      nextHistory.push(selected.id);

      if (step.type === "ban") {
        if (step.side === "blue") nextBlueBans.push(selected.id);
        else nextRedBans.push(selected.id);
      } else if (step.side === "blue") {
        nextBluePicks.push({ championId: selected.id });
      } else {
        nextRedPicks.push({ championId: selected.id });
      }
    }

    setBlueBans(nextBlueBans);
    setRedBans(nextRedBans);
    setBluePicks(nextBluePicks);
    setRedPicks(nextRedPicks);
    setBlueRoleOrder(null);
    setRedRoleOrder(null);
    setDraftHistory(nextHistory);
    setPendingChampionId(null);
    setSwapSource(null);
    setStepIndex(DRAFT_SEQUENCE.length);
  }, [
    blueBans,
    bluePicks,
    champions,
    draftHistory,
    finished,
    loading,
    lockedChampionIds,
    redBans,
    redPicks,
    stepIndex,
  ]);

  const currentStepKey = currentStep
    ? `${stepIndex}-${currentStep.type}-${currentStep.side}-${currentStep.label}`
    : "finished";

  useEffect(() => {
    if (!currentStep || finished) return;

    const duration = AI_TIMING.userTurnMs;

    setTurnDurationMs(duration);
    setTurnStartedAt(Date.now());
    setTurnRemainingMs(duration);
    autoResolvedStepKeyRef.current = null;
    setPendingChampionId(null);
    setSwapSource(null);
  }, [controlledSide, currentStepKey, currentStep, finished]);

  useEffect(() => {
    if (!currentStep || finished) return;

    const interval = setInterval(() => {
      const elapsed = Date.now() - turnStartedAt;
      const remaining = Math.max(0, turnDurationMs - elapsed);
      setTurnRemainingMs(remaining);
    }, 100);

    return () => clearInterval(interval);
  }, [currentStep, finished, turnDurationMs, turnStartedAt]);

  useEffect(() => {
    if (!currentStep || finished || loading || champions.length === 0) return;
    if (currentStep.side === controlledSide) return;

    const minMs = Math.min(AI_TIMING.aiMinMs, AI_TIMING.aiMaxMs);
    const maxMs = Math.max(AI_TIMING.aiMinMs, AI_TIMING.aiMaxMs);
    const aiDelay = minMs + Math.floor(Math.random() * Math.max(1, maxMs - minMs + 1));

    const timer = setTimeout(() => {
      if (autoResolvedStepKeyRef.current === currentStepKey) return;
      autoResolvedStepKeyRef.current = currentStepKey;
      const autoChampion = selectAiChampionForCurrentStep();
      if (autoChampion) {
        handleSelectChampion(autoChampion, "ai");
      }
    }, aiDelay);

    return () => clearTimeout(timer);
  }, [
    champions,
    controlledSide,
    currentStep,
    currentStepKey,
    finished,
    loading,
    usedChampionIds,
  ]);

  useEffect(() => {
    if (!currentStep || finished || loading || champions.length === 0) return;
    if (turnRemainingMs > 0) return;
    if (autoResolvedStepKeyRef.current === currentStepKey) return;

    autoResolvedStepKeyRef.current = currentStepKey;
    const autoChampion = selectAiChampionForCurrentStep();
    if (autoChampion) {
      handleSelectChampion(autoChampion, "ai");
    }
  }, [
    champions,
    currentStep,
    currentStepKey,
    finished,
    loading,
    turnRemainingMs,
    usedChampionIds,
  ]);

  const scoreDraft = (side: Side): DraftScoreBreakdown => {
    const ownPicks = side === "blue" ? bluePicks : redPicks;
    const enemyPicks = side === "blue" ? redPicks : bluePicks;
    const ownPlan = planTempo(side === "blue" ? snapshot.home_team.play_style : snapshot.away_team.play_style);
    const teamId = side === "blue" ? snapshot.home_team.id : snapshot.away_team.id;
    const staffEffects = getLolStaffEffectsForTeam(gameState, teamId);

    let mastery = 0;
    let synergy = 0;
    let counter = 0;
    let comfort = 0;
    let preparation = 0;

    ownPicks.forEach((pick, idx) => {
      const champMastery = resolvePlayerMastery(side, idx, pick.championId);
      if (champMastery >= 75) mastery += 3;
      else if (champMastery >= 50) mastery += 2;
      else if (champMastery >= 25) mastery += 1;

      const tempo = championTempo(pick.championId);
      if (tempo === ownPlan) comfort += 2;
      else if (
        (ownPlan === "early" && tempo === "late") ||
        (ownPlan === "late" && tempo === "early")
      ) {
        comfort -= 2;
      }

      enemyPicks.forEach((enemyPick) => {
        counter +=
          counterValue(pick.championId, enemyPick.championId) *
          AI_WEIGHTS.score.counterAdvantageWeight;
        counter -=
          counterValue(enemyPick.championId, pick.championId) *
          AI_WEIGHTS.score.counterRiskWeight;
      });
    });

    for (let i = 0; i < ownPicks.length; i += 1) {
      for (let j = i + 1; j < ownPicks.length; j += 1) {
        if (hasSynergy(ownPicks[i].championId, ownPicks[j].championId)) synergy += 2;
      }
    }

    if (ownPicks.length > 0) {
      preparation = Math.round(Math.max(-1, Math.min(3, (staffEffects.tactics - 1) * 4 + (staffEffects.analysis - 1) * 3)));
    }

    return {
      mastery,
      synergy,
      counter,
      comfort,
      preparation,
      total: mastery + synergy + counter + comfort + preparation,
    };
  };

  const blueScore = useMemo(() => scoreDraft("blue"), [bluePicks, redPicks, snapshot.home_team.id, snapshot.home_team.play_style, gameState?.staff]);
  const redScore = useMemo(() => scoreDraft("red"), [bluePicks, redPicks, snapshot.away_team.id, snapshot.away_team.play_style, gameState?.staff]);

  useEffect(() => {
    if (!finished) return;
    if (!blueRoleOrder && bluePicks.length > 0) {
      setBlueRoleOrder(buildBestRoleOrder("blue", bluePicks));
    }
    if (!redRoleOrder && redPicks.length > 0) {
      setRedRoleOrder(buildBestRoleOrder("red", redPicks));
    }
  }, [bluePicks, blueRoleOrder, finished, redPicks, redRoleOrder]);

  useEffect(() => {
    if (!finished) {
      finalRoleReassignFxPlayedRef.current = false;
      setShowFinalRoleReassignFx(false);
      return;
    }

    if (finalRoleReassignFxPlayedRef.current) return;

    const blueReady = bluePicks.length === 0 || !!blueRoleOrder;
    const redReady = redPicks.length === 0 || !!redRoleOrder;
    if (!blueReady || !redReady) return;

    finalRoleReassignFxPlayedRef.current = true;
    setShowFinalRoleReassignFx(true);
    const timeoutId = setTimeout(() => setShowFinalRoleReassignFx(false), 800);
    return () => clearTimeout(timeoutId);
  }, [bluePicks.length, blueRoleOrder, finished, redPicks.length, redRoleOrder]);

  const armSwapFromRole = (side: Side, roleIdx: number): void => {
    if (!finished) return;
    setSwapSource((prev) => {
      if (prev && prev.side === side && prev.index === roleIdx) return null;
      return { side, index: roleIdx };
    });
  };

  const completeSwapOnRole = (side: Side, targetRoleIdx: number): void => {
    if (!swapSource || swapSource.side !== side) return;
    const roleOrder = side === "blue" ? blueRoleOrder : redRoleOrder;
    if (!roleOrder) {
      setSwapSource(null);
      return;
    }

    if (swapSource.index === targetRoleIdx) {
      setSwapSource(null);
      return;
    }

    const next = [...roleOrder];
    [next[swapSource.index], next[targetRoleIdx]] = [next[targetRoleIdx], next[swapSource.index]];
    if (side === "blue") setBlueRoleOrder(next);
    else setRedRoleOrder(next);
    setSwapSource(null);
  };

  const controlledScore = controlledSide === "blue" ? blueScore : redScore;
  const rivalScore = controlledSide === "blue" ? redScore : blueScore;

  const rivalMasteryDisplay = useMemo(() => {
    if (!rivalTeamSeed) return [];

    const rivalSide: Side = controlledSide === "blue" ? "red" : "blue";
    const rivalRosterPlayers = rivalSide === "blue" ? bluePlayers : redPlayers;
    const rivalRosterPlayerIds = rivalSide === "blue" ? bluePlayerIds : redPlayerIds;
    const userTeamId = controlledSide === "blue" ? snapshot.home_team.id : snapshot.away_team.id;
    const staffEffects = getLolStaffEffectsForTeam(gameState, userTeamId);
    const staffRevealBudget = calculateStaffRevealBudget(staffEffects.metaDiscovery);

    const scoutedPlayerKeys = new Set<string>();
    (gameState?.messages ?? []).forEach((message) => {
      const report = message.context?.scout_report;
      if (!report) return;
      const reportPlayerId = report.player_id;
      const matchedGamePlayer = gameState?.players.find((player) => player.id === reportPlayerId);
      if (matchedGamePlayer) {
        scoutedPlayerKeys.add(normalizeKey(matchedGamePlayer.match_name));
      }
      if (report.player_name) {
        scoutedPlayerKeys.add(normalizeKey(report.player_name));
      }
    });

    const rivalSeedPlayers = PLAYER_SEEDS.filter((player) => player.teamId === rivalTeamSeed.id);
    const byIgn = new Map<string, PlayerSeed>();
    rivalSeedPlayers.forEach((player) => {
      byIgn.set(normalizeKey(player.ign), player);
    });

    const matchedPlayers: Array<{ seed: PlayerSeed; playerId: string | null }> = [];
    const matchedKeys = new Set<string>();
    rivalRosterPlayers.forEach((player, index) => {
      const key = normalizeKey(player.name);
      const match = byIgn.get(key);
      if (match && !matchedKeys.has(key)) {
        matchedPlayers.push({ seed: match, playerId: rivalRosterPlayerIds[index] ?? null });
        matchedKeys.add(key);
      }
    });

    if (matchedPlayers.length < 5) {
      rivalSeedPlayers
        .filter((player) => !matchedKeys.has(normalizeKey(player.ign)))
        .sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0))
        .slice(0, 5 - matchedPlayers.length)
        .forEach((player) => matchedPlayers.push({ seed: player, playerId: null }));
    }

    if (matchedPlayers.length === 0) return [];

    const selectedChampionIds = new Set<string>();
    const result: RivalMasteryDisplayEntry[] = [];
    const staffCandidates: RivalMasteryDisplayEntry[] = [];

    const addKnownChampion = (entry: RivalMasteryDisplayEntry): boolean => {
      if (selectedChampionIds.has(entry.champion.id)) return false;
      selectedChampionIds.add(entry.champion.id);
      result.push(entry);
      return true;
    };

    matchedPlayers.forEach(({ seed, playerId }) => {
      const isScouted = Boolean(playerId && scoutedPlayerKeys.has(normalizeKey(seed.ign))) || scoutedPlayerKeys.has(normalizeKey(seed.ign));
      const allKnownOptions: RivalMasteryOption[] = (seed.champions ?? [])
        .map((entry) => {
          const championName = String(entry[0] ?? "");
          const mastery = Number(entry[1] ?? 0);
          if (!championName) return null;
          const champion = championLookupByNormalizedName.get(normalizeKey(championName));
          if (!champion) return null;
          return {
            champion,
            mastery,
            playerName: String(seed.ign ?? "").trim() || "N/A",
            playerRole: mapSeedRoleToDraftRole(String(seed.role ?? "")),
          };
        })
        .filter(
          (
            item,
          ): item is {
            champion: ChampionData;
            mastery: number;
            playerName: string;
            playerRole: Role | null;
          } => item !== null,
        )
        .sort((a, b) => b.mastery - a.mastery);

      const playerKnowledge = selectRivalMasteryKnowledgeForPlayer(
        allKnownOptions,
        usedChampionIds,
        selectedChampionIds,
        isScouted,
      );

      playerKnowledge.knownEntries.forEach(addKnownChampion);
      playerKnowledge.staffCandidates.forEach((candidate) => {
        staffCandidates.push(candidate);
      });
    });

    selectStaffRevealEntries(staffCandidates, staffRevealBudget, usedChampionIds)
      .forEach(addKnownChampion);

    return result;
  }, [
    bluePlayerIds,
    bluePlayers,
    championLookupByNormalizedName,
    controlledSide,
    gameState,
    redPlayers,
    redPlayerIds,
    rivalTeamSeed,
    snapshot.away_team.id,
    snapshot.home_team.id,
    usedChampionIds,
  ]);

  // ---------------------------------------------------------------------------
  // Dynamic Draft Tips - Assistant Coach & Player Suggestions
  // ---------------------------------------------------------------------------
  const assistantCoachTips = useMemo(() => {
    const tips: DraftAdviceTip[] = [];
    if (!gameState) return tips;

    const draftAdviceStage: "ban" | "pick" | "post" = finished
      ? "post"
      : currentStep?.type === "ban"
        ? "ban"
        : "pick";

    const uniquePhrase = (
      keyPrefix: string,
      pool: string[],
      seed: string,
      vars: Record<string, string | number>,
    ): string => {
      const { index, template } = pickPhrase(pool, seed);
      return t(`${keyPrefix}.${index}`, {
        ...vars,
        defaultValue: template,
      });
    };

    const userTeamId = controlledSide === "blue" ? snapshot.home_team.id : snapshot.away_team.id;
    const ownPicksList = controlledSide === "blue" ? bluePicks : redPicks;
    const enemyPicksList = controlledSide === "blue" ? redPicks : bluePicks;
    const ownCoveredRoles = new Set<Role>();
    ownPicksList.forEach((pick) => {
      const champion = championById.get(pick.championId);
      champion?.roleHints.forEach((role) => ownCoveredRoles.add(role));
    });
    const availableChampions = champions.filter((champion) => !usedChampionIds.has(champion.id));

    const assistantCoach = gameState.staff.find(
      (staff) => staff.team_id === userTeamId && staff.role === "AssistantManager",
    );
    const coachSkill = assistantCoach?.attributes?.coaching ?? 60;
    const coachName =
      assistantCoach && (assistantCoach.first_name || assistantCoach.last_name)
        ? `${assistantCoach.first_name ?? ""} ${assistantCoach.last_name ?? ""}`.trim()
        : t("match.draft.assistantCoach");

    const addCoachTip = (type: "ban" | "pick" | "warn", text: string, champion?: ChampionData) => {
      tips.push({
        sourceType: "coach",
        sourceName: coachName,
        sourceRole: t("match.draft.assistantCoach"),
        sourceImage: ASSISTANT_COACH_PLACEHOLDER,
        type,
        text,
      champion,
      });
    };

    const enemyPickedRoles = new Set<Role>();
    enemyPicksList.forEach((pick) => {
      const champion = championById.get(pick.championId);
      const primaryRole = champion?.roleHints?.[0];
      if (primaryRole) enemyPickedRoles.add(primaryRole);
    });

    const rivalMasteryCandidates = rivalMasteryDisplay
      .slice()
      .filter((entry) => !entry.playerRole || !enemyPickedRoles.has(entry.playerRole));
    const rivalMasteries = (rivalMasteryCandidates.length > 0 ? rivalMasteryCandidates : rivalMasteryDisplay)
      .slice()
      .sort((a, b) => b.mastery - a.mastery);
    if (draftAdviceStage === "ban" && rivalMasteries.length > 0 && coachSkill >= 50) {
      const topRival = rivalMasteries[0];
      if (topRival.mastery >= 75) {
        addCoachTip(
          "ban",
          uniquePhrase(
            "match.draft.phrases.coachBan",
            COACH_BAN_PHRASES,
            `coach-ban-${topRival.champion.id}-${topRival.playerName}-${stepIndex}`,
            {
            champion: topRival.champion.name,
            player: topRival.playerName,
            mastery: topRival.mastery,
            },
          ),
          topRival.champion,
        );
      }
    }

    if (coachSkill >= 70 && ownPicksList.length > 0) {
      const lastOwnPick = ownPicksList[ownPicksList.length - 1]?.championId;
      if (lastOwnPick && enemyPicksList.length > 0) {
        enemyPicksList.forEach((enemyPick) => {
          const counterVal = counterValue(lastOwnPick, enemyPick.championId);
          if (counterVal <= -2) {
            const enemyChamp = championById.get(enemyPick.championId);
            if (enemyChamp) {
              addCoachTip(
                "warn",
                t("match.draft.warnCounterLastPick", {
                  defaultValue: "Careful with {{champion}}: it is countering your last pick.",
                  champion: enemyChamp.name,
                }),
                enemyChamp,
              );
            }
          }
        });
      }
    }

    if (coachSkill >= 55) {
      const hasEarly = ownPicksList.some((pick) => getChampionTiming(pick.championId) === "Early");
      const hasLate = ownPicksList.some((pick) => getChampionTiming(pick.championId) === "Late");

      if (ownPicksList.length >= 3 && !hasEarly && !hasLate) {
        addCoachTip(
          "warn",
          t("match.draft.warnTimingIdentity", {
            defaultValue: "We are missing a clear timing identity. Look for an Early or Late pick to close better.",
          }),
        );
      }

      if (ownPicksList.length >= 4) {
        const rolesCovered = new Set<Role>();
        ownPicksList.forEach((pick) => {
          const champion = championById.get(pick.championId);
          champion?.roleHints.forEach((role) => rolesCovered.add(role));
        });

        if (rolesCovered.size < 4) {
          addCoachTip(
            "warn",
            t("match.draft.warnRoleCoverage", {
              defaultValue: "We still have role gaps. Add a more flexible pick.",
            }),
          );
        }
      }
    }

    const ownPlayers = controlledSide === "blue" ? bluePlayers : redPlayers;
    const ownTeamSeed = controlledSide === "blue" ? homeTeamSeed : awayTeamSeed;
    const ownTeamSeedId = ownTeamSeed?.id ?? null;
    const ownTeamSeedPlayers = ownTeamSeed
      ? PLAYER_SEEDS.filter((seedPlayer) => seedPlayer.teamId === ownTeamSeed.id)
      : [];
    const usedSeedPlayerIgns = new Set<string>();

    ownPlayers.slice(0, 5).forEach((player, index) => {
      const directPlayerKey = ownTeamSeedId ? `${ownTeamSeedId}:${normalizeKey(player.name)}` : "";
      let masteryMap = playerMasteryMap.get(directPlayerKey);
      let matchedSeedPlayer = ownTeamSeedPlayers.find(
        (seedPlayer) => normalizeKey(seedPlayer.ign) === normalizeKey(player.name),
      );
      if (matchedSeedPlayer) {
        usedSeedPlayerIgns.add(normalizeKey(matchedSeedPlayer.ign));
      }

      if (!masteryMap && ownTeamSeedPlayers.length > 0) {
        const role = ROLE_ORDER[index];
        const roleMatch = ownTeamSeedPlayers.find((seedPlayer) => {
          const roleOk = mapSeedRoleToDraftRole(seedPlayer.role) === role;
          const notUsed = !usedSeedPlayerIgns.has(normalizeKey(seedPlayer.ign));
          return roleOk && notUsed;
        });

        const fallback = roleMatch ?? ownTeamSeedPlayers.find(
          (seedPlayer) => !usedSeedPlayerIgns.has(normalizeKey(seedPlayer.ign)),
        );

        if (fallback) {
          usedSeedPlayerIgns.add(normalizeKey(fallback.ign));
          matchedSeedPlayer = fallback;
          masteryMap = ownTeamSeedId
            ? playerMasteryMap.get(`${ownTeamSeedId}:${normalizeKey(fallback.ign)}`)
            : undefined;
        }
      }

      if (!masteryMap) return;

      const sourceRole = mapSeedRoleToDraftRole(String(matchedSeedPlayer?.role ?? "")) ?? ROLE_ORDER[index];
      const sourceImage =
        resolvePlayerPhoto(player.id, player.name) ??
        playerSeedPhotoUrl(matchedSeedPlayer?.photo) ??
        `/player-photos/${player.id}.png`;

      const playerState = gameState.players.find((item) => item.id === player.id);
      const gameIq = playerState
        ? Math.round(
          (Number(playerState.attributes.decisions ?? 70) +
            Number(playerState.attributes.vision ?? 70) +
            Number(playerState.attributes.positioning ?? 70) +
            Number(playerState.attributes.composure ?? 70)) /
          4,
        )
        : 70;

      const masteredChampions: Array<{ champion: ChampionData; mastery: number }> = [];
      masteryMap.forEach((mastery, normalizedChampionKey) => {
        const champion = championLookupByNormalizedName.get(normalizedChampionKey);
        if (!champion) return;
        masteredChampions.push({ champion, mastery });
      });

      if (masteredChampions.length === 0) return;
      masteredChampions.sort((a, b) => b.mastery - a.mastery);

      const bestChampionAny = masteredChampions[0];

      let bestMastery = 0;
      let resolvedBestChampion: ChampionData | null = null;

      for (const { champion, mastery } of masteredChampions) {
        if (usedChampionIds.has(champion.id)) continue;
        if (mastery > bestMastery) {
          bestMastery = mastery;
          resolvedBestChampion = champion;
        }
      }

      if (!resolvedBestChampion) return;

      const strongCounterTarget = enemyPicksList
        .map((enemyPick) => {
          const enemyChampion = championById.get(enemyPick.championId);
          if (!enemyChampion) return null;
          return {
            enemyChampion,
            value: counterValue(resolvedBestChampion.id, enemyChampion.id),
          };
        })
        .filter((item): item is { enemyChampion: ChampionData; value: number } => item !== null)
        .sort((a, b) => b.value - a.value)[0];

      if (draftAdviceStage !== "ban" && bestMastery >= 50) {
        if (sourceRole && ownCoveredRoles.has(sourceRole)) {
          return;
        }
        const pickPhraseText =
          strongCounterTarget && strongCounterTarget.value >= 2
            ? uniquePhrase(
              gameIq >= 78
                ? "match.draft.phrases.playerSmartCounterPick"
                : "match.draft.phrases.playerCounterPick",
              gameIq >= 78 ? PLAYER_SMART_COUNTER_PICK_PHRASES : PLAYER_COUNTER_PICK_PHRASES,
                 `player-counter-pick-${player.id}-${resolvedBestChampion.id}-${strongCounterTarget.enemyChampion.id}-${stepIndex}`,
               {
                 champion: resolvedBestChampion.name,
                 enemy: strongCounterTarget.enemyChampion.name,
               },
             )
            : uniquePhrase(
              "match.draft.phrases.playerComfortPick",
              PLAYER_COMFORT_PICK_PHRASES,
               `player-comfort-pick-${player.id}-${resolvedBestChampion.id}-${stepIndex}`,
               { champion: resolvedBestChampion.name, mastery: bestMastery },
             );

        tips.push({
          sourceType: "player",
          sourceName: player.name,
          sourceRole,
          sourceImage,
          type: "pick",
          text: pickPhraseText,
          champion: resolvedBestChampion,
        });
      }

      const strongestThreat = availableChampions
        .map((champion) => ({ champion, value: counterValue(champion.id, bestChampionAny.champion.id) }))
        .filter((item) => item.value >= 2)
        .sort((a, b) => b.value - a.value)[0];

      if (draftAdviceStage === "ban" && strongestThreat && bestChampionAny.mastery >= 70) {
        const banTarget =
          availableChampions
            .map((champion) => ({ champion, value: counterValue(champion.id, bestChampionAny.champion.id) }))
            .filter((item) => item.value >= 2)
            .sort((a, b) => b.value - a.value)[0]?.champion ?? strongestThreat.champion;

        tips.push({
          sourceType: "player",
          sourceName: player.name,
          sourceRole,
          sourceImage,
          type: "ban",
          text: uniquePhrase(
            "match.draft.phrases.playerBanRequest",
            PLAYER_BAN_REQUEST_PHRASES,
              `player-ban-${player.id}-${banTarget.id}-${bestChampionAny.champion.id}-${stepIndex}`,
              {
                threat: banTarget.name,
                champion: bestChampionAny.champion.name,
              },
            ),
          champion: banTarget,
        });
      }
    });

    const hasPlayerPickTip = tips.some((tip) => tip.sourceType === "player" && tip.type === "pick");
    if (draftAdviceStage !== "ban" && !hasPlayerPickTip) {
      const fallbackPlayerIndex = ownPlayers.findIndex((_, index) => {
        const fallbackRole =
          mapSeedRoleToDraftRole(String(ownTeamSeedPlayers[index]?.role ?? "")) ?? ROLE_ORDER[index] ?? null;
        return fallbackRole ? !ownCoveredRoles.has(fallbackRole) : true;
      });
      const fallbackPlayer =
        fallbackPlayerIndex >= 0 ? ownPlayers[fallbackPlayerIndex] : ownPlayers[0];
      const fallbackChampion = availableChampions[0];
      const fallbackSeedPlayer =
        fallbackPlayerIndex >= 0 ? ownTeamSeedPlayers[fallbackPlayerIndex] : ownTeamSeedPlayers[0];
      if (fallbackPlayer && fallbackChampion) {
        tips.push({
          sourceType: "player",
          sourceName: fallbackPlayer.name,
          sourceRole: mapSeedRoleToDraftRole(String(fallbackSeedPlayer?.role ?? "")) ?? ROLE_ORDER[0],
          sourceImage:
            resolvePlayerPhoto(fallbackPlayer.id, fallbackPlayer.name) ??
            playerSeedPhotoUrl(fallbackSeedPlayer?.photo) ??
            `/player-photos/${fallbackPlayer.id}.png`,
          type: "pick",
          text: uniquePhrase(
            "match.draft.phrases.playerComfortPick",
            PLAYER_COMFORT_PICK_PHRASES,
            `player-fallback-pick-${fallbackPlayer.id}-${fallbackChampion.id}-${stepIndex}`,
            { champion: fallbackChampion.name, mastery: 60 },
          ),
          champion: fallbackChampion,
        });
      }
    }

    const coachTips = tips.filter((tip) => tip.sourceType === "coach");
    const playerTips = tips.filter((tip) => tip.sourceType === "player");

    if (draftAdviceStage === "pick") {
      const playerPickTips = playerTips.filter((tip) => tip.type === "pick").slice(0, 5);
      const coachWarnTips = coachTips.filter((tip) => tip.type !== "pick").slice(0, 1);
      return [...playerPickTips, ...coachWarnTips].slice(0, 5);
    }

    const mixed: DraftAdviceTip[] = [];
    let coachIdx = 0;
    let playerIdx = 0;

    while (mixed.length < 4 && (coachIdx < coachTips.length || playerIdx < playerTips.length)) {
      if (coachIdx < coachTips.length) {
        mixed.push(coachTips[coachIdx]);
        coachIdx += 1;
        if (mixed.length >= 4) break;
      }
      if (playerIdx < playerTips.length) {
        mixed.push(playerTips[playerIdx]);
        playerIdx += 1;
      }
    }

    return mixed.slice(0, 4);
  }, [
    champions,
    gameState,
    currentStep?.type,
    controlledSide,
    finished,
    homeTeamSeed,
    awayTeamSeed,
    snapshot.home_team.id,
    snapshot.away_team.id,
    stepIndex,
    bluePicks,
    redPicks,
    bluePlayers,
    redPlayers,
    playerMasteryMap,
    championById,
    championLookupByNormalizedName,
    usedChampionIds,
    rivalMasteryDisplay
  ]);

  const patchLabel =
    gameState?.champion_patch?.current_patch_label ??
    (typeof gameState?.champion_patch?.patch_year === "number" &&
    typeof gameState?.champion_patch?.patch_index_in_year === "number"
      ? `${gameState.champion_patch.patch_year}.${gameState.champion_patch.patch_index_in_year}`
      : typeof gameState?.champion_patch?.current_patch === "number"
        ? String(gameState.champion_patch.current_patch)
        : "--");

  const topBgChampion =
    draftHistory[draftHistory.length - 1] ??
    blueOrderedPicks.find((pick) => pick)?.championId ??
    redOrderedPicks.find((pick) => pick)?.championId ??
    "Sylas";
  const [displayedBgChampion, setDisplayedBgChampion] = useState(topBgChampion);
  const [bgOpacity, setBgOpacity] = useState(1);

  useEffect(() => {
    if (topBgChampion === displayedBgChampion) return;

    setBgOpacity(0);
    const timeoutId = setTimeout(() => {
      setDisplayedBgChampion(topBgChampion);
      setBgOpacity(1);
    }, 140);

    return () => clearTimeout(timeoutId);
  }, [displayedBgChampion, topBgChampion]);

  const blueTriCode = teamTriCode(snapshot.home_team.name);
  const redTriCode = teamTriCode(snapshot.away_team.name);
  const controlledTriCode = controlledSide === "blue" ? blueTriCode : redTriCode;
  const blueLogo = teamLogo(snapshot.home_team.name);
  const redLogo = teamLogo(snapshot.away_team.name);
  const seriesSquares = seriesLength === 5 ? 3 : seriesLength === 3 ? 2 : 0;
  const scoreDelta = controlledScore.total - rivalScore.total;
  const blueWinProb = Math.max(5, Math.min(95, Math.round(50 + scoreDelta * 4)));
  const grade =
    scoreDelta >= 6 ? "S" : scoreDelta >= 3 ? "A" : scoreDelta >= 0 ? "B" : scoreDelta >= -3 ? "C" : "D";
  const scoreRows = [
    { label: t("match.draft.scoreLabels.counter"), value: controlledScore.counter },
    { label: t("match.draft.scoreLabels.synergy"), value: controlledScore.synergy },
    { label: t("match.draft.scoreLabels.mastery"), value: controlledScore.mastery },
    { label: t("match.draft.scoreLabels.comfort"), value: controlledScore.comfort },
    { label: t("match.draft.scoreLabels.preparation"), value: controlledScore.preparation },
  ];
  const formattedScoreDelta = scoreDelta >= 0 ? `+${scoreDelta}` : `${scoreDelta}`;
  const seriesBansRequiresTwoRows = seriesLength > 1 && seriesLockedChampions.length > 10;

  return (
    <div className="h-screen overflow-hidden bg-[#0a0a0a] text-white p-3 md:p-4">
      <div className="w-full h-full max-w-[1350px] mx-auto flex flex-col gap-3 overflow-hidden">
        <section className="order-2 rounded-md overflow-hidden border border-[#222]">
          <div
            className={`relative flex items-stretch bg-linear-to-b from-[#032e35] via-[#021720] to-[#000] border-b-4 border-cyan-400 shadow-[0_0_16px_rgba(0,242,255,0.35)] ${
              seriesBansRequiresTwoRows ? "h-28" : "h-20"
            }`}
          >
            <div
              className="absolute inset-0"
              style={{
                backgroundImage: `url(${splashUrl(displayedBgChampion)})`,
                backgroundPosition: "center",
                backgroundSize: "cover",
                opacity: bgOpacity * 0.24,
                transition: "opacity 220ms ease",
              }}
            />
            <div className="absolute inset-0 bg-black/72" />

            <div className="relative z-10 w-[250px] p-2.5 border-t-2 border-cyan-400/90 bg-black/28">
              <p className="font-heading text-[13px] uppercase font-bold tracking-[0.08em] mb-1 truncate text-cyan-100 [text-shadow:0_0_10px_rgba(0,0,0,0.9)]">
                {blueHeader}
              </p>
              <div className="flex gap-1">
                {blueBanDisplay.map((championId, index) => {
                  const champion = championId ? championById.get(championId) : null;
                  return (
                    <button
                      key={`top-blue-ban-${index}`}
                      disabled
                      className="relative w-8 h-8 border border-white/25 bg-black overflow-hidden"
                    >
                      {champion ? (
                        <img
                          src={champion.image}
                          alt={champion.name}
                          className="w-full h-full object-cover grayscale opacity-70"
                        />
                      ) : null}
                      <span className="absolute top-1/2 -left-[10%] w-[120%] h-px bg-orange-500 -rotate-45" />
                    </button>
                  );
                })}
              </div>
            </div>

            <div className="relative z-10 flex-1 flex flex-col items-center justify-center gap-1.5 px-2">
              {seriesLength > 1 && seriesLockedChampions.length > 0 ? (
                <div className="flex flex-col items-center gap-1">
                  <p className="text-[11px] uppercase tracking-[0.2em] text-gray-100 font-heading font-bold">
                    {t("match.draft.seriesBans", { defaultValue: "SERIES BANS" })}
                  </p>
                  <div className="flex flex-wrap items-center justify-center gap-1 max-w-[520px]">
                    {seriesLockedChampions.map((champion) => (
                      <span
                        key={`series-lock-${champion.id}`}
                        className="relative w-8 h-8 border border-white/30 bg-black overflow-hidden"
                        title={champion.name}
                      >
                        <img
                          src={champion.image}
                          alt={champion.name}
                          className="w-full h-full object-cover grayscale opacity-80"
                        />
                        <span className="absolute top-1/2 -left-[10%] w-[120%] h-px bg-orange-500 -rotate-45" />
                      </span>
                    ))}
                  </div>
                </div>
              ) : null}

              <p className="text-[11px] uppercase tracking-[0.2em] text-gray-300 font-semibold">
                {t("match.draft.championSelection")}
              </p>
            </div>

            <div className="relative z-10 w-[250px] p-2.5 text-right border-t-2 border-orange-500/95 bg-black/28">
              <p className="font-heading text-[13px] uppercase font-bold tracking-[0.08em] mb-1 truncate text-orange-100 [text-shadow:0_0_10px_rgba(0,0,0,0.9)]">
                {redHeader}
              </p>
              <div className="flex gap-1 justify-end">
                {redBanDisplay.map((championId, index) => {
                  const champion = championId ? championById.get(championId) : null;
                  return (
                    <button
                      key={`top-red-ban-${index}`}
                      disabled
                      className="relative w-8 h-8 border border-white/25 bg-black overflow-hidden"
                    >
                      {champion ? (
                        <img
                          src={champion.image}
                          alt={champion.name}
                          className="w-full h-full object-cover grayscale opacity-70"
                        />
                      ) : null}
                      <span className="absolute top-1/2 -left-[10%] w-[120%] h-px bg-orange-500 -rotate-45" />
                    </button>
                  );
                })}
              </div>
            </div>
          </div>

          <div className="h-[340px] grid grid-cols-[repeat(5,minmax(0,1fr))_240px_repeat(5,minmax(0,1fr))] border-b-4 border-cyan-400 shadow-[0_0_16px_rgba(0,242,255,0.45)]">
            {blueOrderedPicks.map((pick, index) => (
              <DraftSlot
                key={`blue-slot-${ROLE_ORDER[index]}`}
                side="blue"
                playerName={bluePlayerLabels[index] ?? ROLE_ORDER[index]}
                role={ROLE_ORDER[index]}
                pick={pick}
                championById={championById}
                showSwapControls={finished && controlledSide === "blue"}
                onSwapArm={() => armSwapFromRole("blue", index)}
                onSwapTarget={() => completeSwapOnRole("blue", index)}
                swapArmed={swapSource?.side === "blue" && swapSource.index === index}
                swapTargetable={!!swapSource && swapSource.side === "blue" && swapSource.index !== index}
                reorderFxActive={showFinalRoleReassignFx}
              />
            ))}

            <div className="bg-linear-to-b from-[#032e35] via-[#021720] to-[#000] border-t-4 border-white flex flex-col items-center justify-between py-2 px-1">
              <div className="w-[88%] flex items-center justify-between mt-1 min-h-2">
                {seriesSquares > 0 ? (
                  <div className="flex gap-1">
                    {Array.from({ length: seriesSquares }).map((_, index) => (
                      <span
                        key={`blue-series-${index}`}
                        className={`w-4 h-2 border border-white/60 ${index < blueSeriesWins ? "bg-white" : "bg-black/60"}`}
                      />
                    ))}
                  </div>
                ) : (
                  <span />
                )}
                {seriesSquares > 0 ? (
                  <div className="flex gap-1">
                    {Array.from({ length: seriesSquares }).map((_, index) => (
                      <span
                        key={`red-series-${index}`}
                        className={`w-4 h-2 border border-white/60 ${index < redSeriesWins ? "bg-white" : "bg-black/60"}`}
                      />
                    ))}
                  </div>
                ) : (
                  <span />
                )}
              </div>

              <div className="w-[92%] grid grid-cols-[92px_auto_92px] items-center justify-center gap-2 mt-2">
                <div className="flex flex-col items-center w-[92px]">
                  {blueLogo ? (
                    <img
                      src={blueLogo}
                      alt={blueTriCode}
                      className="w-11 h-11 object-contain mb-1 drop-shadow-[0_0_8px_rgba(0,0,0,0.7)]"
                    />
                  ) : null}
                  <p
                    className={`w-full text-center ${tricodeSizeClass(blueTriCode)} leading-none font-black tracking-tight uppercase`}
                  >
                    {blueTriCode}
                  </p>
                </div>

                <div className="w-10 h-10" />

                <div className="flex flex-col items-center w-[92px]">
                  {redLogo ? (
                    <img
                      src={redLogo}
                      alt={redTriCode}
                      className="w-11 h-11 object-contain mb-1 drop-shadow-[0_0_8px_rgba(0,0,0,0.7)]"
                    />
                  ) : null}
                  <p
                    className={`w-full text-center ${tricodeSizeClass(redTriCode)} leading-none font-black tracking-tight uppercase`}
                  >
                    {redTriCode}
                  </p>
                </div>
              </div>

              <img
                src={LEC_LOGO_URL}
                alt={t("match.draft.leagueLogoAlt")}
                className="w-10 h-10 object-contain opacity-100 mt-1"
              />
              <p className="text-[11px] tracking-[0.15em] text-gray-300 uppercase mb-1">
                {t("match.draft.patchLabel", { patch: patchLabel })}
              </p>
            </div>

            {redOrderedPicks.map((pick, index) => (
              <DraftSlot
                key={`red-slot-${ROLE_ORDER[index]}`}
                side="red"
                playerName={redPlayerLabels[index] ?? ROLE_ORDER[index]}
                role={ROLE_ORDER[index]}
                pick={pick}
                championById={championById}
                showSwapControls={finished && controlledSide === "red"}
                onSwapArm={() => armSwapFromRole("red", index)}
                onSwapTarget={() => completeSwapOnRole("red", index)}
                swapArmed={swapSource?.side === "red" && swapSource.index === index}
                swapTargetable={!!swapSource && swapSource.side === "red" && swapSource.index !== index}
                reorderFxActive={showFinalRoleReassignFx}
              />
            ))}
          </div>
        </section>

        <section className="order-1 flex-1 min-h-0 rounded-md border border-cyan-400/25 bg-[#050608] p-3 overflow-hidden shadow-[0_0_28px_rgba(18,215,255,0.06)]">
          <div className="h-full min-h-0 grid grid-cols-1 xl:grid-cols-[270px_minmax(0,1fr)_270px] gap-3 items-stretch">
            <aside className="hidden xl:flex h-full flex-col gap-2 min-h-0 overflow-y-auto scrollbar-draft pr-1">
              {assistantCoachTips.length > 0 ? (
                <div className="rounded-md border border-cyan-400/25 bg-[#0a0b0f] p-3 text-[12px] text-gray-200">
                  <p className="font-heading uppercase tracking-wide text-xs text-white mb-2">
                    {t("match.draft.tipsTitle")}
                  </p>
                  {(() => {
                    const tip = assistantCoachTips[0];
                    if (!tip) return null;
                    return (
                      <article className="rounded-md border border-white/10 bg-[#070915] px-2.5 py-2">
                        <div className="flex items-start gap-2">
                          <img
                            src={tip.sourceImage}
                            alt={tip.sourceName}
                            className="w-8 h-8 rounded-full object-cover border border-cyan-400/40 bg-[#090d1f]"
                            loading="lazy"
                            onError={(event) => {
                              const target = event.currentTarget;
                              if (target.src.endsWith(ASSISTANT_COACH_PLACEHOLDER)) return;
                              target.src = ASSISTANT_COACH_PLACEHOLDER;
                            }}
                          />

                          <div className="min-w-0 flex-1">
                            <p className="text-[13px] font-semibold leading-none text-white truncate">
                              {tip.sourceName}
                            </p>
                            <div className="mt-1 flex items-center gap-1">
                              {tip.sourceType === "player" && tip.sourceRole && ROLE_ORDER.includes(tip.sourceRole as Role) ? (
                                <img
                                  src={ROLE_ICON_URLS[tip.sourceRole as Role]}
                                  alt={tip.sourceRole}
                                  className="w-3 h-3 invert opacity-75"
                                  loading="lazy"
                                />
                              ) : null}
                              <p className="text-[10px] uppercase tracking-wide text-cyan-300/90">
                                {tip.sourceRole
                                  ?? (tip.sourceType === "coach"
                                    ? t("match.draft.assistantCoach")
                                    : t("match.draft.playerLabel"))}
                              </p>
                            </div>
                          </div>
                        </div>

                        <p
                          className={`mt-2 text-[12px] leading-snug ${
                            tip.type === "ban" ? "text-orange-200" : tip.type === "pick" ? "text-cyan-200" : "text-yellow-200"
                          }`}
                        >
                          “{tip.text}”
                        </p>

                        {tip.champion ? (
                          <div className="mt-2 pt-2 border-t border-white/10 flex items-center gap-2">
                            <img
                              src={tip.champion.image}
                              alt={tip.champion.name}
                              className="w-7 h-7 rounded-sm object-cover border border-orange-400/60"
                              loading="lazy"
                            />
                            <span className="text-[11px] text-gray-200 truncate">{tip.champion.name}</span>
                          </div>
                        ) : null}
                      </article>
                    );
                  })()}
                </div>
              ) : (
                <div className="rounded-md border border-cyan-400/25 bg-[#0a0b0f] p-3 text-[12px] text-gray-200">
                  <p className="font-heading uppercase tracking-wide text-xs text-white mb-2">
                    {t("match.draft.tipsTitle")}
                  </p>
                  <p className="text-[12px] mb-1">• {t("match.draft.tip1")}</p>
                  <p className="text-[12px] mb-1">• {t("match.draft.tip2")}</p>
                  <p className="text-[12px]">• {t("match.draft.tip3")}</p>
                </div>
              )}

              <div className="rounded-md border border-orange-400/30 bg-[#0a0b0f] p-3 text-[12px] text-gray-200">
                <p className="font-heading uppercase tracking-wide text-xs text-white mb-2">
                  {t("match.draft.scoreTitle")}
                </p>
                <div className="mb-2 text-[11px] text-gray-300">
                  <p>
                    {t("match.draft.winProb")} <span className="text-cyan-300 font-semibold">{controlledTriCode} {blueWinProb}%</span>
                  </p>
                </div>
                <div className="space-y-1 text-[11px]">
                  {scoreRows.map((row) => (
                    <p key={row.label} className="text-gray-300">
                      {row.label}
                      <span className={`float-right font-semibold ${row.value >= 0 ? "text-cyan-300" : "text-red-400"}`}>
                        {row.value >= 0 ? `+${row.value}` : row.value}
                      </span>
                    </p>
                  ))}
                </div>
                <div className="mt-2 pt-2 border-t border-white/10 text-[11px]">
                  <p className="text-gray-300">
                    {t("match.draft.total")} <span className="float-right font-bold text-white">{controlledScore.total}</span>
                  </p>
                  <p className="text-gray-300 mt-1">
                    {t("match.draft.rivalTotal")} <span className="float-right font-bold text-gray-200">{rivalScore.total}</span>
                  </p>
                  <p className="text-gray-300 mt-1">
                    {t("match.draft.scoreDelta")} <span className={`float-right font-bold ${scoreDelta >= 0 ? "text-cyan-300" : "text-red-400"}`}>{formattedScoreDelta}</span>
                  </p>
                  <p className="text-gray-300 mt-1">
                    {t("match.draft.grade")} <span className="float-right font-bold text-orange-300">{grade}</span>
                  </p>
                </div>
              </div>
            </aside>

            <div className="h-full min-h-0 rounded-md border border-cyan-400/25 bg-[#0a0b0f]/95 p-3 space-y-2 relative overflow-hidden shadow-[inset_0_0_0_1px_rgba(255,255,255,0.03)] flex flex-col">
              <div className="absolute inset-0 pointer-events-none bg-[radial-gradient(circle_at_35%_45%,rgba(0,209,255,0.12),transparent_40%),radial-gradient(circle_at_70%_45%,rgba(255,146,56,0.1),transparent_45%)]" />

              <div className="relative grid grid-cols-1 lg:grid-cols-[1fr_auto_auto] gap-2 items-center">
                <div className="flex flex-wrap gap-1">
                  <button
                    className={`px-2 py-1 text-[11px] border rounded ${roleFilter === "ALL" ? "border-cyan-300/80 bg-cyan-400/10 text-cyan-100" : "border-white/20 text-gray-200"}`}
                    onClick={() => setRoleFilter("ALL")}
                  >
                    ALL
                  </button>
                  {ROLE_ORDER.map((role) => (
                      <button
                        key={`chip-${role}`}
                        className={`px-2 py-1 text-[11px] border rounded ${roleFilter === role ? "border-cyan-300/80 bg-cyan-400/10 text-cyan-100" : "border-white/20 text-gray-200"}`}
                        onClick={() => setRoleFilter(role)}
                      >
                      {role}
                    </button>
                  ))}
                </div>

                <select
                  className="rounded-md bg-[#111318] border border-white/15 px-2 py-1.5 text-xs"
                  value={masteryFilter}
                  onChange={(event) => setMasteryFilter(Number(event.target.value))}
                >
                  {[0, 10, 20, 25, 30].map((value) => (
                    <option key={value} value={value}>
                      {t("match.draft.masteryMin")} {value}
                    </option>
                  ))}
                </select>

                <input
                  value={searchTerm}
                  onChange={(event) => setSearchTerm(event.target.value)}
                  className="rounded-md bg-[#111318] border border-white/15 px-2 py-1.5 text-xs w-44"
                  placeholder={t("match.draft.searchPlaceholder")}
                />

                {!finished ? (
                  <button
                    type="button"
                    onClick={handleSkipDraft}
                    disabled={loading || champions.length === 0}
                    className="rounded-md bg-cyan-500/20 hover:bg-cyan-500/30 border border-cyan-300/50 disabled:opacity-40 disabled:cursor-not-allowed text-cyan-100 px-2.5 py-1.5 text-[11px] font-heading uppercase tracking-wide"
                  >
                    {t("match.draft.skipFast")}
                  </button>
                ) : null}
              </div>

              <div className="relative text-center text-xs text-gray-200">
                {finished
                  ? t("match.draft.completed")
                  : `${actionLabel} · ${sideLabel} ${currentStep?.label ?? ""}`}
                <span className="ml-2 text-gray-400">
                  {t("match.draft.stepCounter", {
                    current: currentStepNumber,
                    total: totalSteps,
                  })}
                </span>
                {!finished ? (
                  <span className="ml-2 font-semibold text-gray-300">
                    {Math.ceil(turnRemainingMs / 1000)}s
                  </span>
                ) : null}
              </div>

              <div className="relative w-full h-1.5 rounded-full bg-[#121624] overflow-hidden">
                <div
                  className={`h-full transition-[width] duration-100 ease-linear ${currentStep?.side === "blue" ? "bg-cyan-400" : "bg-orange-500"}`}
                  style={{ width: `${Math.max(0, Math.min(100, (turnRemainingMs / Math.max(1, turnDurationMs)) * 100))}%` }}
                />
              </div>

              {!finished && isUserTurn ? (
                <div className="relative flex items-center justify-between gap-2 rounded-md border border-white/12 bg-[#0c1018] px-2 py-1.5">
                  <p className="text-[11px] text-gray-300 truncate">
                    {pendingChampionId
                      ? `${t("match.draft.selected")}: ${championById.get(pendingChampionId)?.name ?? pendingChampionId}`
                      : t("match.draft.selectThenConfirm")}
                  </p>
                  <button
                    type="button"
                    onClick={handleConfirmPendingAction}
                    disabled={!pendingChampionId}
                    className="rounded-md bg-orange-500 hover:bg-orange-400 disabled:opacity-40 disabled:cursor-not-allowed text-navy-900 px-3 py-1 text-[11px] font-heading font-bold uppercase tracking-wide"
                  >
                    {currentStep?.type === "ban"
                      ? t("match.draft.actions.ban")
                      : t("match.draft.actions.pick")}
                  </button>
                </div>
              ) : null}

              <div className="relative min-h-0 flex-1 overflow-y-auto scrollbar-draft pr-1">
                {loading ? (
                  <p className="relative text-sm text-gray-300">{t("match.draft.loadingChampions")}</p>
                ) : error ? (
                  <p className="relative text-sm text-red-300">
                    {t("match.draft.loadError")}: {error}
                  </p>
                ) : visibleChampions.length === 0 ? (
                  <p className="relative text-sm text-gray-300">{t("match.draft.noChampionsForFilters")}</p>
                ) : (
                  <div className="relative grid grid-cols-4 sm:grid-cols-6 md:grid-cols-8 xl:grid-cols-10 gap-1">
                    {visibleChampions.map((champion) => {
                      const isUsed = usedChampionIds.has(champion.id);
                      return (
                        <button
                          key={champion.id}
                          onClick={() => handleChampionTileClick(champion)}
                          disabled={finished || !isUserTurn || isUsed}
                          className={`rounded-sm border transition-colors p-0.5 text-left overflow-hidden ${isUsed ? "border-white/10 bg-[#0d0f13]" : pendingChampionId === champion.id ? "border-orange-400 bg-orange-500/10" : "border-white/15 bg-[#111318] hover:border-orange-400"}`}
                        >
                          <img
                            src={champion.image}
                            alt={champion.name}
                            className={`w-full aspect-square object-cover ${isUsed ? "grayscale opacity-45" : ""}`}
                            loading="lazy"
                          />
                          <p
                            className={`px-1 py-0.5 text-[10px] font-semibold truncate border-t ${isUsed ? "bg-[#090a0d] border-white/5 text-gray-500" : "bg-[#0a0b0f] border-white/10"}`}
                          >
                            {champion.name}
                          </p>
                        </button>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>

            <aside className="hidden xl:flex h-full flex-col rounded-md border border-cyan-400/25 bg-[#0a0b0f] p-2 min-h-0 overflow-y-auto scrollbar-draft pr-1">
              <p className="text-[11px] font-heading uppercase tracking-wide text-gray-200 mb-2 text-right">
                {t("match.draft.enemyComfortTitle")}
              </p>
              <div className="space-y-1">
                {rivalMasteryDisplay.length === 0 ? (
                  <div className="rounded-sm border border-white/10 bg-[#111318] p-2 text-[10px] text-gray-500 text-center">
                    {t("match.draft.enemyComfortUnknown")}
                  </div>
                ) : null}
                {rivalMasteryDisplay.map(({ champion, mastery, playerName, source }) => (
                  <div key={`rival-row-${champion.id}-${playerName}-${source}`} className="rounded-sm border border-white/10 bg-[#111318] p-1">
                    <div className="flex items-center gap-2">
                      <img
                        src={champion.image}
                        alt={champion.name}
                        className="w-10 h-10 object-cover rounded-sm"
                        loading="lazy"
                      />
                      <div className="min-w-0 flex-1">
                        <p className="text-[11px] truncate">{champion.name}</p>
                        <p className="text-[10px] text-gray-400 truncate">{playerName}</p>
                        <p className="text-[9px] uppercase tracking-wide text-cyan-200/70 truncate">
                          {source === "insignia"
                            ? t("match.draft.masterySourceSignature")
                            : source === "scouting"
                              ? t("match.draft.masterySourceScouting")
                              : t("match.draft.masterySourceStaff")}
                        </p>
                        <div className="mt-1 h-1.5 bg-black/35 rounded overflow-hidden">
                          <div className="h-full bg-cyan-400" style={{ width: `${Math.min(100, mastery)}%` }} />
                        </div>
                      </div>
                      <p className="text-[11px] text-cyan-300 font-semibold">{mastery}</p>
                    </div>
                  </div>
                ))}
              </div>
            </aside>
          </div>
        </section>

        {finished ? (
          <button
            className="w-full md:w-80 self-center rounded-md bg-orange-500 hover:bg-orange-400 text-navy-900 py-2 font-heading font-bold uppercase tracking-wide"
            onClick={() =>
              onComplete({
                blue: {
                  picks: blueOrderedPicks.filter((pick): pick is DraftPick => pick !== null),
                  bans: blueBans,
                  score: blueScore,
                },
                red: {
                  picks: redOrderedPicks.filter((pick): pick is DraftPick => pick !== null),
                  bans: redBans,
                  score: redScore,
                },
                history: draftHistory,
              })
            }
          >
            {t("match.draft.finalizeDraft")}
          </button>
        ) : null}
      </div>
    </div>
  );
}

function DraftSlot({
  side,
  playerName,
  role,
  pick,
  championById,
  showSwapControls,
  onSwapArm,
  onSwapTarget,
  swapArmed,
  swapTargetable,
  reorderFxActive,
}: {
  side: Side;
  playerName: string;
  role: Role;
  pick: DraftPick | null;
  championById: Map<string, ChampionData>;
  showSwapControls: boolean;
  onSwapArm: () => void;
  onSwapTarget: () => void;
  swapArmed: boolean;
  swapTargetable: boolean;
  reorderFxActive: boolean;
}) {
  const { t } = useTranslation();
  const champion = pick ? championById.get(pick.championId) : null;

  return (
    <div
      onClick={() => {
        if (!showSwapControls || !swapTargetable) return;
        onSwapTarget();
      }}
      className={`relative bg-black overflow-hidden border-r border-white/10 text-left transition-all duration-300 ${side === "blue" ? "border-t-4 border-t-cyan-400" : "border-t-4 border-t-orange-500"} ${swapTargetable ? "ring-1 ring-orange-300/70" : ""} ${swapArmed ? "ring-2 ring-cyan-300" : ""} ${reorderFxActive ? "shadow-[0_0_16px_rgba(34,211,238,0.45)] scale-[1.01]" : ""}`}
    >
      {champion ? (
        <>
          <img
            src={loadingUrl(champion.id)}
            alt={champion.name}
            className="absolute inset-0 w-full h-full object-cover object-top opacity-95"
          />
          <div className="absolute inset-0 bg-linear-to-t from-black/20 via-transparent to-transparent" />
        </>
      ) : null}

      <div
        className={`absolute inset-y-0 ${side === "blue" ? "left-0 border-r border-r-cyan-300/40" : "right-0 border-l border-l-orange-400/45"} w-8 bg-black/65 border-white/25 z-20 flex items-center justify-center`}
      >
        <span
          className={`text-[13px] font-black uppercase tracking-[0.09em] text-white z-30 leading-none [writing-mode:vertical-rl] [text-shadow:0_0_10px_rgba(0,0,0,0.95)] ${side === "blue" ? "rotate-180" : ""}`}
        >
          {playerName}
        </span>
      </div>

      <img
        src={ROLE_ICON_URLS[role]}
        alt={role}
        className={`absolute bottom-2 w-[18px] invert opacity-70 ${side === "blue" ? "right-2" : "left-2"}`}
      />

      {showSwapControls ? (
        <div className={`absolute bottom-2 z-30 flex flex-col gap-1 ${side === "blue" ? "left-10" : "right-10"}`}>
          <button
            type="button"
            onClick={(event) => {
              event.stopPropagation();
              onSwapArm();
            }}
            className={`w-5 h-5 rounded border text-[10px] leading-none text-white ${swapArmed ? "bg-cyan-500/35 border-cyan-300/80" : "bg-black/65 border-white/25"}`}
            title={t("match.draft.swapTargetTitle")}
          >
            ▲
          </button>
          <button
            type="button"
            onClick={(event) => {
              event.stopPropagation();
              onSwapArm();
            }}
            className={`w-5 h-5 rounded border text-[10px] leading-none text-white ${swapArmed ? "bg-cyan-500/35 border-cyan-300/80" : "bg-black/65 border-white/25"}`}
            title={t("match.draft.swapTargetTitle")}
          >
            ▼
          </button>
        </div>
      ) : null}
    </div>
  );
}
