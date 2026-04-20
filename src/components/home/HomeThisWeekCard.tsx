import { useTranslation } from "react-i18next";

import type { GameStateData } from "../../store/gameStore";
import { Card, CardBody, CardHeader } from "../ui";

interface HomeThisWeekCardProps {
  gameState: GameStateData;
}

function toDateKey(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function extractDateKey(value: string): string {
  return String(value).slice(0, 10);
}

function parseYmdAsLocalDate(value: string): Date {
  const [y, m, d] = value.split("-").map(Number);
  return new Date(y, (m || 1) - 1, d || 1);
}

function weekdayLabel(date: Date, lang: string): string {
  const raw = new Intl.DateTimeFormat(lang, { weekday: "short" }).format(date);
  return raw.replace(".", "").toUpperCase();
}

function startOfWeek(date: Date): Date {
  const copy = new Date(date);
  const day = (copy.getDay() + 6) % 7;
  copy.setDate(copy.getDate() - day);
  copy.setHours(0, 0, 0, 0);
  return copy;
}

export default function HomeThisWeekCard({ gameState }: HomeThisWeekCardProps) {
  const { t, i18n } = useTranslation();

  const league = gameState.league;
  const teamId = gameState.manager.team_id;

  const currentDate = parseYmdAsLocalDate(
    extractDateKey(gameState.clock.current_date),
  );
  const weekStart = startOfWeek(currentDate);
  const currentDateKey = toDateKey(currentDate);

  const weekDays = Array.from({ length: 7 }, (_, index) => {
    const date = new Date(weekStart);
    date.setDate(weekStart.getDate() + index);

    let fixture = null;
    if (league && teamId) {
      fixture = league.fixtures.find((item) => {
        if (item.home_team_id !== teamId && item.away_team_id !== teamId) return false;
        return extractDateKey(item.date) === toDateKey(date);
      }) ?? null;
    }

    return {
      date,
      label: weekdayLabel(date, i18n.language),
      isToday: toDateKey(date) === currentDateKey,
      fixture,
    };
  });

  return (
    <Card>
      <CardHeader>{t("home.schedule", { defaultValue: "Cronograma" })}</CardHeader>
      <CardBody>
        <div className="grid grid-cols-7 gap-1.5">
          {weekDays.map((day) => {
            const isMatchDay = !!day.fixture;
            return (
              <div
                key={`${day.label}-${day.date.toISOString()}`}
                className={`rounded-md border px-1.5 py-2 text-center ${day.isToday ? "border-accent-400/70 bg-accent-500/10" : "border-gray-100 dark:border-navy-600 bg-gray-50 dark:bg-navy-800/40"}`}
              >
                <p className="text-[10px] font-heading font-bold text-gray-500 dark:text-gray-400">
                  {day.label}
                </p>
                <p className="text-xs font-heading font-bold text-gray-800 dark:text-gray-100 mt-1">
                  {day.date.getDate()}
                </p>
                <p
                  className={`text-[10px] mt-2 font-heading font-bold ${isMatchDay ? "text-primary-500" : "text-gray-400 dark:text-gray-500"}`}
                >
                  {isMatchDay
                    ? t("home.matchShort", { defaultValue: "Partido" })
                    : t("home.restShort", { defaultValue: "Descanso" })}
                </p>
                {isMatchDay ? (
                  <p className="text-[10px] text-gray-500 dark:text-gray-400 truncate mt-1">
                    {day.fixture?.competition === "League"
                      ? t("home.leagueShort", { defaultValue: "League" })
                      : t("home.otherShort", { defaultValue: "Other" })}
                  </p>
                ) : null}
              </div>
            );
          })}
        </div>
      </CardBody>
    </Card>
  );
}
