import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  GameStateData,
  PlayerData,
  PlayerSelectionOptions,
} from "../../store/gameStore";
import { Card } from "../ui";
import { AlertTriangle, ChevronRight, Repeat, ShoppingCart, User } from "lucide-react";
import { calcAge, formatVal } from "../../lib/helpers";
import { useTranslation } from "react-i18next";
import ContextMenu from "../ContextMenu";
import playersSeed from "../../../data/lec/draft/players.json";
import { buildStartingXIIds, isPlayerOutOfPosition } from "./SquadTab.helpers";
import { calculateLolOvr } from "../../lib/lolPlayerStats";
import { resolvePlayerPhoto } from "../../lib/playerPhotos";

type LolRole = "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT";
type SortKey = "pos" | "ovr" | "condition" | "morale" | "age";

const LOL_ROLE_ORDER: Record<LolRole, number> = {
  TOP: 1,
  JUNGLE: 2,
  MID: 3,
  ADC: 4,
  SUPPORT: 5,
};

const ROLE_LABEL: Record<LolRole, string> = {
  TOP: "Top",
  JUNGLE: "JUNGLE",
  MID: "Mid",
  ADC: "Bot",
  SUPPORT: "Support",
};

const ROLE_ICON_URLS: Record<LolRole, string> = {
  TOP: "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-top.png",
  JUNGLE:
    "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-jungle.png",
  MID: "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-middle.png",
  ADC: "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-bottom.png",
  SUPPORT:
    "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-utility.png",
};

interface PlayerSeed {
  ign: string;
  role: string;
  champions: Array<Array<string | number>>;
}

const PLAYER_SEEDS: PlayerSeed[] = [
  ...(((playersSeed as { data?: { rostered_seeds?: PlayerSeed[] } }).data?.rostered_seeds ?? []) as PlayerSeed[]),
  ...(((playersSeed as { data?: { free_agent_seeds?: PlayerSeed[] } }).data?.free_agent_seeds ?? []) as PlayerSeed[]),
];

function normalizeKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z]/g, "");
}

function seedRoleToDraftRole(role: string): LolRole | null {
  const normalized = normalizeKey(role);
  if (normalized === "top") return "TOP";
  if (normalized === "jungle") return "JUNGLE";
  if (normalized === "mid" || normalized === "middle") return "MID";
  if (normalized === "bot" || normalized === "bottom" || normalized === "adc") return "ADC";
  if (normalized === "support" || normalized === "sup") return "SUPPORT";
  return null;
}

const ROLE_BY_IGN = new Map(
  PLAYER_SEEDS.map((player) => [normalizeKey(player.ign), String(player.role || "")]),
);

const TOP_3_CHAMPIONS_BY_IGN = new Map(
  PLAYER_SEEDS.map((player) => {
    const top = [...(player.champions ?? [])]
      .map((entry) => ({ name: String(entry[0] ?? ""), mastery: Number(entry[1] ?? 0) }))
      .filter((entry) => entry.name.length > 0)
      .sort((a, b) => b.mastery - a.mastery)
      .slice(0, 3)
      .map((entry) => entry.name);
    return [normalizeKey(player.ign), top] as const;
  }),
);

function roleFromPosition(position: string): LolRole {
  const normalized = normalizeKey(position || "");
  if (normalized.includes("attackingmidfielder") || normalized.includes("midlane")) return "MID";
  if (normalized.includes("forward") || normalized.includes("striker") || normalized.includes("adc")) return "ADC";
  if (normalized.includes("defensivemidfielder") || normalized.includes("goalkeeper") || normalized.includes("support")) {
    return "SUPPORT";
  }
  if (normalized.includes("jungle")) return "JUNGLE";
  if (normalized.includes("midfielder")) return "JUNGLE";
  return "TOP";
}

function resolveRole(player: PlayerData): LolRole {
  const fromSeed = seedRoleToDraftRole(ROLE_BY_IGN.get(normalizeKey(player.match_name)) ?? "");
  if (fromSeed) return fromSeed;
  return roleFromPosition(player.natural_position || player.position);
}

