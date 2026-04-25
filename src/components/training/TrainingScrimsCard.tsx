import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { CalendarDays, Shuffle, Swords } from "lucide-react";

import type { GameStateData } from "../../store/gameStore";
import { setWeeklyScrims } from "../../services/trainingService";
import { calculateLolOvr } from "../../lib/lolPlayerStats";
import { Card, CardBody, CardHeader, Select } from "../ui";

interface TrainingScrimsCardProps {
  gameState: GameStateData;
  onGameUpdate?: (state: GameStateData) => void;
  isSaving: boolean;
  setIsSaving: (value: boolean) => void;
  currentSchedule: string;
}

const SCRIMS_PER_WEEK: Record<string, number> = {
  Intense: 6,
  Balanced: 4,
  Light: 2,
};

const SLOT_WEEKDAYS: Record<string, number[]> = {
  Intense: [1, 1, 2, 2, 3, 3],
  Balanced: [1, 2, 2, 3],
  Light: [1, 3],
};

const WEEKDAY_LABELS = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"] as const;

function getWeekdayFromDate(dateStr: string): number {
  const date = new Date(dateStr);
  return (date.getUTCDay() + 6) % 7;
}

function isoWeekKey(dateStr: string): string {
  const date = new Date(dateStr);
  if (!Number.isFinite(date.getTime())) return "unknown";
  const utc = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()));
  const weekday = utc.getUTCDay() || 7;
  utc.setUTCDate(utc.getUTCDate() + 4 - weekday);
  const yearStart = new Date(Date.UTC(utc.getUTCFullYear(), 0, 1));
  const weekNo = Math.ceil((((utc.getTime() - yearStart.getTime()) / 86400000) + 1) / 7);
  return `${utc.getUTCFullYear()}-W${weekNo}`;
}

function teamLogoPath(teamId: string): string {
  return `/team-logos/${teamId.replace(/^lec-/, "")}.png`;
}

