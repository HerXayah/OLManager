import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Sparkles, Clock3, Search } from "lucide-react";
import type { GameStateData } from "../../store/gameStore";
import championsSeed from "../../../data/lec/draft/champions.json";
import playersSeed from "../../../data/lec/draft/players.json";
import { setPlayerChampionTrainingTarget } from "../../services/playerService";
import { calculateLolOvr } from "../../lib/lolPlayerStats";
import { formatStaffEffectPercent, getLolStaffEffectsForTeam } from "../../lib/lolStaffEffects";

interface ChampionsTabProps {
  gameState: GameStateData;
  onGameUpdate: (state: GameStateData) => void;
}

type ChampionRolesMap = Record<string, string[]>;
type UiRole = "Top" | "Jungle" | "Mid" | "ADC" | "Support";

const CHAMPION_ROLES: ChampionRolesMap =
  ((championsSeed as { data?: { roles?: ChampionRolesMap } }).data?.roles ?? {}) as ChampionRolesMap;

const ROLE_ORDER: Record<UiRole, number> = {
  Top: 1,
  Jungle: 2,
  Mid: 3,
  ADC: 4,
  Support: 5,
};

const ROLE_ICON_URLS: Record<UiRole, string> = {
  Top: "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-top.png",
  Jungle:
    "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-jungle.png",
  Mid: "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-middle.png",
  ADC: "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-bottom.png",
  Support:
    "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-utility.png",
};

const CHAMPIONS_BY_ROLE = Object.entries(CHAMPION_ROLES).reduce<Record<UiRole, string[]>>(
  (acc, [champion, roles]) => {
    roles.forEach((role) => {
      const mapped = role === "Bot" ? "ADC" : role;
      if (!["Top", "Jungle", "Mid", "ADC", "Support"].includes(mapped)) return;
      const key = mapped as UiRole;
      if (!acc[key]) acc[key] = [];
      acc[key].push(champion);
    });
    return acc;
  },
  { Top: [], Jungle: [], Mid: [], ADC: [], Support: [] },
);

type PlayerSeedLite = {
  ign: string;
  role?: string;
};

const PLAYER_SEEDS: PlayerSeedLite[] = [
  ...(((playersSeed as { data?: { rostered_seeds?: PlayerSeedLite[] } }).data
    ?.rostered_seeds ?? []) as PlayerSeedLite[]),
  ...(((playersSeed as { data?: { free_agent_seeds?: PlayerSeedLite[] } }).data
    ?.free_agent_seeds ?? []) as PlayerSeedLite[]),
];

const PLAYER_SEED_BY_IGN = new Map<string, PlayerSeedLite>(
  PLAYER_SEEDS.map((entry) => [normalizeKey(entry.ign), entry]),
);

function normalizeKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]/g, "");
}

function normalizeRole(role: string): UiRole {
  const key = normalizeKey(role);
  if (key === "top") return "Top";
  if (key === "jungle") return "Jungle";
  if (key === "mid") return "Mid";
  if (key === "bot" || key === "adc") return "ADC";
  return "Support";
}

function inferLolRole(player: GameStateData["players"][number]): UiRole {
  const key = normalizeKey(player.natural_position || player.position || "");
  if (key.includes("defender") && !key.includes("midfielder")) return "Top";
  if (key.includes("midfielder") && !key.includes("attacking")) return "Jungle";
  if (key.includes("attackingmidfielder")) return "Mid";
  if (key.includes("forward") || key.includes("striker")) return "ADC";
  return "Support";
}

function playerPhotoUrl(playerId: string): string | null {
  const match = playerId.match(/^lec-player-(.+)$/);
  if (!match) return null;
  return `/player-photos/${match[1]}.png`;
}