function championIdFromName(name: string): string | null {
  const normalized = normalizeKey(name);
  if (!normalized) return null;

  const overrides: Record<string, string> = {
    aurelionsol: "AurelionSol",
    belveth: "Belveth",
    chogath: "Chogath",
    drmundo: "DrMundo",
    jarvaniv: "JarvanIV",
    ksante: "KSante",
    kaisa: "Kaisa",
    khazix: "Khazix",
    kogmaw: "KogMaw",
    leblanc: "Leblanc",
    leesin: "LeeSin",
    monkeyking: "MonkeyKing",
    nunuandwillump: "Nunu",
    reksai: "RekSai",
    tahmkench: "TahmKench",
    twistedfate: "TwistedFate",
    velkoz: "Velkoz",
  };

  if (overrides[normalized]) return overrides[normalized];
  return normalized.charAt(0).toUpperCase() + normalized.slice(1);
}

function championPortraitUrl(championId: string | null): string | null {
  if (!championId) return null;
  // Tile art se parece más al "rostro" que el splash completo.
  return `https://ddragon.leagueoflegends.com/cdn/img/champion/tiles/${championId}_0.jpg`;
}

function clampBar(value: number): number {
  return Math.max(0, Math.min(100, value));
}

function formatContractMonth(date: string | null): string {
  if (!date) return "—";
  const parsed = new Date(`${date}T00:00:00`);
  if (Number.isNaN(parsed.getTime())) return date;
  return new Intl.DateTimeFormat("es-ES", { month: "short", year: "numeric" })
    .format(parsed)
    .replace(".", "");
}

interface SquadRosterViewProps {
  gameState: GameStateData;
  managerId: string;
  onGameUpdate?: (g: GameStateData) => void;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
}

