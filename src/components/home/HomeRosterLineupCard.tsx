import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import playersSeed from "../../../data/lec/draft/players.json";

import { calcOvr } from "../../lib/helpers";
import type { PlayerData } from "../../store/gameStore";
import { Card, CardBody, CardHeader } from "../ui";

type DraftRole = "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT";

const ROLE_ORDER: DraftRole[] = ["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"];

interface HomeRosterLineupCardProps {
  roster: PlayerData[];
  onNavigate?: (tab: string) => void;
}

interface PlayerSeed {
  ign: string;
  role: string;
  champions: Array<Array<string | number>>;
}

const PLAYER_SEEDS: PlayerSeed[] = [
  ...(((playersSeed as { data?: { rostered_seeds?: PlayerSeed[] } }).data?.rostered_seeds ?? []) as PlayerSeed[]),
  ...(((playersSeed as { data?: { free_agent_seeds?: PlayerSeed[] } }).data?.free_agent_seeds ?? []) as PlayerSeed[]),
];

const ROLE_BY_IGN = new Map(
  PLAYER_SEEDS.map((player) => [normalizeKey(player.ign), String(player.role || "").toLowerCase()]),
);

const TOP_CHAMPION_BY_IGN = new Map(
  PLAYER_SEEDS.map((player) => {
    const best = [...(player.champions ?? [])]
      .map((entry) => ({ name: String(entry[0] ?? ""), mastery: Number(entry[1] ?? 0) }))
      .filter((entry) => entry.name.length > 0)
      .sort((a, b) => b.mastery - a.mastery)[0];

    return [normalizeKey(player.ign), best?.name ?? ""] as const;
  }),
);

function normalizeKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z]/g, "");
}

function positionToDraftRole(position: string): DraftRole | null {
  const normalized = normalizeKey(position);
  if (normalized === "defender") return "TOP";
  if (normalized === "midfielder") return "JUNGLE";
  if (normalized === "attackingmidfielder") return "MID";
  if (normalized === "forward") return "ADC";
  if (normalized === "defensivemidfielder" || normalized === "goalkeeper") return "SUPPORT";
  return null;
}

function seedRoleToDraftRole(role: string): DraftRole | null {
  const normalized = normalizeKey(role);
  if (normalized === "top") return "TOP";
  if (normalized === "jungle") return "JUNGLE";
  if (normalized === "mid") return "MID";
  if (normalized === "bot" || normalized === "bottom" || normalized === "adc") return "ADC";
  if (normalized === "support" || normalized === "sup") return "SUPPORT";
  return null;
}

function roleToOvrPosition(role: DraftRole): string {
  switch (role) {
    case "TOP":
      return "Defender";
    case "JUNGLE":
      return "Midfielder";
    case "MID":
      return "AttackingMidfielder";
    case "ADC":
      return "Forward";
    case "SUPPORT":
      return "DefensiveMidfielder";
  }
}

function playerPhotoUrl(playerId: string): string | null {
  const match = playerId.match(/^lec-player-(.+)$/);
  if (!match) return null;
  return `/player-photos/${match[1]}.png`;
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
    kaisa: "Kaisa",
    khazix: "Khazix",
    kogmaw: "KogMaw",
    leesin: "LeeSin",
    monkeyking: "MonkeyKing",
    nunuandwillump: "Nunu",
    reksai: "RekSai",
    tahmkench: "TahmKench",
    velkoz: "Velkoz",
  };

  if (overrides[normalized]) return overrides[normalized];

  const special = normalized.charAt(0).toUpperCase() + normalized.slice(1);
  return special;
}

function championSplashUrl(championId: string | null): string | null {
  if (!championId) return null;
  return `https://ddragon.leagueoflegends.com/cdn/img/champion/splash/${championId}_0.jpg`;
}