function championTileUrl(championId: string): string {
  const normalized = normalizeKey(championId);
  const overrides: Record<string, string> = {
    fiddlestick: "Fiddlesticks",
    fiddlesticks: "Fiddlesticks",
    ksante: "KSante",
    kaisa: "Kaisa",
    khazix: "Khazix",
    kogmaw: "KogMaw",
    leesin: "LeeSin",
    reksai: "RekSai",
    velkoz: "Velkoz",
    wukong: "MonkeyKing",
  };
  const canonical = overrides[normalized] ?? championId;
  return `https://ddragon.leagueoflegends.com/cdn/img/champion/tiles/${canonical}_0.jpg`;
}

function tierLabelClass(tier: string): string {
  if (tier === "S") return "bg-red-400 text-black";
  if (tier === "A") return "bg-orange-300 text-black";
  if (tier === "B") return "bg-yellow-300 text-black";
  if (tier === "C") return "bg-lime-300 text-black";
  return "bg-green-300 text-black";
}

type SoloQTier = "Challenger" | "Grandmaster" | "Master";

const SOLOQ_POINTS_BASELINE = 3000;
const SOLOQ_POINTS_MIN = 3000;
const SOLOQ_POINTS_MAX = 7000;
const SOLOQ_GRANDMASTER_LP_CUTOFF = 800;
const SOLOQ_CHALLENGER_LP_CUTOFF = 1300;

function hashText(value: string): number {
  let hash = 0;
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash * 31 + value.charCodeAt(i)) >>> 0;
  }
  return hash;
}

function pseudoRandom(seed: string): number {
  return (hashText(seed) % 10000) / 10000;
}

function daysBetween(startIso: string, endIso: string): number {
  const start = new Date(startIso).getTime();
  const end = new Date(endIso).getTime();
  if (!Number.isFinite(start) || !Number.isFinite(end)) return 0;
  return Math.max(0, Math.floor((end - start) / (24 * 60 * 60 * 1000)));
}

function computeSoloQ(
  player: GameStateData["players"][number],
  gameState: GameStateData,
  masterySignal: number,
): {
  tier: SoloQTier;
  lp: number;
  delta: number;
} {
  const ovr = calculateLolOvr(player);
  const dayIndex = daysBetween(gameState.clock.start_date, gameState.clock.current_date);
  const baseline = 3520 + (ovr - 76) * 52 + ((hashText(player.id) % 121) - 60);

  let points = baseline;
  for (let day = 1; day <= dayIndex; day += 1) {
    const rand = pseudoRandom(`${player.id}:${day}`);
    const randDelta = Math.round(rand * 48 - 24);
    const skillDrift = Math.round((ovr - 78) * 0.35);
    const masteryDrift = Math.round(masterySignal * 0.2);
    points += randDelta + skillDrift + masteryDrift;
    points = Math.max(SOLOQ_POINTS_MIN, Math.min(SOLOQ_POINTS_MAX, points));
  }

  const lp = Math.max(0, Math.round(points - SOLOQ_POINTS_BASELINE));

  const yesterdayRand = pseudoRandom(`${player.id}:${Math.max(1, dayIndex)}`);
  const yesterdayDelta =
    Math.round(yesterdayRand * 48 - 24) +
    Math.round((ovr - 78) * 0.35) +
    Math.round(masterySignal * 0.2);

  if (lp >= SOLOQ_CHALLENGER_LP_CUTOFF) {
    return { tier: "Challenger", lp, delta: yesterdayDelta };
  }
  if (lp >= SOLOQ_GRANDMASTER_LP_CUTOFF) {
    return { tier: "Grandmaster", lp, delta: yesterdayDelta };
  }
  return { tier: "Master", lp, delta: yesterdayDelta };
}

function soloQTierClass(tier: SoloQTier): string {
  if (tier === "Challenger") return "text-yellow-300";
  if (tier === "Grandmaster") return "text-red-300";
  return "text-fuchsia-300";
}

function soloQEmblemUrl(tier: SoloQTier): string {
  if (tier === "Challenger") {
    return "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests/challenger.png";
  }
  if (tier === "Grandmaster") {
    return "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests/grandmaster.png";
  }
  return "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests/master.png";
}

