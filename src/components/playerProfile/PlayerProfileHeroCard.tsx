import { useEffect, useState } from "react";
import { Pencil, Shield, User } from "lucide-react";
import type { PlayerData } from "../../store/gameStore";
import { formatPlayerMarketValue, formatPlayerWage } from "./PlayerProfile.helpers";
import type {
  PlayerProfileScoutStatus,
  ScoutAvailability,
} from "./PlayerProfile.scouting";
import PlayerProfileScoutAction from "./PlayerProfileScoutAction";
import { TraitList } from "../TraitBadge";
import { Badge, Card } from "../ui";

type TranslateFn = (
  key: string,
  options?: Record<string, string | number>,
) => string;

interface PlayerProfileHeroCardProps {
  player: PlayerData;
  ovr: number;
  primaryPosition: string;
  age: number;
  teamName: string;
  weeklySuffix: string;
  language: string;
  isOwnClub: boolean;
  scoutAvailability: ScoutAvailability;
  scoutStatus: PlayerProfileScoutStatus;
  scoutError: string | null;
  onScout: () => void;
  onRerollRole?: (role: "TOP" | "JUNGLA" | "MID" | "ADC" | "SUPPORT") => void;
  rerollingRole?: boolean;
  insigniaChampionId?: string | null;
  onSelectTeam?: (id: string) => void;
  t: TranslateFn;
}

