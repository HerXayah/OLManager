import { useTranslation } from "react-i18next";
import { MatchSnapshot, EnginePlayerData } from "./types";
import { Badge } from "../ui";
import { ArrowUpDown, AlertTriangle, Wand2 } from "lucide-react";
import { resolvePlayerPhoto } from "../../lib/playerPhotos";

export type LolRole = "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT";

export const LOL_ROLE_ORDER: LolRole[] = ["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"];

export const ROLE_KEY_STATS: Record<LolRole, { label: string; key: string }[]> = {
  TOP: [
    { label: "MEC", key: "dribbling" },
    { label: "TF", key: "teamwork" },
    { label: "DISC", key: "composure" },
  ],
  JUNGLE: [
    { label: "MAC", key: "vision" },
    { label: "SHOT", key: "leadership" },
    { label: "DISC", key: "composure" },
  ],
  MID: [
    { label: "MEC", key: "dribbling" },
    { label: "LAN", key: "shooting" },
    { label: "MAC", key: "vision" },
  ],
  ADC: [
    { label: "LAN", key: "shooting" },
    { label: "MEC", key: "dribbling" },
    { label: "MENT", key: "stamina" },
  ],
  SUPPORT: [
    { label: "SHOT", key: "leadership" },
    { label: "MAC", key: "vision" },
    { label: "TF", key: "teamwork" },
  ],
};

export function getPlayerLolRole(player: EnginePlayerData): LolRole {
  const explicitRole = String(player.lol_role || "")
    .toUpperCase()
    .replace(/[^A-Z]/g, "");

  if (explicitRole === "TOP") return "TOP";
  if (explicitRole === "JUNGLE" || explicitRole === "JG") return "JUNGLE";
  if (explicitRole === "MID") return "MID";
  if (explicitRole === "ADC") return "ADC";
  if (explicitRole === "SUPPORT" || explicitRole === "SUP") return "SUPPORT";

  const key = String(player.position || "")
    .toLowerCase()
    .replace(/[^a-z]/g, "");

  if (
    key === "defender" ||
    key === "rightback" ||
    key === "leftback" ||
    key === "centerback" ||
    key === "rightwingback" ||
    key === "leftwingback"
  ) {
    return "TOP";
  }
  if (key === "attackingmidfielder" || key === "rightmidfielder" || key === "leftmidfielder") {
    return "MID";
  }
  if (key === "forward" || key === "striker" || key === "rightwinger" || key === "leftwinger") {
    return "ADC";
  }
  if (key === "defensivemidfielder" || key === "goalkeeper") {
    return "SUPPORT";
  }
  return "JUNGLE";
}

export function getPositionOvr(p: EnginePlayerData): number {
  const avg =
    (p.dribbling +
      p.shooting +
      p.teamwork +
      p.vision +
      p.decisions +
      p.leadership +
      p.agility +
      p.composure +
      p.stamina) /
    9;
  return Math.max(1, Math.min(99, Math.round(avg)));
}

export function condColor(c: number): string {
  if (c >= 75) return "text-primary-400";
  if (c >= 50) return "text-amber-400";
  return "text-red-400";
}

export function statColor(v: number): string {
  if (v >= 75) return "text-primary-400 font-bold";
  if (v >= 60) return "text-gray-200";
  return "text-gray-500";
}

export function getStatVal(p: EnginePlayerData, key: string): number {
  return (p as unknown as Record<string, number>)[key] ?? 0;
}

export function parseFormationNeeds(_formation: string): Record<string, number> {
  return {
    TOP: 1,
    JUNGLE: 1,
    MID: 1,
    ADC: 1,
    SUPPORT: 1,
  };
}

interface PreMatchLineupProps {
  userTeam: MatchSnapshot["home_team"];
  userBench: EnginePlayerData[];
  oppTeam: MatchSnapshot["home_team"];
  userColor: string;
  homeTeamColor: string;
  awayTeamColor: string;
  userSide: "Home" | "Away";
  selectedStarterId: string | null;
  isAutoSelecting: boolean;
  onSelectStarter: (id: string | null) => void;
  onSwap: (benchPlayerId: string) => void;
  onAutoSelect: () => void;
}

