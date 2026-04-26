import { Trophy } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge, Card, CardBody, CardHeader } from "../ui";
import type { LeagueData, TeamData } from "../../store/gameStore";

interface LeagueStandingSnapshot {
  team_id: string;
  played: number;
  won: number;
  drawn: number;
  lost: number;
  goals_for: number;
  goals_against: number;
  points: number;
}

interface HomeLeaguePositionCardProps {
  isPreseason: boolean;
  phase: string;
  seasonStartLabel: string | null;
  league?: LeagueData | null;
  sortedStandings: LeagueStandingSnapshot[];
  teams: TeamData[];
  myTeamId: string | null;
  onNavigate?: (tab: string) => void;
}

function teamLogoUrl(team: TeamData | undefined): string | null {
  if (!team) return null;
  const slug = team.id.replace(/^lec-/, "");

  const aliases: Record<string, string> = {
  };

  if (slug === "shifters") {
    return "https://static.lolesports.com/teams/1765897071435_600px-Shifters_allmode.png";
  }

  const file = aliases[slug] ?? slug;
  return `/team-logos/${file}.png`;
}

function getTeamLabel(teams: TeamData[], teamId: string): string {
  const team = teams.find((item) => item.id === teamId);
  return team?.short_name ?? team?.name ?? teamId;
}

