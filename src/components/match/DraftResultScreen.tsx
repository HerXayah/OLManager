import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import teamsSeed from "../../../data/lec/draft/teams.json";
import type { MatchSnapshot } from "./types";
import type { DraftMatchResult, DraftTimelineEvent } from "./draftResultSimulator";

type Side = "blue" | "red";

interface TeamSeed {
  id: string;
  name: string;
  shortName: string;
}

interface DraftResultScreenProps {
  snapshot: MatchSnapshot;
  controlledSide: Side;
  result: DraftMatchResult;
  seriesGames?: DraftResultSeriesGame[];
  seriesLength?: 1 | 3 | 5;
  seriesGameIndex?: number;
  userSeriesWins?: number;
  opponentSeriesWins?: number;
  canUserChooseSide?: boolean;
  onPressConference?: () => void;
  onContinue: (nextUserSide?: Side) => void;
}

export interface DraftResultSeriesGame {
  gameIndex: number;
  result: DraftMatchResult;
  winnerSide?: Side;
}

const TEAM_SEEDS: TeamSeed[] = ((teamsSeed as { data?: { teams?: TeamSeed[] } }).data?.teams ?? []) as TeamSeed[];

const TEAM_BRAND_MAP: Record<string, { tricode: string }> = {
  "g2 esports": { tricode: "G2" },
  fnatic: { tricode: "FNC" },
  "team vitality": { tricode: "VIT" },
  vitality: { tricode: "VIT" },
  "team heretics": { tricode: "HRTS" },
  "sk gaming": { tricode: "SK" },
  "movistar koi": { tricode: "MKOI" },
  "mad lions koi": { tricode: "MKOI" },
  "team bds": { tricode: "BDS" },
  giantx: { tricode: "GX" },
  heretics: { tricode: "HRTS" },
  "natus vincere": { tricode: "NAVI" },
  "karmine corp": { tricode: "KC" },
};

function normalizeKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]/g, "");
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

function playerPhotoUrl(playerId: string): string | null {
  const match = playerId.match(/^lec-player-(.+)$/);
  if (!match) return null;
  return `/player-photos/${match[1]}.png`;
}

function sideTeam(snapshot: MatchSnapshot, side: Side) {
  return side === "blue" ? snapshot.home_team : snapshot.away_team;
}

function eventPillClass(event: DraftTimelineEvent): string {
  const sideBase =
    event.side === "blue"
      ? "border-cyan-400/60 bg-cyan-500/10 text-cyan-200"
      : "border-orange-400/60 bg-orange-500/10 text-orange-200";

  if (event.type === "baron") {
    return `${sideBase} shadow-[0_0_0_1px_rgba(168,85,247,0.35),0_0_14px_rgba(168,85,247,0.45)]`;
  }

  if (event.type === "dragon_soul" || event.type === "elder") {
    return `${sideBase} shadow-[0_0_0_1px_rgba(250,204,21,0.35),0_0_14px_rgba(250,204,21,0.45)]`;
  }

  return sideBase;
}