export default function PlayerProfileHeroCard({
  player,
  ovr,
  primaryPosition,
  age,
  teamName,
  weeklySuffix,
  isOwnClub,
  scoutAvailability,
  scoutStatus,
  scoutError,
  onScout,
  onRerollRole,
  rerollingRole = false,
  insigniaChampionId = null,
  onSelectTeam,
  t,
}: PlayerProfileHeroCardProps) {
  const role = getLolRoleLabel(primaryPosition);
  const roleVariant = getLolRoleBadgeVariant(role);
  const playerPhoto = getPlayerPhotoPath(player.id);
  const [insigniaBackground, setInsigniaBackground] = useState<string | null>(null);
  const [editingRole, setEditingRole] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function resolveRandomSkin(): Promise<void> {
      if (!insigniaChampionId) {
        setInsigniaBackground(null);
        return;
      }

      const fallbackBase = getChampionSplashPath(insigniaChampionId, 0);
      if (fallbackBase) {
        setInsigniaBackground(fallbackBase);
      }

      const cacheKey = buildSkinCacheKey(player.id, insigniaChampionId);
      const cachedUrl = readCachedSkinUrl(cacheKey);
      if (cachedUrl) {
        if (await canLoadImage(cachedUrl)) {
          setInsigniaBackground(cachedUrl);
          return;
        }
        clearCachedSkinUrl(cacheKey);
      }

      try {
        const versionsResponse = await fetch("https://ddragon.leagueoflegends.com/api/versions.json");
        const versions = (await versionsResponse.json()) as string[];
        const version = versions[0];
        if (!version) {
          if (!cancelled && fallbackBase) setInsigniaBackground(fallbackBase);
          return;
        }

        const detailResponse = await fetch(
          `https://ddragon.leagueoflegends.com/cdn/${version}/data/en_US/champion/${insigniaChampionId}.json`,
        );
        const detailPayload = (await detailResponse.json()) as {
          data?: Record<string, { skins?: Array<{ num: number }> }>;
        };

        const championEntry = detailPayload.data?.[insigniaChampionId];
        const skinNums = (championEntry?.skins ?? [])
          .map((skin) => Number(skin.num))
          .filter((num) => Number.isFinite(num));

        const shuffled = shuffleNumbers(skinNums.length > 0 ? skinNums : [0]);
        const chosenSkinNum = shuffled[0] ?? 0;
        const chosenUrl = getChampionSplashPath(insigniaChampionId, chosenSkinNum);

        if (chosenUrl && !cancelled && (await canLoadImage(chosenUrl))) {
          setInsigniaBackground(chosenUrl);
          writeCachedSkinUrl(cacheKey, chosenUrl);
          return;
        }

        if (!cancelled && fallbackBase) {
          setInsigniaBackground(fallbackBase);
        }
      } catch {
        if (!cancelled && fallbackBase) {
          setInsigniaBackground(fallbackBase);
        }
      }
    }

    void resolveRandomSkin();

    return () => {
      cancelled = true;
    };
  }, [insigniaChampionId, player.id]);

  return (
    <Card accent="primary" className="mb-5">
      <div className="relative p-8 rounded-t-xl overflow-hidden">
        {insigniaBackground ? (
          <>
            <div className="absolute inset-0 bg-cover opacity-100" style={{ backgroundImage: `url(${insigniaBackground})`, backgroundPosition: "center 12%" }} />
            <div className="absolute inset-0 bg-linear-to-r from-black/88 via-black/28 to-transparent" />
          </>
        ) : (
          <div className="absolute inset-0 bg-linear-to-r from-navy-700 to-navy-800" />
        )}

        <div className="relative z-10 flex items-center gap-6">
          <div className="relative w-24 h-24 shrink-0">
            <div
              className={`w-24 h-24 rounded-2xl overflow-hidden border-2 ${
                ovr >= 75
                  ? "border-primary-500/40"
                  : ovr >= 55
                    ? "border-accent-500/40"
                    : "border-gray-500/40"
              }`}
            >
              {playerPhoto ? (
                <img
                  src={playerPhoto}
                  alt={player.match_name}
                  className="w-full h-full object-cover"
                  loading="lazy"
                />
              ) : (
                <div className="w-full h-full bg-navy-700 flex items-center justify-center text-gray-400">
                  <User className="w-10 h-10" />
                </div>
              )}
            </div>
            <div className="absolute -bottom-3 right-0 bg-navy-900 border border-navy-500 rounded-lg px-2 py-1 font-heading font-bold text-lg text-accent-300 leading-none">
              {ovr}
            </div>
          </div>

          <div className="flex-1">
            <h2 className="text-3xl font-heading font-bold text-white uppercase tracking-wide">
              {player.match_name}
            </h2>
            <div className="flex items-center gap-3 mt-2">
              <Badge variant={roleVariant}>{role}</Badge>
              {isOwnClub ? (
                <button
                  type="button"
                  onClick={() => setEditingRole((prev) => !prev)}
                  className="inline-flex items-center justify-center w-7 h-7 rounded-md border border-white/15 text-gray-200 hover:text-primary-300 hover:border-primary-400/60 transition-colors"
                  title={t("playerProfile.editPosition", { defaultValue: "Cambiar posición" })}
                >
                  <Pencil className="w-3.5 h-3.5" />
                </button>
              ) : null}
              <span className="text-gray-400 text-sm">
                {t("common.age")} {age}
              </span>
            </div>

            {isOwnClub && editingRole ? (
              <div className="mt-3 rounded-lg border border-amber-300/30 bg-amber-500/10 p-3 max-w-xl">
                <p className="text-[11px] text-amber-200 mb-2">
                  {t("playerProfile.rerollWarning", {
                    defaultValue:
                      "Cambiar posición hace un reroll y puede modificar el OVR del jugador.",
                  })}
                </p>
                <div className="flex flex-wrap gap-2">
                  {(["TOP", "JUNGLA", "MID", "ADC", "SUPPORT"] as const).map((candidateRole) => (
                    <button
                      key={candidateRole}
                      type="button"
                      disabled={rerollingRole}
                      onClick={() => {
                        if (!onRerollRole) return;
                        onRerollRole(candidateRole);
                        setEditingRole(false);
                      }}
                      className={`px-2.5 py-1 rounded-md text-xs font-heading font-bold uppercase tracking-wide border transition-colors ${
                        candidateRole === role
                          ? "bg-primary-500/15 border-primary-400 text-primary-300"
                          : "bg-white/5 border-white/15 text-gray-200 hover:border-primary-400/60"
                      } ${rerollingRole ? "opacity-60 cursor-not-allowed" : ""}`}
                    >
                      {candidateRole}
                    </button>
                  ))}
                </div>
              </div>
            ) : null}
            <p className="text-gray-400 text-sm mt-2 flex items-center gap-1.5">
              <Shield className="w-4 h-4" />
              {player.team_id ? (
                <button
                  onClick={() => onSelectTeam?.(player.team_id!)}
                  className="hover:text-primary-400 transition-colors underline underline-offset-2"
                >
                  {teamName}
                </button>
              ) : (
                <span>{teamName}</span>
              )}
            </p>
            {player.traits && player.traits.length > 0 ? (
              <div className="mt-3">
                <TraitList traits={player.traits} size="sm" />
              </div>
            ) : null}
          </div>

          {!isOwnClub ? (
            <div className="mt-3">
              <PlayerProfileScoutAction
                availability={scoutAvailability}
                scoutStatus={scoutStatus}
                scoutError={scoutError}
                onScout={onScout}
              />
            </div>
          ) : null}

          <div className="hidden md:flex items-center gap-3">
            <div className="flex items-center">
              <QuickStat
                label={t("common.potential", { defaultValue: "Potencial" })}
                value="??"
                color="text-gray-200"
              />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <QuickStat
                label={t("common.energy", { defaultValue: "Energía" })}
                value={`${player.condition}%`}
                color={player.condition >= 70 ? "text-primary-400" : "text-red-400"}
              />
              <QuickStat
                label={t("common.morale")}
                value={`${player.morale}%`}
                color={player.morale >= 70 ? "text-primary-400" : "text-accent-400"}
              />
              <QuickStat
                label={t("common.value")}
                value={formatPlayerMarketValue(player.market_value)}
                color="text-white"
              />
              <QuickStat
                label={t("common.wage")}
                value={formatPlayerWage(player.wage, weeklySuffix)}
                color="text-white"
              />
            </div>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-4 gap-px bg-gray-200 dark:bg-navy-600 md:hidden">
        <MobileQuickStat
          label={t("common.energy", { defaultValue: "Energía" })}
          value={`${player.condition}%`}
          color={player.condition >= 70 ? "text-primary-500" : "text-red-500"}
        />
        <MobileQuickStat
          label={t("common.morale")}
          value={`${player.morale}%`}
          color={player.morale >= 70 ? "text-primary-500" : "text-accent-500"}
        />
        <MobileQuickStat
          label={t("common.value")}
          value={formatPlayerMarketValue(player.market_value)}
          color="text-gray-700 dark:text-gray-200"
        />
        <MobileQuickStat
          label={t("common.wage")}
          value={formatPlayerWage(player.wage, weeklySuffix)}
          color="text-gray-700 dark:text-gray-200"
        />
      </div>
    </Card>
  );
}