export default function SquadRosterView({
  gameState,
  managerId,
  onGameUpdate,
  onSelectPlayer,
}: SquadRosterViewProps) {
  const { t } = useTranslation();
  const myTeam = gameState.teams.find((team) => team.manager_id === managerId);
  const [sortKey, setSortKey] = useState<SortKey>("pos");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");

  if (!myTeam) {
    return <p className="text-gray-500 dark:text-gray-400">{t("common.unemployed")}</p>;
  }

  const roster = gameState.players.filter((player) => player.team_id === myTeam.id);
  const available = roster.filter((player) => !player.injury);
  const startingXiIds = buildStartingXIIds(available, myTeam.starting_xi_ids || [], myTeam.formation || "4-4-2");
  const xiIds = new Set(startingXiIds);

  const sortedRoster = useMemo(() => {
    const sorted = [...roster].sort((a, b) => {
      switch (sortKey) {
        case "pos":
          return LOL_ROLE_ORDER[resolveRole(a)] - LOL_ROLE_ORDER[resolveRole(b)] || calculateLolOvr(b) - calculateLolOvr(a);
        case "ovr":
          return calculateLolOvr(a) - calculateLolOvr(b);
        case "condition":
          return a.condition - b.condition;
        case "morale":
          return a.morale - b.morale;
        case "age":
          return calcAge(a.date_of_birth) - calcAge(b.date_of_birth);
        default:
          return 0;
      }
    });
    return sortDir === "desc" ? sorted.reverse() : sorted;
  }, [roster, sortDir, sortKey]);

  const toggleSort = (nextKey: SortKey): void => {
    if (sortKey === nextKey) {
      setSortDir((prev) => (prev === "asc" ? "desc" : "asc"));
      return;
    }
    setSortKey(nextKey);
    setSortDir(nextKey === "ovr" ? "desc" : "asc");
  };

  return (
    <div className="max-w-7xl mx-auto flex flex-col gap-4">
      <Card>
        <div className="p-4 border-b border-[#22345d] bg-[#08132a] rounded-t-xl">
          <h3 className="text-sm font-heading font-bold text-blue-100 uppercase tracking-wide">
            {t("squad.detailsTitle", { defaultValue: "Detalles de plantilla" })}
          </h3>
          <div className="mt-3 flex flex-wrap gap-2">
            {([
              ["pos", t("squad.pos", { defaultValue: "Posición" })],
              ["ovr", t("common.ovr", { defaultValue: "OVR" })],
              ["condition", t("common.condition", { defaultValue: "Energía" })],
              ["morale", t("common.morale", { defaultValue: "Moral" })],
              ["age", t("common.age", { defaultValue: "Edad" })],
            ] as Array<[SortKey, string]>).map(([key, label]) => (
              <button
                key={key}
                className={`px-2.5 py-1 rounded-md text-xs font-heading font-bold uppercase tracking-wide border transition-colors ${
                  sortKey === key
                    ? "bg-primary-500/15 border-primary-400 text-primary-300"
                    : "bg-[#0f1e3c] border-[#2a3f6f] text-blue-200/80 hover:border-primary-400"
                }`}
                onClick={() => toggleSort(key)}
              >
                {label}
              </button>
            ))}
          </div>
        </div>

        <div className="p-3 md:p-4 space-y-2 bg-[#061027] rounded-b-xl">
          {sortedRoster.map((player) => {
            const role = resolveRole(player);
            const ovr = calculateLolOvr(player);
            const photo = resolvePlayerPhoto(player.id, player.match_name);
            const championNames = TOP_3_CHAMPIONS_BY_IGN.get(normalizeKey(player.match_name)) ?? [];
            const inXI = xiIds.has(player.id);
            const currentPos = player.natural_position || player.position;
            const wrongPos = inXI && isPlayerOutOfPosition(player, currentPos);
            const annualWage = player.wage * 52;

            const contextItems = [
              {
                label: t("squad.viewProfile", { defaultValue: "Ver perfil" }),
                icon: <User className="w-4 h-4" />,
                onClick: () => onSelectPlayer(player.id),
              },
              {
                label: "",
                icon: undefined,
                onClick: () => {},
                divider: true,
              },
              {
                label: player.transfer_listed
                  ? t("squad.removeFromTransferList", { defaultValue: "Quitar de transferibles" })
                  : t("squad.addToTransferList", { defaultValue: "Añadir a transferibles" }),
                icon: <ShoppingCart className="w-4 h-4" />,
                onClick: async () => {
                  try {
                    const updated = await invoke<GameStateData>("toggle_transfer_list", { playerId: player.id });
                    onGameUpdate?.(updated);
                  } catch {
                    return;
                  }
                },
              },
              {
                label: player.loan_listed
                  ? t("squad.removeFromLoanList", { defaultValue: "Quitar de cesión" })
                  : t("squad.addToLoanList", { defaultValue: "Añadir a cesión" }),
                icon: <Repeat className="w-4 h-4" />,
                onClick: async () => {
                  try {
                    const updated = await invoke<GameStateData>("toggle_loan_list", { playerId: player.id });
                    onGameUpdate?.(updated);
                  } catch {
                    return;
                  }
                },
              },
            ];

            return (
              <ContextMenu items={contextItems} key={player.id}>
                <button
                  className="w-full text-left rounded-xl border border-[#21365f] bg-[#13274a] hover:bg-[#17305a] transition-colors px-3 py-2.5"
                  onClick={() => onSelectPlayer(player.id)}
                >
                  <div className="grid grid-cols-1 xl:grid-cols-[34px_44px_minmax(220px,1fr)_72px_130px_170px_170px_110px_90px] items-center gap-3">
                    <div className="w-8 h-8 rounded-md bg-[#0f213f] border border-white/10 flex items-center justify-center">
                      <img src={ROLE_ICON_URLS[role]} alt={ROLE_LABEL[role]} className="w-4 h-4 object-contain opacity-90" />
                    </div>

                    {photo ? (
                      <img src={photo} alt={player.match_name} className="w-10 h-10 object-cover rounded-full shrink-0" loading="lazy" />
                    ) : (
                      <div className="w-10 h-10 rounded-full bg-[#0f213f] border border-white/10 shrink-0" />
                    )}

                    <div className="min-w-0 flex items-center gap-2">
                      <div className="min-w-0">
                        <p className="text-[28px] leading-none font-heading font-black text-amber-300 xl:hidden">{ovr}</p>
                        <p className="text-xl leading-none font-heading font-bold text-white truncate">{player.match_name}</p>
                        <p className="text-[12px] text-blue-200/70 truncate">{player.full_name}</p>
                        <p className="text-[11px] uppercase tracking-wide text-blue-200/70 xl:hidden">{ROLE_LABEL[role]}</p>
                      </div>
                      {wrongPos ? (
                        <span className="text-amber-400 shrink-0" title={t("squad.outOfPositionTooltip", { defaultValue: "Fuera de rol" })}>
                          <AlertTriangle className="w-4 h-4" />
                        </span>
                      ) : null}
                    </div>

                    <div className="hidden xl:block text-center">
                      <p className="text-[30px] leading-none font-heading font-black text-amber-300">{ovr}</p>
                      <p className="text-[11px] uppercase tracking-wide text-blue-200/65">OVR</p>
                    </div>

                    <div className="hidden xl:flex items-center gap-1.5 justify-start">
                      {championNames.map((name) => {
                        const portrait = championPortraitUrl(championIdFromName(name));
                        return (
                          <div key={name} className="w-6 h-6 rounded-sm bg-[#0d1d39] overflow-hidden border border-white/10" title={name}>
                            {portrait ? (
                              <div className="w-full h-full bg-cover bg-center" style={{ backgroundImage: `url(${portrait})` }} />
                            ) : null}
                          </div>
                        );
                      })}
                    </div>

                    <div className="hidden xl:block min-w-[150px]">
                      <div className="flex items-center justify-between mb-1">
                        <p className="text-[10px] uppercase tracking-wide text-blue-200/60">Moral</p>
                        <p className="text-[11px] font-heading font-bold text-emerald-300">{player.morale}</p>
                      </div>
                      <div className="w-full h-1.5 rounded-full bg-[#0a1b37] overflow-hidden">
                        <div className="h-full bg-emerald-400" style={{ width: `${clampBar(player.morale)}%` }} />
                      </div>
                    </div>

                    <div className="hidden xl:block min-w-[150px]">
                      <div className="flex items-center justify-between mb-1">
                        <p className="text-[10px] uppercase tracking-wide text-blue-200/60">Energía</p>
                        <p className="text-[11px] font-heading font-bold text-amber-300">{player.condition}</p>
                      </div>
                      <div className="w-full h-1.5 rounded-full bg-[#0a1b37] overflow-hidden">
                        <div className="h-full bg-amber-400" style={{ width: `${clampBar(player.condition)}%` }} />
                      </div>
                    </div>

                    <div className="hidden xl:block text-right min-w-[88px]">
                      <p className="text-sm font-heading font-bold text-white">{formatVal(annualWage)}</p>
                      <p className="text-[11px] text-blue-200/60">/año</p>
                    </div>

                    <div className="hidden xl:flex items-center justify-end gap-2 min-w-[100px] text-blue-200/70">
                      <span className="text-sm">{formatContractMonth(player.contract_end)}</span>
                      <ChevronRight className="w-4 h-4" />
                    </div>

                    <div className="xl:hidden mt-2 grid grid-cols-2 gap-3">
                      <div>
                        <div className="flex items-center justify-between mb-1">
                          <p className="text-[10px] uppercase tracking-wide text-blue-200/60">Moral</p>
                          <p className="text-[11px] font-heading font-bold text-emerald-300">{player.morale}</p>
                        </div>
                        <div className="w-full h-1.5 rounded-full bg-[#0a1b37] overflow-hidden">
                          <div className="h-full bg-emerald-400" style={{ width: `${clampBar(player.morale)}%` }} />
                        </div>
                      </div>
                      <div>
                        <div className="flex items-center justify-between mb-1">
                          <p className="text-[10px] uppercase tracking-wide text-blue-200/60">Energía</p>
                          <p className="text-[11px] font-heading font-bold text-amber-300">{player.condition}</p>
                        </div>
                        <div className="w-full h-1.5 rounded-full bg-[#0a1b37] overflow-hidden">
                          <div className="h-full bg-amber-400" style={{ width: `${clampBar(player.condition)}%` }} />
                        </div>
                      </div>
                    </div>
                  </div>
                </button>
              </ContextMenu>
            );
          })}

          {sortedRoster.length === 0 ? (
            <div className="p-8 text-center text-blue-200/70 font-heading uppercase tracking-wider text-sm">
              {t("squad.noPlayers", { defaultValue: "Sin jugadores" })}
            </div>
          ) : null}
        </div>
      </Card>
    </div>
  );
}