export default function HomeLeaguePositionCard({
  isPreseason,
  phase,
  seasonStartLabel,
  league,
  sortedStandings,
  teams,
  myTeamId,
  onNavigate,
}: HomeLeaguePositionCardProps) {
  const { t } = useTranslation();
  const hasPlayoffsStarted = Boolean(
    league?.fixtures.some((fixture) => fixture.competition === "Playoffs"),
  );
  const playoffFixtures = league?.fixtures.filter(
    (fixture) => fixture.competition === "Playoffs",
  ) ?? [];
  const myPlayoffFixtures = myTeamId
    ? playoffFixtures.filter(
        (fixture) =>
          fixture.home_team_id === myTeamId || fixture.away_team_id === myTeamId,
      )
    : [];
  myPlayoffFixtures.sort((left, right) => {
    if (left.matchday !== right.matchday) {
      return left.matchday - right.matchday;
    }
    return left.date.localeCompare(right.date);
  });
  const nextPlayoffFixture = myPlayoffFixtures.find(
    (fixture) => fixture.status !== "Completed",
  );
  const latestCompletedPlayoffFixture = [...myPlayoffFixtures]
    .reverse()
    .find((fixture) => fixture.status === "Completed");
  const spotlightFixture = nextPlayoffFixture ?? latestCompletedPlayoffFixture ?? null;

  return (
    <Card accent="accent">
      <CardHeader
        action={
          <button
            onClick={() => onNavigate?.("Schedule")}
            className="text-primary-500 dark:text-primary-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-primary-600 dark:hover:text-primary-300 transition-colors"
          >
            {hasPlayoffsStarted ? t("schedule.playoffs") : t("home.standings")}
          </button>
        }
      >
        {hasPlayoffsStarted ? t("schedule.playoffs") : t("home.leaguePosition")}
      </CardHeader>

      <CardBody>
        {isPreseason ? (
          <div className="flex flex-col items-center gap-2 py-4 text-center">
            <Badge variant="accent" size="sm">
              {t(`season.phases.${phase}`)}
            </Badge>
            <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
              {seasonStartLabel
                ? t("season.startsOn", { date: seasonStartLabel })
                : t("season.noOpener")}
            </p>
            <p className="text-xs text-gray-500 dark:text-gray-400 max-w-xs">
              {t("season.standingsLocked")}
            </p>
          </div>
        ) : hasPlayoffsStarted ? (
          <div className="rounded-xl border border-cyan-300/20 bg-navy-900/40 px-3 py-3">
            {spotlightFixture ? (
              <>
                <div className="flex items-center justify-between gap-2">
                  <Badge variant="accent" size="sm">
                    {nextPlayoffFixture
                      ? t("home.nextMatch", { defaultValue: "Next Match" })
                      : t("schedule.lastResult", { defaultValue: "Last Result" })}
                  </Badge>
                  <span className="text-[10px] font-heading font-bold uppercase tracking-wider text-cyan-300/90">
                    {t("schedule.round", {
                      number: spotlightFixture.matchday,
                      defaultValue: `Round ${spotlightFixture.matchday}`,
                    })}
                    {` · BO${spotlightFixture.best_of ?? 3}`}
                  </span>
                </div>

                <p className="mt-2 text-sm font-heading font-bold text-gray-100 uppercase tracking-wide">
                  {getTeamLabel(teams, spotlightFixture.home_team_id)} vs {getTeamLabel(teams, spotlightFixture.away_team_id)}
                </p>

                {spotlightFixture.result ? (
                  <p className="mt-1 text-xs text-gray-300 font-heading uppercase tracking-wider">
                    {(spotlightFixture.result.home_wins ?? spotlightFixture.result.home_goals ?? 0)} - {(spotlightFixture.result.away_wins ?? spotlightFixture.result.away_goals ?? 0)}
                  </p>
                ) : (
                  <p className="mt-1 text-xs text-gray-400 font-heading uppercase tracking-wider">
                    {spotlightFixture.date}
                  </p>
                )}
              </>
            ) : (
              <p className="text-xs text-gray-400 dark:text-gray-500">
                {t("home.noUpcomingOpponent", {
                  defaultValue: "No upcoming playoff match.",
                })}
              </p>
            )}
          </div>
        ) : sortedStandings.length > 0 ? (
          <div className="space-y-1">
            {sortedStandings.slice(0, 10).map((entry, index) => {
              const team = teams.find((item) => item.id === entry.team_id);
              const short = team?.short_name ?? team?.name ?? entry.team_id;
              const wr =
                entry.won + entry.lost > 0
                  ? Math.round((entry.won / (entry.won + entry.lost)) * 100)
                  : 0;
              const isMine = myTeamId === entry.team_id;
              const logo = teamLogoUrl(team);

              return (
                <div
                  key={entry.team_id}
                  className={`grid grid-cols-[18px_1fr_24px_24px_44px] items-center gap-2 rounded px-2 py-1 text-[11px] ${isMine ? "bg-cyan-500/10 border border-cyan-400/30" : "bg-gray-50 dark:bg-navy-800/40"}`}
                >
                  <span className={`font-heading font-black ${isMine ? "text-cyan-300" : "text-gray-500 dark:text-gray-400"}`}>
                    {index + 1}
                  </span>

                  <div className="flex items-center gap-2 min-w-0">
                    {logo ? (
                      <img
                        src={logo}
                        alt={short}
                        className="w-4 h-4 object-contain"
                        loading="lazy"
                      />
                    ) : null}
                    <span className={`truncate font-heading font-bold ${isMine ? "text-cyan-200" : "text-gray-800 dark:text-gray-100"}`}>
                      {short}
                    </span>
                  </div>

                  <span className="text-center font-heading font-bold text-gray-500 dark:text-gray-300">
                    {entry.won}
                  </span>
                  <span className="text-center font-heading font-bold text-gray-500 dark:text-gray-300">
                    {entry.lost}
                  </span>
                  <span className={`text-right font-heading font-black ${wr >= 50 ? "text-green-500" : "text-red-500"}`}>
                    {wr}%
                  </span>
                </div>
              );
            })}
          </div>
        ) : (
          <div className="flex flex-col items-center gap-2 py-4">
            <Trophy className="w-8 h-8 text-gray-300 dark:text-navy-600" />
            <p className="text-xs text-gray-500 dark:text-gray-400">
              {t("home.noLeague")}
            </p>
          </div>
        )}
      </CardBody>
    </Card>
  );
}