function getPlayerPhotoPath(playerId: string): string | null {
  const match = playerId.match(/^lec-player-(.+)$/);
  if (!match) return null;
  return `/player-photos/${match[1]}.png`;
}

function getChampionSplashPath(championId: string | null | undefined, skinNum: number): string | null {
  if (!championId) return null;
  return `https://ddragon.leagueoflegends.com/cdn/img/champion/splash/${championId}_${skinNum}.jpg`;
}

function buildSkinCacheKey(playerId: string, championId: string): string {
  return `profile-hero-skin:${playerId}:${championId}`;
}

function readCachedSkinUrl(cacheKey: string): string | null {
  try {
    const raw = localStorage.getItem(cacheKey);
    if (!raw) return null;
    return raw;
  } catch {
    return null;
  }
}

function writeCachedSkinUrl(cacheKey: string, url: string): void {
  try {
    localStorage.setItem(cacheKey, url);
  } catch {
    // ignore storage failures
  }
}

function clearCachedSkinUrl(cacheKey: string): void {
  try {
    localStorage.removeItem(cacheKey);
  } catch {
    // ignore storage failures
  }
}

function shuffleNumbers(values: number[]): number[] {
  const copy = [...values];
  for (let i = copy.length - 1; i > 0; i -= 1) {
    const j = Math.floor(Math.random() * (i + 1));
    [copy[i], copy[j]] = [copy[j], copy[i]];
  }
  return copy;
}

function canLoadImage(url: string): Promise<boolean> {
  return new Promise((resolve) => {
    const image = new Image();
    image.onload = () => resolve(true);
    image.onerror = () => resolve(false);
    image.src = url;
  });
}

function getLolRoleLabel(position: string): "TOP" | "JUNGLA" | "MID" | "ADC" | "SUPPORT" {
  const normalized = (position || "").toLowerCase();
  if (normalized.includes("attackingmidfielder")) return "MID";
  if (normalized.includes("forward")) return "ADC";
  if (normalized.includes("defensivemidfielder") || normalized.includes("goalkeeper")) {
    return "SUPPORT";
  }
  if (normalized === "midfielder") return "JUNGLA";
  return "TOP";
}

function getLolRoleBadgeVariant(role: "TOP" | "JUNGLA" | "MID" | "ADC" | "SUPPORT") {
  switch (role) {
    case "TOP":
      return "accent" as const;
    case "JUNGLA":
      return "success" as const;
    case "MID":
      return "primary" as const;
    case "ADC":
      return "danger" as const;
    case "SUPPORT":
      return "neutral" as const;
  }
}

function QuickStat({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color: string;
}) {
  return (
    <div className="bg-black/42 border border-white/20 rounded-xl px-5 py-3 text-center min-w-25 backdrop-blur-xs">
      <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
        {label}
      </p>
      <p className={`font-heading font-bold text-xl mt-0.5 ${color}`}>{value}</p>
    </div>
  );
}

function MobileQuickStat({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color: string;
}) {
  return (
    <div className="bg-white dark:bg-navy-800 p-3 text-center">
      <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
        {label}
      </p>
      <p className={`font-heading font-bold text-lg mt-0.5 ${color}`}>{value}</p>
    </div>
  );
}