function soloQMasteryMultiplier(tier: SoloQTier): number {
  if (tier === "Challenger") return 1.2;
  if (tier === "Grandmaster") return 1.0;
  return 0.8;
}

function getFocusMultiplier(focus: string | null | undefined): number {
  if (!focus) return 0.85;
  if (focus === "ChampionPoolPractice") return 1.25;
  if (focus === "IndividualCoaching") return 1.0;
  if (focus === "Scrims") return 0.85;
  if (focus === "MacroSystems") return 0.75;
  if (focus === "VODReview") return 0.7;
  return 0.85;
}

function expectedGainBadge(slotIndex: number, focus: string | null | undefined): {
  label: string;
  className: string;
  baseMult: number;
} {
  const priorityWeight = [1.0, 0.65, 0.4][slotIndex] ?? 0.35;
  const focusMult = getFocusMultiplier(focus);
  if (slotIndex === 0) return { label: "Alta", className: "text-emerald-300", baseMult: priorityWeight * focusMult };
  if (slotIndex === 1) return { label: "Media", className: "text-amber-300", baseMult: priorityWeight * focusMult };
  return { label: "Baja", className: "text-gray-300", baseMult: priorityWeight * focusMult };
}

const TIER_ORDER: Array<"S" | "A" | "B" | "C" | "D"> = ["S", "A", "B", "C", "D"];
const TIER_SORT_WEIGHT: Record<string, number> = { S: 0, A: 1, B: 2, C: 3, D: 4 };