export default function HomeRosterLineupCard({
  roster,
  onNavigate,
}: HomeRosterLineupCardProps) {
  const { t } = useTranslation();

  const lineup = useMemo(
    () =>
      ROLE_ORDER.map((role) => {
        const candidates = roster
          .filter(
            (player) =>
              (
                seedRoleToDraftRole(
                  ROLE_BY_IGN.get(normalizeKey(player.match_name)) ?? "",
                ) ?? positionToDraftRole(player.natural_position || player.position)
              ) === role,
          )
          .sort(
            (a, b) =>
              calcOvr(b, roleToOvrPosition(role)) -
              calcOvr(a, roleToOvrPosition(role)),
          );

        return {
          role,
          player: candidates[0] ?? null,
        };
      }),
    [roster],
  );

  return (
    <Card>
      <CardHeader
        action={
          <button
            onClick={() => onNavigate?.("Squad")}
            className="text-primary-500 dark:text-primary-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-primary-600 dark:hover:text-primary-300 transition-colors"
          >
            {t("home.fullRoster", { defaultValue: "Full Roster" })}
          </button>
        }
      >
        {t("home.roster", { defaultValue: "Roster" })}
      </CardHeader>
      <CardBody>
        <div className="grid grid-cols-2 lg:grid-cols-5 gap-2">
          {lineup.map(({ role, player }) => {
            const photo = player ? playerPhotoUrl(player.id) : null;
            const ovr = player ? calcOvr(player, roleToOvrPosition(role)) : null;
            const condition = player?.condition ?? null;
            const morale = player?.morale ?? null;
            const topChampion = player
              ? TOP_CHAMPION_BY_IGN.get(normalizeKey(player.match_name)) ?? ""
              : "";
            const championSplash = championSplashUrl(championIdFromName(topChampion));

            return (
              <div
                key={role}
                className="relative overflow-hidden rounded-md border border-gray-100 dark:border-navy-600 bg-gray-50 dark:bg-navy-800/40 p-2"
              >
                {championSplash ? (
                  <>
                    <div
                      className="absolute inset-0 opacity-35 bg-cover bg-center"
                      style={{ backgroundImage: `url(${championSplash})` }}
                    />
                    <div className="absolute inset-0 bg-linear-to-b from-black/35 via-black/55 to-black/75" />
                  </>
                ) : null}

                <div className="relative z-10">
                <p className="text-[10px] font-heading font-bold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  {role}
                </p>

                <div className="mt-2 flex items-center gap-2">
                  {photo ? (
                    <img
                      src={photo}
                      alt={player?.match_name ?? role}
                      className="w-8 h-8 rounded-full object-cover border border-white/15"
                      loading="lazy"
                    />
                  ) : (
                    <div className="w-8 h-8 rounded-full bg-navy-700 border border-white/10" />
                  )}
                  <div className="min-w-0">
                    <p className="text-xs font-heading font-bold truncate text-gray-800 dark:text-gray-100">
                      {player?.match_name ?? "—"}
                    </p>
                    <p className="text-[10px] text-gray-500 dark:text-gray-400">
                      OVR {ovr ?? "—"}
                    </p>
                    {topChampion ? (
                      <p className="text-[10px] text-primary-300 truncate">{topChampion}</p>
                    ) : null}
                  </div>
                </div>

                <div className="mt-2 grid grid-cols-2 gap-1 text-[10px]">
                  <div className="rounded bg-navy-900/60 px-1.5 py-1 text-center">
                    <p className="text-gray-400">{t("common.condition", { defaultValue: "Condition" })}</p>
                    <p className="font-heading font-bold text-primary-400">
                      {condition !== null ? `${condition}%` : "—"}
                    </p>
                  </div>
                  <div className="rounded bg-navy-900/60 px-1.5 py-1 text-center">
                    <p className="text-gray-400">{t("common.morale", { defaultValue: "Morale" })}</p>
                    <p className="font-heading font-bold text-accent-400">
                      {morale !== null ? `${morale}%` : "—"}
                    </p>
                  </div>
                </div>
                </div>
              </div>
            );
          })}
        </div>
      </CardBody>
    </Card>
  );
}