export default function PreMatchLineup({
  userTeam,
  userBench,
  oppTeam,
  userColor,
  homeTeamColor,
  awayTeamColor,
  userSide,
  selectedStarterId,
  isAutoSelecting,
  onSelectStarter,
  onSwap,
  onAutoSelect,
}: PreMatchLineupProps) {
  const { t } = useTranslation();
  const autoSelectLabel = t("match.autoSelectXI").replace(/XI/g, "5");
  const startersLabel = `${t("match.lineup")} 5`;
  const opponentColor = userSide === "Home" ? awayTeamColor : homeTeamColor;
  const oppOrderedPlayers = [...oppTeam.players].sort((left, right) => {
    const leftIdx = LOL_ROLE_ORDER.indexOf(getPlayerLolRole(left));
    const rightIdx = LOL_ROLE_ORDER.indexOf(getPlayerLolRole(right));
    if (leftIdx !== rightIdx) return leftIdx - rightIdx;
    return right.name.localeCompare(left.name);
  });

  return (
    <div className="flex flex-col gap-4">
      <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-3 flex items-center justify-between transition-colors duration-300">
        <div className="flex items-center gap-4">
          <span className="text-[10px] font-heading uppercase tracking-widest text-gray-700 dark:text-gray-500">
            {t("match.formationFit")}
          </span>
          {LOL_ROLE_ORDER.map((role) => {
            const actual = userTeam.players.filter((p) => getPlayerLolRole(p) === role).length;
            const ok = actual === 1;
            return (
              <div key={role} className="flex items-center gap-1">
                <span className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-400">
                  {role === "JUNGLE" ? "JG" : role}
                </span>
                <span
                  className={`text-sm font-heading font-bold tabular-nums ${ok ? "text-primary-700 dark:text-primary-400" : "text-amber-600 dark:text-amber-400"}`}
                >
                  {actual}/1
                </span>
                {!ok && <AlertTriangle className="w-3 h-3 text-amber-600 dark:text-amber-400" />}
              </div>
            );
          })}
        </div>
        <button
          onClick={onAutoSelect}
          disabled={isAutoSelecting}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all ${
            isAutoSelecting
              ? "bg-gray-200 dark:bg-navy-700 text-gray-600 dark:text-gray-400 cursor-wait"
              : "bg-accent-100 text-accent-700 hover:bg-accent-200 dark:bg-accent-500/20 dark:text-accent-300 dark:hover:bg-accent-500/30"
          }`}
        >
          <Wand2 className="w-3.5 h-3.5" />
          {isAutoSelecting ? t("match.selecting") : autoSelectLabel}
        </button>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
              {startersLabel}
            </h3>
            <div className="flex items-center gap-2">
              {selectedStarterId && (
                <button
                  onClick={() => onSelectStarter(null)}
                  className="text-[10px] text-gray-500 hover:text-gray-800 dark:hover:text-gray-300 font-heading uppercase tracking-wider"
                >
                  {t("match.cancel")}
                </button>
              )}
              <Badge variant="primary" size="sm">
                {t("match.nPlayers", { count: userTeam.players.length })}
              </Badge>
            </div>
          </div>
          {selectedStarterId && (
            <p className="text-[10px] text-accent-400 font-heading uppercase tracking-wider mb-2">
              {t("match.swapPrompt")}
            </p>
          )}

          {LOL_ROLE_ORDER.map((role) => {
            const players = userTeam.players.filter((p) => getPlayerLolRole(p) === role);
            const keyStats = ROLE_KEY_STATS[role] || [];
            return (
              <div key={role} className="mb-3">
                <div className="flex items-center justify-between mb-1">
                  <div className="flex items-center gap-1.5">
                    <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600">
                      {role === "JUNGLE" ? "JG" : role}
                    </p>
                    {players.length !== 1 && (
                      <span className="flex items-center gap-0.5">
                        <AlertTriangle className="w-2.5 h-2.5 text-amber-400" />
                        <span className="text-[9px] font-heading text-amber-400">{players.length}/1</span>
                      </span>
                    )}
                  </div>
                  <div className="flex items-center gap-0">
                    <span className="text-[8px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 w-7 text-center">
                      OVR
                    </span>
                    {keyStats.map((s) => (
                      <span
                        key={s.label}
                        className="text-[8px] font-heading uppercase tracking-widest text-gray-600 w-7 text-center"
                      >
                        {s.label}
                      </span>
                    ))}
                    <span className="text-[8px] font-heading uppercase tracking-widest text-gray-600 w-8 text-right">
                      FIT
                    </span>
                  </div>
                </div>

                {players.length === 0 ? (
                  <div className="py-1.5 px-2 text-[11px] text-gray-500 dark:text-gray-400">
                    {t("match.noBenchAvailable2")}
                  </div>
                ) : (
                  players.map((p) => {
                    const ovr = getPositionOvr(p);
                    const isSelected = selectedStarterId === p.id;
                    return (
                      <button
                        key={p.id}
                        onClick={() => onSelectStarter(isSelected ? null : p.id)}
                        className={`flex items-center gap-2 py-1.5 px-2 rounded w-full text-left transition-all ${
                          isSelected
                            ? "bg-primary-500/20 ring-1 ring-primary-500/50"
                            : "hover:bg-gray-100 dark:hover:bg-navy-700/50"
                        }`}
                      >
                        {resolvePlayerPhoto(p.id, p.name) ? (
                          <img
                            src={resolvePlayerPhoto(p.id, p.name) || ""}
                            alt={p.name}
                            className="w-7 h-7 rounded-full object-cover flex-shrink-0"
                            loading="lazy"
                          />
                        ) : (
                          <div
                            className="w-7 h-7 rounded-full flex items-center justify-center text-[10px] font-heading font-bold flex-shrink-0"
                            style={{
                              backgroundColor: userColor + "30",
                              color: userColor,
                            }}
                          >
                            {p.name.substring(0, 1).toUpperCase()}
                          </div>
                        )}
                        <span className="text-sm text-gray-800 dark:text-gray-200 font-medium flex-1 truncate">
                          {p.name}
                        </span>
                        {isSelected && <ArrowUpDown className="w-3.5 h-3.5 text-primary-400 flex-shrink-0" />}
                        <div className="flex items-center gap-0">
                          <span
                            className={`text-[10px] font-heading font-bold tabular-nums w-7 text-center ${
                              ovr >= 70 ? "text-primary-400" : ovr >= 50 ? "text-gray-300" : "text-red-400"
                            }`}
                          >
                            {ovr}
                          </span>
                          {keyStats.map((s) => (
                            <span
                              key={s.label}
                              className={`text-[10px] font-heading tabular-nums w-7 text-center ${statColor(
                                getStatVal(p, s.key),
                              )}`}
                            >
                              {getStatVal(p, s.key)}
                            </span>
                          ))}
                        </div>
                        <span className={`text-xs tabular-nums w-8 text-right ${condColor(p.condition)}`}>
                          {Math.round(p.condition)}%
                        </span>
                      </button>
                    );
                  })
                )}
              </div>
            );
          })}
        </div>

        <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
              {t("match.substitutes")}
            </h3>
            <Badge variant="neutral" size="sm">
              {t("match.nAvailable", { count: userBench.length })}
            </Badge>
          </div>
          {userBench.length === 0 ? (
            <p className="text-xs text-gray-600 dark:text-gray-500">{t("match.noBenchAvailable2")}</p>
          ) : (
            <div className="flex flex-col gap-1">
              <div className="flex items-center gap-2 px-2 pb-1">
                <span className="w-7" />
                <span className="flex-1" />
                <span className="text-[8px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 w-8 text-center">
                  POS
                </span>
                <span className="text-[8px] font-heading uppercase tracking-widest text-gray-600 w-[84px] text-center">
                  {t("match.keyStats")}
                </span>
                <span className="text-[8px] font-heading uppercase tracking-widest text-gray-600 w-8 text-right">
                  FIT
                </span>
              </div>
              {userBench.map((bp) => {
                const ovr = getPositionOvr(bp);
                const role = getPlayerLolRole(bp);
                const keyStats = ROLE_KEY_STATS[role] || [];
                return (
                  <button
                    key={bp.id}
                    onClick={() => (selectedStarterId ? onSwap(bp.id) : null)}
                    className={`flex items-center gap-2 py-1.5 px-2 rounded w-full text-left transition-all ${
                      selectedStarterId
                        ? "hover:bg-primary-500/20 hover:ring-1 hover:ring-primary-500/50 cursor-pointer"
                        : "hover:bg-gray-100 dark:hover:bg-navy-700/50"
                    }`}
                  >
                    {resolvePlayerPhoto(bp.id, bp.name) ? (
                      <img
                        src={resolvePlayerPhoto(bp.id, bp.name) || ""}
                        alt={bp.name}
                        className="w-7 h-7 rounded-full object-cover flex-shrink-0"
                        loading="lazy"
                      />
                    ) : (
                      <div className="w-7 h-7 rounded-full bg-gray-200 dark:bg-navy-600 flex items-center justify-center text-[10px] font-heading font-bold text-gray-500 dark:text-gray-400 flex-shrink-0 transition-colors duration-300">
                        {bp.name.substring(0, 1).toUpperCase()}
                      </div>
                    )}
                    <span className="text-sm text-gray-700 dark:text-gray-300 font-medium flex-1 truncate">
                      {bp.name}
                    </span>
                    <Badge variant="neutral" size="sm">
                      {role === "JUNGLE" ? "JG" : role}
                    </Badge>
                    <div className="flex items-center gap-0">
                      {keyStats.map((s) => (
                        <span
                          key={s.label}
                          className={`text-[10px] font-heading tabular-nums w-7 text-center ${statColor(
                            getStatVal(bp, s.key),
                          )}`}
                        >
                          {getStatVal(bp, s.key)}
                        </span>
                      ))}
                    </div>
                    <span className={`text-xs tabular-nums w-8 text-right ${condColor(bp.condition)}`}>
                      {Math.round(bp.condition)}%
                    </span>
                  </button>
                );
              })}
            </div>
          )}

          <div className="mt-6 pt-4 border-t border-gray-200 dark:border-navy-700">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
              {t("match.opponent")}
            </h3>
            <div className="flex items-center gap-3 mb-2">
                  <div
                    className="w-10 h-10 rounded-lg flex items-center justify-center font-heading font-bold text-sm"
                    style={{
                      backgroundColor: opponentColor + "30",
                    }}
                  >
                    {oppTeam.name.substring(0, 3).toUpperCase()}
              </div>
              <div>
                <p className="font-heading font-bold text-sm text-gray-800 dark:text-gray-200">{oppTeam.name}</p>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {t("match.lineup")} · {oppTeam.players.length}/5
                </p>
              </div>
            </div>
            <div className="flex flex-col gap-1.5">
              {oppOrderedPlayers.slice(0, 5).map((player) => {
                const role = getPlayerLolRole(player);
                const photo = resolvePlayerPhoto(player.id, player.name);
                return (
                  <div
                    key={player.id}
                    className="flex items-center gap-2 rounded px-2 py-1 bg-gray-100/80 dark:bg-navy-700/40"
                  >
                    {photo ? (
                      <img
                        src={photo}
                        alt={player.name}
                        className="w-6 h-6 rounded-full object-cover"
                        loading="lazy"
                      />
                    ) : (
                      <div className="w-6 h-6 rounded-full bg-gray-300 dark:bg-navy-600 text-[10px] font-heading font-bold flex items-center justify-center text-gray-700 dark:text-gray-300">
                        {player.name.substring(0, 1).toUpperCase()}
                      </div>
                    )}
                    <span className="text-xs text-gray-700 dark:text-gray-300 font-medium flex-1 truncate">
                      {player.name}
                    </span>
                    <Badge variant="neutral" size="sm">
                      {role === "JUNGLE" ? "JG" : role}
                    </Badge>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