export default function TrainingScrimsCard({
  gameState,
  onGameUpdate,
  isSaving,
  setIsSaving,
  currentSchedule,
}: TrainingScrimsCardProps) {
  const { t } = useTranslation();

  const myTeam = gameState.teams.find((team) => team.id === gameState.manager.team_id);
  if (!myTeam) return null;

  const slots = SCRIMS_PER_WEEK[currentSchedule] ?? 2;
  const selected = Array.from({ length: slots }, (_, idx) => myTeam.weekly_scrim_opponent_ids?.[idx] ?? "");
  const slotDays = SLOT_WEEKDAYS[currentSchedule] ?? SLOT_WEEKDAYS.Balanced;
  const currentWeekday = getWeekdayFromDate(gameState.clock.current_date);
  const weekKey = isoWeekKey(gameState.clock.current_date);

  const teamOvrById = useMemo(() => {
    const map = new Map<string, number>();
    gameState.teams.forEach((team) => {
      const starters = (team.starting_xi_ids ?? [])
        .map((playerId) => gameState.players.find((player) => player.id === playerId))
        .filter((player): player is NonNullable<typeof player> => Boolean(player))
        .slice(0, 5);

      const baseRoster = gameState.players
        .filter((player) => player.team_id === team.id)
        .sort((a, b) => calculateLolOvr(b) - calculateLolOvr(a))
        .slice(0, 5);

      const sample = starters.length >= 5 ? starters : baseRoster;
      if (sample.length === 0) {
        map.set(team.id, 74);
        return;
      }

      const avg = sample.reduce((sum, player) => sum + calculateLolOvr(player), 0) / sample.length;
      map.set(team.id, Math.round(avg));
    });
    return map;
  }, [gameState.players, gameState.teams]);

  const options = useMemo(
    () => gameState.teams
      .filter((team) => team.id !== myTeam.id)
      .sort((a, b) => (teamOvrById.get(b.id) ?? 0) - (teamOvrById.get(a.id) ?? 0) || a.name.localeCompare(b.name)),
    [gameState.teams, myTeam.id, teamOvrById],
  );

  const saveWeeklyScrims = async (next: string[]) => {
    setIsSaving(true);
    try {
      const updated = await setWeeklyScrims(next);
      onGameUpdate?.(updated);
    } catch (error) {
      console.error("Failed to save weekly scrims:", error);
    } finally {
      setIsSaving(false);
    }
  };

  const setSlot = (index: number, teamId: string) => {
    const next = Array.from({ length: slots }, (_, slotIndex) => selected[slotIndex] ?? "");
    next[index] = teamId;
    void saveWeeklyScrims(next);
  };

  const streak = myTeam.scrim_loss_streak ?? 0;
  const usingAutoRandom = selected.every((entry) => !entry);

  const resultBySlot = useMemo(() => {
    const map = new Map<number, { won: boolean; opponentTeamId: string }>();
    (myTeam.scrim_slot_results ?? []).forEach((entry) => {
      if (entry.week_key !== weekKey) return;
      map.set(entry.slot_index, {
        won: entry.won,
        opponentTeamId: entry.opponent_team_id,
      });
    });
    return map;
  }, [myTeam.scrim_slot_results, weekKey]);

  const teamNameById = useMemo(
    () => new Map(gameState.teams.map((team) => [team.id, team.name])),
    [gameState.teams],
  );

  return (
    <Card>
      <CardHeader>
        <span className="inline-flex items-center gap-2">
          <Swords className="w-4 h-4 text-amber-400" />
          {t("training.scrims.title", "Scrims semanales")}
        </span>
      </CardHeader>
      <CardBody>
        <p className="text-sm text-gray-500 dark:text-gray-400 mb-3 leading-relaxed">
          {t(
            "training.scrims.description",
            "Planificá scrims por día. El rendimiento rival se calcula con los 5 activos del roster; rivales más fuertes aceleran el crecimiento en Scrims.",
          )}
        </p>

        <div className="mb-3 flex flex-wrap items-center gap-2">
          <span className="rounded-full border border-gray-300 dark:border-navy-600 px-2.5 py-1 text-[11px] font-heading uppercase tracking-wide text-gray-600 dark:text-gray-300">
            {t("training.scrims.weekCapacity", "Capacidad semanal")}: {slots}
          </span>
          <span className="rounded-full border border-gray-300 dark:border-navy-600 px-2.5 py-1 text-[11px] font-heading uppercase tracking-wide text-gray-600 dark:text-gray-300">
            {t("training.scrims.lossStreak", "Racha")}: {streak}
          </span>
          {usingAutoRandom ? (
            <span className="inline-flex items-center gap-1 rounded-full border border-amber-400/50 bg-amber-500/10 px-2.5 py-1 text-[11px] font-heading uppercase tracking-wide text-amber-300">
              <Shuffle className="h-3.5 w-3.5" />
              {t("training.scrims.autoRandom", "Auto-random activo")}
            </span>
          ) : null}
        </div>

        <div className="space-y-2 rounded-lg border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-900/40 p-2.5">
          {Array.from({ length: slots }).map((_, index) => (
            <div key={`scrim-slot-${index}`} className="grid grid-cols-[94px_1fr_auto] items-center gap-2">
              <div className="text-[11px] font-heading uppercase tracking-wide text-gray-500 dark:text-gray-400">
                <span className="inline-flex items-center gap-1">
                  <CalendarDays className="h-3.5 w-3.5" />
                  {(() => {
                    const day = slotDays[index] ?? 0;
                    const previousSameDay = slotDays.slice(0, index).filter((candidate) => candidate === day).length;
                    const totalSameDay = slotDays.filter((candidate) => candidate === day).length;
                    const suffix = totalSameDay > 1 ? ` ${String.fromCharCode(65 + previousSameDay)}` : "";
                    return `${WEEKDAY_LABELS[day]}${suffix}`;
                  })()}
                </span>
              </div>
              <Select
                value={selected[index] ?? ""}
                onChange={(event) => setSlot(index, event.target.value)}
                disabled={isSaving || (slotDays[index] ?? 0) < currentWeekday || resultBySlot.has(index)}
                variant="muted"
                selectSize="sm"
                fullWidth
              >
                <option value="">{t("training.scrims.noOpponent", "Aleatorio")}</option>
                {options.map((team) => (
                  <option key={team.id} value={team.id}>
                    {team.name} · OVR {teamOvrById.get(team.id) ?? 74}
                  </option>
                ))}
              </Select>
              <div className="flex items-center gap-2 min-w-[82px] justify-end">
                {(selected[index] || resultBySlot.get(index)?.opponentTeamId) ? (
                  <img
                    src={teamLogoPath(selected[index] || resultBySlot.get(index)?.opponentTeamId || "")}
                    alt="logo"
                    className="h-6 w-6 rounded-md object-contain bg-black/20 p-0.5"
                    loading="lazy"
                    onError={(event) => {
                      event.currentTarget.style.display = "none";
                    }}
                  />
                ) : null}
                {resultBySlot.has(index) ? (
                  <span
                    className={`rounded-full px-2 py-0.5 text-[10px] font-heading uppercase tracking-wide ${resultBySlot.get(index)?.won ? "bg-emerald-500/15 text-emerald-300 border border-emerald-400/40" : "bg-rose-500/15 text-rose-300 border border-rose-400/40"}`}
                  >
                    {resultBySlot.get(index)?.won ? "W" : "L"}
                  </span>
                ) : (
                  <span className="text-[10px] text-gray-400">—</span>
                )}
              </div>
              {!selected[index] && resultBySlot.get(index)?.opponentTeamId ? (
                <div className="col-start-2 text-[10px] text-amber-300">
                  {t("training.scrims.randomResolved", "Aleatorio → {{team}}", {
                    team: teamNameById.get(resultBySlot.get(index)?.opponentTeamId ?? "")
                      ?? resultBySlot.get(index)?.opponentTeamId,
                  })}
                </div>
              ) : null}
            </div>
          ))}
        </div>
      </CardBody>
    </Card>
  );
}