export default function ChampionsTab({ gameState, onGameUpdate }: ChampionsTabProps) {
  const { t } = useTranslation();
  const [submittingKey, setSubmittingKey] = useState<string | null>(null);
  const [metaRoleFilter, setMetaRoleFilter] = useState<"ALL" | UiRole>("ALL");
  const managerTeamId = gameState.manager.team_id;
  const patch = gameState.champion_patch;
  const staffEffects = getLolStaffEffectsForTeam(gameState, managerTeamId);

  const ownPlayers = useMemo(() => {
    if (!managerTeamId) return [];
    const roleOf = (player: GameStateData["players"][number]) => {
      const seedEntry = PLAYER_SEED_BY_IGN.get(normalizeKey(player.match_name));
      return normalizeRole(seedEntry?.role ?? inferLolRole(player));
    };

    return gameState.players
      .filter((player) => player.team_id === managerTeamId)
      .sort((a, b) => {
        const roleDiff = ROLE_ORDER[roleOf(a)] - ROLE_ORDER[roleOf(b)];
        if (roleDiff !== 0) return roleDiff;
        return a.match_name.localeCompare(b.match_name);
      });
  }, [gameState.players, managerTeamId]);

  const managerTeam = useMemo(
    () => gameState.teams.find((team) => team.id === managerTeamId) ?? null,
    [gameState.teams, managerTeamId],
  );

  const masteryMap = useMemo(() => {
    const map = new Map<string, number>();
    (gameState.champion_masteries ?? []).forEach((entry) => {
      map.set(`${entry.player_id}:${normalizeKey(entry.champion_id)}`, entry.mastery);
    });
    return map;
  }, [gameState.champion_masteries]);

  const masterySignalByPlayer = useMemo(() => {
    const bucket = new Map<string, number[]>();
    (gameState.champion_masteries ?? []).forEach((entry) => {
      const list = bucket.get(entry.player_id) ?? [];
      list.push(Number(entry.mastery ?? 25));
      bucket.set(entry.player_id, list);
    });

    const signal = new Map<string, number>();
    bucket.forEach((values, playerId) => {
      const top = [...values].sort((a, b) => b - a).slice(0, 3);
      const avg = top.length > 0 ? top.reduce((sum, value) => sum + value, 0) / top.length : 25;
      signal.set(playerId, Math.max(0, avg - 60));
    });
    return signal;
  }, [gameState.champion_masteries]);

  const discoveredSet = useMemo(
    () => new Set((patch?.discovered_champion_ids ?? []).map(normalizeKey)),
    [patch?.discovered_champion_ids],
  );

  const discoveredTierByChampion = useMemo(() => {
    const map = new Map<string, string>();
    (patch?.hidden_meta ?? []).forEach((entry) => {
      const championKey = normalizeKey(entry.champion_id);
      if (!discoveredSet.has(championKey)) return;
      const tier = (entry.tier || "C").toUpperCase();
      const previous = map.get(championKey);

      if (!previous) {
        map.set(championKey, tier);
        return;
      }

      const previousWeight = TIER_SORT_WEIGHT[previous] ?? 99;
      const currentWeight = TIER_SORT_WEIGHT[tier] ?? 99;
      if (currentWeight < previousWeight) {
        map.set(championKey, tier);
      }
    });
    return map;
  }, [patch?.hidden_meta, discoveredSet]);

  const discoveredTierByRoleChampion = useMemo(() => {
    const map = new Map<string, string>();
    (patch?.hidden_meta ?? []).forEach((entry) => {
      const championKey = normalizeKey(entry.champion_id);
      if (!discoveredSet.has(championKey)) return;
      const role = normalizeRole(entry.role);
      const tier = (entry.tier || "C").toUpperCase();
      const key = `${role}:${championKey}`;
      const previous = map.get(key);

      if (!previous) {
        map.set(key, tier);
        return;
      }

      const previousWeight = TIER_SORT_WEIGHT[previous] ?? 99;
      const currentWeight = TIER_SORT_WEIGHT[tier] ?? 99;
      if (currentWeight < previousWeight) {
        map.set(key, tier);
      }
    });
    return map;
  }, [patch?.hidden_meta, discoveredSet]);

  const discoveredMeta = useMemo(() => {
    const all = (patch?.hidden_meta ?? []).filter((entry) => discoveredSet.has(normalizeKey(entry.champion_id)));
    if (metaRoleFilter !== "ALL") {
      return all.filter((entry) => normalizeRole(entry.role) === metaRoleFilter);
    }

    const dedupBestByChampion = new Map<string, typeof all[number]>();
    all.forEach((entry) => {
      const key = normalizeKey(entry.champion_id);
      const previous = dedupBestByChampion.get(key);
      if (!previous) {
        dedupBestByChampion.set(key, entry);
        return;
      }

      const previousWeight = TIER_SORT_WEIGHT[(previous.tier || "C").toUpperCase()] ?? 99;
      const currentWeight = TIER_SORT_WEIGHT[(entry.tier || "C").toUpperCase()] ?? 99;
      if (currentWeight < previousWeight) {
        dedupBestByChampion.set(key, entry);
      }
    });

    return [...dedupBestByChampion.values()];
  }, [patch?.hidden_meta, discoveredSet, metaRoleFilter]);

  const tierRows = useMemo(() => {
    const rows: Record<string, typeof discoveredMeta> = { S: [], A: [], B: [], C: [], D: [] };
    discoveredMeta.forEach((entry) => {
      const tier = (entry.tier || "C").toUpperCase();
      if (rows[tier]) rows[tier].push(entry);
    });
    TIER_ORDER.forEach((tier) => {
      rows[tier].sort((a, b) => a.champion_id.localeCompare(b.champion_id));
    });
    return rows;
  }, [discoveredMeta]);

  const discoveredPct = useMemo(() => {
    const totalChampionKeys = new Set((patch?.hidden_meta ?? []).map((entry) => normalizeKey(entry.champion_id)));
    if (totalChampionKeys.size === 0) return 0;

    const discoveredCount = [...discoveredSet].filter((key) => totalChampionKeys.has(key)).length;
    return Math.round((discoveredCount / totalChampionKeys.size) * 100);
  }, [patch?.hidden_meta, discoveredSet]);

  async function handleTrainingTargetChange(
    playerId: string,
    slotIndex: number,
    championId: string,
  ) {
    const key = `${playerId}:${slotIndex}`;
    setSubmittingKey(key);
    try {
      const updated = await setPlayerChampionTrainingTarget(
        playerId,
        slotIndex,
        championId.length > 0 ? championId : null,
      );
      onGameUpdate(updated);
    } finally {
      setSubmittingKey(null);
    }
  }

  return (
    <div className="space-y-6">
      <section className="rounded-2xl border border-yellow-400/30 bg-linear-to-br from-navy-900 via-navy-900 to-black p-5 shadow-[0_0_30px_rgba(251,191,36,0.08)]">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div>
            <p className="text-[11px] uppercase tracking-widest text-yellow-300/80 font-heading">
              {t("champions.patchLabel", "Patch")}
            </p>
            <h2 className="mt-1 text-2xl font-heading font-bold text-white">
              {patch?.current_patch_label || t("champions.patchFallback", "25.1")}
            </h2>
            <p className="mt-1 text-sm text-gray-300">
              {patch?.last_patch_date
                ? t("champions.patchLastDate", {
                  defaultValue: "Último update: {{date}}",
                  date: patch.last_patch_date,
                })
                : t("champions.patchPending", "Esperando primer update de parche")}
            </p>
          </div>

          <div className="min-w-[230px] rounded-xl border border-navy-600 bg-navy-900/70 px-4 py-3">
            <div className="flex items-center justify-between text-xs text-gray-300">
              <span className="inline-flex items-center gap-1"><Search className="h-3.5 w-3.5" />
                {t("champions.discoveryProgress", "Meta descubierto")}</span>
              <span className="font-semibold text-yellow-300">{discoveredPct}%</span>
            </div>
            <div className="mt-2 h-2 rounded-full bg-navy-700">
              <div className="h-2 rounded-full bg-linear-to-r from-yellow-400 to-amber-500" style={{ width: `${discoveredPct}%` }} />
            </div>
            <p className="mt-2 text-[11px] text-gray-400">
              {t("champions.staffMetaImpact", "Scout read")}: {formatStaffEffectPercent(staffEffects.metaDiscovery)} · {t("champions.staffMasteryImpact", "mastery learning")}: {formatStaffEffectPercent(staffEffects.development)}
            </p>
          </div>
        </div>

        <div className="mt-5 rounded-xl border border-navy-600 bg-navy-900/60 p-4">
          <div className="mb-3 flex items-center justify-between gap-3">
            <div className="flex items-center gap-2 text-sm text-yellow-300">
              <Sparkles className="h-4 w-4" />
              <span className="font-heading uppercase tracking-wider">{t("champions.metaTitle", "Meta del parche")}</span>
            </div>
            <div className="flex items-center gap-1 rounded-lg border border-navy-600 bg-black/20 p-1">
              <button
                type="button"
                onClick={() => setMetaRoleFilter("ALL")}
                className={`rounded-md px-2 py-1 text-[11px] font-heading ${metaRoleFilter === "ALL" ? "bg-yellow-400/20 text-yellow-200" : "text-gray-300"}`}
              >
                ALL
              </button>
              {(Object.keys(ROLE_ORDER) as UiRole[]).map((role) => (
                <button
                  key={role}
                  type="button"
                  onClick={() => setMetaRoleFilter(role)}
                  className={`rounded-md p-1 ${metaRoleFilter === role ? "bg-yellow-400/20" : "hover:bg-white/5"}`}
                  title={role}
                >
                  <img src={ROLE_ICON_URLS[role]} alt={role} className="h-4 w-4" />
                </button>
              ))}
            </div>
          </div>

          <div className="space-y-1">
            {TIER_ORDER.map((tier) => (
              <div key={tier} className="grid grid-cols-[56px_1fr] overflow-hidden rounded-lg border border-navy-600">
                <div className={`flex items-center justify-center text-xl font-heading font-bold ${tierLabelClass(tier)}`}>
                  {tier}
                </div>
                <div className="min-h-[70px] bg-black/30 p-2">
                  {tierRows[tier].length === 0 ? (
                    <p className="text-xs text-gray-500 italic">—</p>
                  ) : (
                    <div className="flex flex-wrap gap-2">
                      {tierRows[tier].map((entry) => (
                        <div key={`${tier}-${entry.champion_id}-${entry.role}`} className="relative group">
                          <div className="h-14 w-24 rounded-md border border-navy-500/80 bg-navy-800 transition-all duration-300 group-hover:-translate-y-0.5 group-hover:border-yellow-300 overflow-hidden">
                            <img
                              src={championTileUrl(entry.champion_id)}
                              alt={entry.champion_id}
                              className="h-full w-full object-cover"
                              loading="lazy"
                              onError={(event) => {
                                const element = event.currentTarget;
                                element.onerror = null;
                                element.src = `https://ddragon.leagueoflegends.com/cdn/15.7.1/img/champion/${entry.champion_id}.png`;
                              }}
                              title={`${entry.champion_id} · ${entry.role}`}
                            />
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="rounded-2xl border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-800 p-4">
        <div className="flex items-center gap-2 mb-3">
          <Clock3 className="h-4 w-4 text-amber-500" />
          <h3 className="font-heading font-bold uppercase tracking-wide text-gray-800 dark:text-gray-100">
            {t("champions.masteryTrainingTitle", "Entrenamiento de maestría")}
          </h3>
        </div>

        <div className="space-y-3">
          {ownPlayers.map((player) => {
            const seedEntry = PLAYER_SEED_BY_IGN.get(normalizeKey(player.match_name));
            const role = normalizeRole(seedEntry?.role ?? inferLolRole(player));
            const roleChampions = CHAMPIONS_BY_ROLE[role] ?? [];
            const sortedRoleChampions = [...roleChampions].sort((a, b) => {
              const aKey = normalizeKey(a);
              const bKey = normalizeKey(b);
              const aMastery = masteryMap.get(`${player.id}:${aKey}`) ?? 25;
              const bMastery = masteryMap.get(`${player.id}:${bKey}`) ?? 25;
              if (aMastery !== bMastery) return bMastery - aMastery;

              const aTier = discoveredTierByRoleChampion.get(`${role}:${aKey}`)
                ?? discoveredTierByChampion.get(aKey)
                ?? "";
              const bTier = discoveredTierByRoleChampion.get(`${role}:${bKey}`)
                ?? discoveredTierByChampion.get(bKey)
                ?? "";
              const aTierWeight = TIER_SORT_WEIGHT[aTier] ?? 99;
              const bTierWeight = TIER_SORT_WEIGHT[bTier] ?? 99;
              if (aTierWeight !== bTierWeight) return aTierWeight - bTierWeight;

              return a.localeCompare(b);
            });

            const targetsRaw = player.champion_training_targets ?? [];
            const legacy = player.champion_training_target ?? "";
            const targets = [
              targetsRaw[0] ?? legacy,
              targetsRaw[1] ?? "",
              targetsRaw[2] ?? "",
            ];
            const soloQ = computeSoloQ(player, gameState, masterySignalByPlayer.get(player.id) ?? 0);
            const effectiveFocus = player.training_focus ?? managerTeam?.training_focus ?? null;
            const soloQMult = soloQMasteryMultiplier(soloQ.tier);

            return (
              <div
                key={player.id}
                className="rounded-xl border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-900/40 p-3 transition-all duration-300 hover:border-yellow-400/50 hover:shadow-[0_8px_24px_rgba(245,158,11,0.12)]"
              >
                <div className="mb-2 flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <div className="h-10 w-10 overflow-hidden rounded-lg bg-gray-200 dark:bg-navy-700">
                      {playerPhotoUrl(player.id) ? (
                        <img
                          src={playerPhotoUrl(player.id) ?? ""}
                          alt={player.match_name}
                          className="h-full w-full object-cover"
                        />
                      ) : (
                        <div className="h-full w-full bg-linear-to-br from-navy-600 to-navy-800" />
                      )}
                    </div>
                    <div>
                      <p className="text-sm font-semibold text-gray-800 dark:text-gray-100">{player.match_name}</p>
                      <p className="text-[11px] text-gray-500 dark:text-gray-400">{role}</p>
                    </div>
                  </div>
                  <div className="flex items-center gap-3">
                    <div className="text-right">
                      <p className={`text-[11px] font-heading uppercase tracking-wide ${soloQTierClass(soloQ.tier)}`}>
                        {soloQ.tier}
                      </p>
                      <p className="text-[11px] text-white font-semibold">
                        {soloQ.lp} LP
                        <span className={`ml-1 ${soloQ.delta >= 0 ? "text-emerald-300" : "text-rose-300"}`}>
                          {soloQ.delta >= 0 ? `+${soloQ.delta}` : soloQ.delta}
                        </span>
                      </p>
                      <p className="text-[10px] text-gray-300">x{soloQMult.toFixed(1)} mastery</p>
                    </div>
                    <img
                      src={soloQEmblemUrl(soloQ.tier)}
                      alt={soloQ.tier}
                      className="h-16 w-16 object-contain drop-shadow-[0_0_10px_rgba(0,0,0,0.5)]"
                    />
                    <img src={ROLE_ICON_URLS[role]} alt={role} className="h-5 w-5" />
                  </div>
                </div>

                <div className="grid gap-2 md:grid-cols-3">
                  {targets.map((target, slotIndex) => {
                    const masteryValue = target
                      ? masteryMap.get(`${player.id}:${normalizeKey(target)}`) ?? 25
                      : 25;
                    const gainHint = expectedGainBadge(slotIndex, effectiveFocus);

                    return (
                      <div key={`${player.id}-slot-${slotIndex}`} className="rounded-lg border border-gray-200 dark:border-navy-600 p-2">
                        <div className="mb-1 flex items-center justify-between">
                          <p className="text-[11px] font-heading uppercase tracking-wide text-gray-500 dark:text-gray-400">
                            P{slotIndex + 1}
                          </p>
                          <p className={`text-[10px] font-heading uppercase tracking-wide ${gainHint.className}`}>
                            Ganancia {gainHint.label}
                          </p>
                        </div>
                        <select
                          value={target}
                          disabled={submittingKey === `${player.id}:${slotIndex}`}
                          onChange={(event) => {
                            void handleTrainingTargetChange(player.id, slotIndex, event.target.value);
                          }}
                          className="w-full rounded-md border border-gray-300 dark:border-navy-500 bg-white dark:bg-navy-700 text-sm text-gray-800 dark:text-gray-100 px-2 py-1.5"
                        >
                          <option value="">{t("champions.noTarget", "Sin objetivo")}</option>
                          {sortedRoleChampions.map((champion) => {
                            const championKey = normalizeKey(champion);
                            const mastery = masteryMap.get(`${player.id}:${championKey}`) ?? 25;
                            const discoveredTier = discoveredTierByRoleChampion.get(`${role}:${championKey}`)
                              ?? discoveredTierByChampion.get(championKey)
                              ?? "?";
                            return (
                              <option key={`${player.id}-${slotIndex}-${champion}`} value={champion}>
                                {`${champion} · M${mastery} · Tier ${discoveredTier}`}
                              </option>
                            );
                          })}
                        </select>
                        <div className="mt-1 h-1.5 rounded-full bg-gray-200 dark:bg-navy-700">
                          <div
                            className="h-1.5 rounded-full bg-linear-to-r from-emerald-400 to-teal-500 transition-all duration-500"
                            style={{ width: `${Math.min(100, masteryValue)}%` }}
                          />
                        </div>
                        <p className="mt-1 text-[11px] text-right text-gray-500 dark:text-gray-300">
                          {target
                            ? `M ${masteryValue} · foco x${gainHint.baseMult.toFixed(2)} · soloQ x${soloQMult.toFixed(1)}`
                            : "—"}
                        </p>
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
      </section>
    </div>
  );
}