export default function DraftResultScreen({
  snapshot,
  controlledSide,
  result,
  seriesGames,
  seriesLength = 1,
  seriesGameIndex = 1,
  userSeriesWins = 0,
  opponentSeriesWins = 0,
  canUserChooseSide = false,
  onPressConference,
  onContinue,
}: DraftResultScreenProps) {
  const { t } = useTranslation();

  const seriesGamesForTabs = useMemo<DraftResultSeriesGame[]>(() => {
    if (!Array.isArray(seriesGames) || seriesGames.length === 0) {
      return [{ gameIndex: Math.max(1, seriesGameIndex), result, winnerSide: result.winnerSide }];
    }

    return [...seriesGames]
      .filter((entry) => Number.isFinite(entry.gameIndex) && entry.gameIndex >= 1)
      .sort((left, right) => left.gameIndex - right.gameIndex);
  }, [result, seriesGameIndex, seriesGames]);

  const latestSeriesGame = seriesGamesForTabs[seriesGamesForTabs.length - 1];
  const [selectedGameIndex, setSelectedGameIndex] = useState<number>(latestSeriesGame?.gameIndex ?? 1);

  useEffect(() => {
    const hasSelectedGame = seriesGamesForTabs.some((entry) => entry.gameIndex === selectedGameIndex);
    if (!hasSelectedGame) {
      setSelectedGameIndex(latestSeriesGame?.gameIndex ?? 1);
    }
  }, [latestSeriesGame?.gameIndex, selectedGameIndex, seriesGamesForTabs]);

  const selectedSeriesGame =
    seriesGamesForTabs.find((entry) => entry.gameIndex === selectedGameIndex) ?? latestSeriesGame;
  const selectedResult = selectedSeriesGame?.result ?? result;

  const blueTeam = sideTeam(snapshot, "blue");
  const redTeam = sideTeam(snapshot, "red");
  const blueTri = teamTriCode(blueTeam.name);
  const redTri = teamTriCode(redTeam.name);

  const controlledWon = selectedResult.winnerSide === controlledSide;
  const title = controlledWon
    ? t("match.victory", "Victory")
    : t("match.defeat", "Defeat");

  const blueRows = selectedResult.playerResults.filter((row) => row.side === "blue");
  const redRows = selectedResult.playerResults.filter((row) => row.side === "red");

  const maxAbsGold =
    Math.max(...selectedResult.goldDiffTimeline.map((point) => Math.abs(point.diff)), 1000) ||
    1000;

  const points = selectedResult.goldDiffTimeline
    .map((point, idx) => {
      const x = (idx / Math.max(1, selectedResult.goldDiffTimeline.length - 1)) * 100;
      const y = 50 - (point.diff / maxAbsGold) * 45;
      return `${x},${y}`;
    })
    .join(" ");

  const mvpPhoto = playerPhotoUrl(selectedResult.mvp.playerId);
  const nextGameLabel = `${Math.min(seriesLength, seriesGameIndex + 1)}/${seriesLength}`;
  const targetSeriesWins = seriesLength === 1 ? 1 : seriesLength === 3 ? 2 : 3;
  const isSeriesFinished =
    seriesLength === 1 ||
    userSeriesWins >= targetSeriesWins ||
    opponentSeriesWins >= targetSeriesWins;

  return (
    <div className="min-h-screen bg-[#050608] text-white p-4 md:p-6">
      <div className="max-w-[1200px] mx-auto space-y-4">
        <header className="rounded-xl border border-cyan-400/25 bg-[#0a1433] p-5 text-center shadow-[0_0_24px_rgba(0,242,255,0.1)]">
          <p className="text-xs uppercase tracking-[0.3em] text-gray-400">{t("match.matchOver", "Match Over")}</p>
          <h1 className={`mt-1 text-4xl font-heading uppercase ${controlledWon ? "text-green-400" : "text-red-400"}`}>
            {title}
          </h1>

          <div className="mt-3 flex items-center justify-center gap-4 text-3xl font-black">
            <span className="text-cyan-300">{blueTri}</span>
            <span>{selectedResult.blueKills}</span>
            <span className="text-gray-500">-</span>
            <span>{selectedResult.redKills}</span>
            <span className="text-orange-300">{redTri}</span>
          </div>

          <p className="mt-2 text-sm text-gray-300">
            MVP: <span className="font-bold text-cyan-300">{selectedResult.mvp.playerName}</span>
          </p>
          {seriesGamesForTabs.length > 1 ? (
            <div className="mt-3 flex flex-wrap items-center justify-center gap-2">
              {seriesGamesForTabs.map((entry) => {
                const isSelected = entry.gameIndex === selectedGameIndex;
                return (
                  <button
                    key={`game-tab-${entry.gameIndex}`}
                    type="button"
                    className={`rounded-md border px-3 py-1 text-xs font-heading font-bold uppercase tracking-wide ${isSelected ? "border-cyan-300 bg-cyan-500/20 text-cyan-100" : "border-white/20 bg-white/5 text-gray-300 hover:bg-white/10"}`}
                    onClick={() => setSelectedGameIndex(entry.gameIndex)}
                  >
                    {t("match.game", { defaultValue: "Game" })} {entry.gameIndex}
                  </button>
                );
              })}
            </div>
          ) : null}
          {seriesLength > 1 ? (
            <p className="mt-1 text-xs text-gray-400">
              Serie ({seriesLength === 3 ? "Bo3" : "Bo5"}) · {userSeriesWins} - {opponentSeriesWins}
            </p>
          ) : null}
        </header>

        <section className="grid grid-cols-1 xl:grid-cols-[320px_minmax(0,1fr)] gap-4">
          <aside className="space-y-4">
            <div className="rounded-xl border border-yellow-400/25 bg-[#0a1433] p-4">
              <p className="text-[11px] uppercase tracking-[0.2em] text-yellow-300">Mejor del partido</p>
              <div className="mt-3 flex items-center gap-3">
                {mvpPhoto ? (
                  <img
                    src={mvpPhoto}
                    alt={selectedResult.mvp.playerName}
                    className="w-14 h-14 rounded-full object-cover border border-white/15"
                    loading="lazy"
                  />
                ) : (
                  <div className="w-14 h-14 rounded-full bg-[#0b1226] border border-white/15 grid place-items-center text-xl font-black">
                    {selectedResult.mvp.playerName.charAt(0).toUpperCase()}
                  </div>
                )}
                <div>
                  <p className="font-bold text-lg">{selectedResult.mvp.playerName}</p>
                  <p className="text-sm text-gray-300">
                    {selectedResult.mvp.kills}/{selectedResult.mvp.deaths}/{selectedResult.mvp.assists}
                  </p>
                </div>
              </div>
            </div>

            <div className="rounded-xl border border-cyan-400/25 bg-[#0a1433] p-4">
              <p className="text-[11px] uppercase tracking-[0.2em] text-cyan-200">Gold advantage</p>
              <div className="mt-3 rounded-md bg-[#081028] border border-white/10 p-2">
                <svg viewBox="0 0 100 100" className="w-full h-36">
                  <line x1="0" y1="50" x2="100" y2="50" stroke="rgba(255,255,255,0.2)" strokeDasharray="2 2" />
                  <polyline
                    fill="none"
                    stroke="#22d3ee"
                    strokeWidth="2"
                    points={points}
                  />
                </svg>
              </div>
              <p className="text-xs text-gray-400 mt-2">Duración: {selectedResult.durationMinutes}m</p>
            </div>
          </aside>

          <div className="rounded-xl border border-cyan-400/25 bg-[#0a1433] p-4">
            <p className="text-[11px] uppercase tracking-[0.2em] text-cyan-200 mb-3">Performance</p>

            <div className="space-y-1">
              <p className="text-sm font-bold text-cyan-300">{blueTri}</p>
              {blueRows.map((row) => {
                const icon = playerPhotoUrl(row.playerId);
                const isMvp = row.playerId === selectedResult.mvp.playerId;
                return (
                  <div
                    key={`blue-${row.playerId}-${row.role}`}
                    className={`grid grid-cols-[1fr_auto_auto_auto] items-center gap-3 rounded-md border px-3 py-2 ${isMvp ? "border-yellow-400/50 bg-yellow-500/10" : "border-white/10 bg-white/5"}`}
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      {icon ? <img src={icon} alt={row.playerName} className="w-7 h-7 rounded-full object-cover border border-white/15" loading="lazy" /> : null}
                      <span className="truncate">{row.playerName}</span>
                    </div>
                    <span className="text-sm text-gray-300">{row.kills}/{row.deaths}/{row.assists}</span>
                    <span className="text-sm text-gray-300">{row.gold}</span>
                    <span className="text-sm font-bold text-cyan-300">{row.rating.toFixed(1)}</span>
                  </div>
                );
              })}
            </div>

            <div className="space-y-1 mt-4">
              <p className="text-sm font-bold text-orange-300">{redTri}</p>
              {redRows.map((row) => {
                const icon = playerPhotoUrl(row.playerId);
                const isMvp = row.playerId === selectedResult.mvp.playerId;
                return (
                  <div
                    key={`red-${row.playerId}-${row.role}`}
                    className={`grid grid-cols-[1fr_auto_auto_auto] items-center gap-3 rounded-md border px-3 py-2 ${isMvp ? "border-yellow-400/50 bg-yellow-500/10" : "border-white/10 bg-white/5"}`}
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      {icon ? <img src={icon} alt={row.playerName} className="w-7 h-7 rounded-full object-cover border border-white/15" loading="lazy" /> : null}
                      <span className="truncate">{row.playerName}</span>
                    </div>
                    <span className="text-sm text-gray-300">{row.kills}/{row.deaths}/{row.assists}</span>
                    <span className="text-sm text-gray-300">{row.gold}</span>
                    <span className="text-sm font-bold text-orange-300">{row.rating.toFixed(1)}</span>
                  </div>
                );
              })}
            </div>
          </div>
        </section>

        <section className="rounded-xl border border-cyan-400/25 bg-[#0a1433] p-4">
          <p className="text-[11px] uppercase tracking-[0.2em] text-cyan-200 mb-3">Game Timeline</p>
          <div className="space-y-2">
            <div className="h-px bg-white/10" />
            <div className="flex flex-wrap gap-2">
              {selectedResult.timelineEvents.map((event, idx) => (
                <span
                  key={`${event.type}-${event.minute}-${idx}`}
                  className={`inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs ${eventPillClass(event)}`}
                >
                  <span className="font-semibold">{event.minute}m</span>
                  <span>{event.label}</span>
                </span>
              ))}
            </div>
          </div>
        </section>

        <div className="flex justify-end">
          <div className="flex items-center gap-2">
            {onPressConference ? (
              <button
                className="rounded-md border border-white/20 bg-white/5 hover:bg-white/10 text-white font-heading font-bold uppercase tracking-wide px-4 py-2"
                onClick={onPressConference}
              >
                {t("match.pressConference", { defaultValue: "Press Conference" })}
              </button>
            ) : null}

            {canUserChooseSide ? (
              <div className="flex items-center gap-1 rounded-md border border-white/15 bg-[#081028] px-1 py-1">
                <button
                  className={`rounded px-3 py-1 text-xs font-heading font-bold uppercase ${controlledSide === "blue" ? "bg-cyan-500/20 text-cyan-200" : "text-gray-300"}`}
                  onClick={() => onContinue("blue")}
                >
                  Blue next
                </button>
                <button
                  className={`rounded px-3 py-1 text-xs font-heading font-bold uppercase ${controlledSide === "red" ? "bg-orange-500/20 text-orange-200" : "text-gray-300"}`}
                  onClick={() => onContinue("red")}
                >
                  Red next
                </button>
              </div>
            ) : null}

            <button
              className="rounded-md bg-orange-500 hover:bg-orange-400 text-navy-900 font-heading font-bold uppercase tracking-wide px-6 py-2"
              onClick={() => onContinue()}
            >
              {seriesLength > 1 && !isSeriesFinished
                ? `${t("match.game", { defaultValue: "Game" })} ${nextGameLabel}`
                : t("match.continue", { defaultValue: "Continue" })}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
