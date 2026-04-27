import { useState } from "react";
import { GameStateData, FixtureData } from "../../store/gameStore";
import { Card, CardHeader, CardBody, Badge } from "../ui";
import {
  Trophy,
  Calendar,
  TableProperties,
} from "lucide-react";
import {
  getTeamName,
  formatMatchDate,
} from "../../lib/helpers";
import { resolveSeasonContext } from "../../lib/seasonContext";
import { useTranslation } from "react-i18next";
import PlayoffBracketBoard from "../playoffs/PlayoffBracketBoard";

interface TournamentsTabProps {
  gameState: GameStateData;
  onSelectTeam: (id: string) => void;
}

export default function TournamentsTab({
  gameState,
  onSelectTeam,
}: TournamentsTabProps) {
  const { t } = useTranslation();
  const league = gameState.league;
  const userTeamId = gameState.manager.team_id;
  const seasonContext = resolveSeasonContext(gameState);
  const isPreseason = seasonContext.phase === "Preseason";
  const [view, setView] = useState<"overview" | "fixtures" | "standings">(
    "overview",
  );

  if (!league) {
    return (
      <div className="max-w-4xl mx-auto text-center py-12">
        <Trophy className="w-12 h-12 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
        <p className="text-gray-500 dark:text-gray-400 text-sm">
          {t("tournaments.noActive")}
        </p>
      </div>
    );
  }

  const standings = [...league.standings].sort(
    (a, b) =>
      b.points - a.points ||
      b.goals_for - b.goals_against - (a.goals_for - a.goals_against) ||
      b.goals_for - a.goals_for,
  );

  const playoffFixtures = league.fixtures.filter((fixture) => fixture.competition === "Playoffs");
  const hasPlayoffsStarted = playoffFixtures.length > 0;
  const tournamentFixtures = league.fixtures.filter(
    (fixture) => fixture.competition === "League" || fixture.competition === "Playoffs",
  );

  const matchdays = new Map<number, FixtureData[]>();
  tournamentFixtures.forEach((f) => {
    const list = matchdays.get(f.matchday) || [];
    list.push(f);
    matchdays.set(f.matchday, list);
  });
  const sortedMatchdays = Array.from(matchdays.entries()).sort(
    (a, b) => a[0] - b[0],
  );

  const completedMatchdays = sortedMatchdays.filter(([, fixtures]) =>
    fixtures.every((f) => f.status === "Completed"),
  ).length;
  const totalMatchdays = sortedMatchdays.length;
  const userStanding = standings.find((entry) => entry.team_id === userTeamId);
  const userWins = userStanding?.won ?? 0;
  const completedMatches = tournamentFixtures.filter(
    (f) => f.status === "Completed",
  ).length;

  return (
    <div className="max-w-6xl mx-auto">
      {isPreseason && (
        <Card accent="accent" className="mb-5">
          <CardBody>
            <div className="flex flex-col gap-1.5">
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant="accent" size="sm">
                  {t(`season.phases.${seasonContext.phase}`)}
                </Badge>
                <span className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
                  {seasonContext.season_start
                    ? t("season.startsOn", {
                        date: formatMatchDate(seasonContext.season_start),
                      })
                    : t("season.noOpener")}
                </span>
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                {t("season.tournamentsPreseasonHint")}
              </p>
            </div>
          </CardBody>
        </Card>
      )}

      {/* League header */}
      <Card accent="primary" className="mb-5">
        <div className="bg-gradient-to-r from-navy-700 to-navy-800 p-6 rounded-t-xl">
          <div className="flex items-center gap-4">
            <div className="w-14 h-14 rounded-xl bg-accent-500/20 flex items-center justify-center">
              <Trophy className="w-7 h-7 text-accent-400" />
            </div>
            <div className="flex-1">
              <h2 className="text-2xl font-heading font-bold text-white uppercase tracking-wide">
                {league.name}
              </h2>
              <p className="text-gray-400 text-sm mt-0.5">
                {t("schedule.season", { number: league.season })} —{" "}
                {t("tournaments.nTeams", { count: league.standings.length })}
              </p>
            </div>
            <div className="hidden md:flex gap-4">
              <div className="bg-white/5 rounded-xl px-4 py-2 text-center">
                <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                  {t("tournaments.progress")}
                </p>
                <p className="font-heading font-bold text-lg text-white">
                  {completedMatchdays}/{totalMatchdays}
                </p>
              </div>
              <div className="bg-white/5 rounded-xl px-4 py-2 text-center">
                <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                  {t("tournaments.matches")}
                </p>
                <p className="font-heading font-bold text-lg text-white">
                  {completedMatches}
                </p>
              </div>
              <div className="bg-white/5 rounded-xl px-4 py-2 text-center">
                <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">
                  Victorias
                </p>
                <p className="font-heading font-bold text-lg text-accent-400">
                  {userWins}
                </p>
              </div>
            </div>
          </div>
        </div>
      </Card>

      {/* Tab switcher */}
      <div className="flex gap-2 mb-5">
        {(["overview", "standings", "fixtures"] as const).map((v) => (
          <button
            key={v}
            onClick={() => setView(v)}
            className={`px-4 py-2 rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-all ${
              view === v
                ? "bg-primary-500 text-white shadow-md shadow-primary-500/20"
                : "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 border border-gray-200 dark:border-navy-600"
            }`}
          >
            {v === "overview" ? (
              <>
                <Trophy className="w-4 h-4 inline mr-1.5 -mt-0.5" />
                {t("tournaments.overview")}
              </>
            ) : v === "standings" ? (
              <>
                <TableProperties className="w-4 h-4 inline mr-1.5 -mt-0.5" />
                {t("schedule.standings")}
              </>
            ) : (
              <>
                <Calendar className="w-4 h-4 inline mr-1.5 -mt-0.5" />
                {t("schedule.fixtures")}
              </>
            )}
          </button>
        ))}
      </div>

      {/* Overview */}
      {view === "overview" && (
        <div className="space-y-5">
          {hasPlayoffsStarted ? (
            <PlayoffBracketBoard
              league={league}
              teams={gameState.teams}
              onSelectTeam={onSelectTeam}
              title={`${t("schedule.playoffs")} · ${t("tournaments.bracket")}`}
            />
          ) : null}

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
          {/* Mini standings */}
          <Card className="lg:col-span-2">
            <CardHeader>{t("tournaments.leagueTable")}</CardHeader>
            <CardBody className="p-0">
              {isPreseason ? (
                <div className="flex flex-col items-center gap-2 px-6 py-8 text-center">
                  <Trophy className="w-8 h-8 text-gray-300 dark:text-navy-600" />
                  <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
                    {t("season.standingsLocked")}
                  </p>
                  <p className="text-xs text-gray-500 dark:text-gray-400 max-w-md">
                    {t("season.tournamentsPreseasonHint")}
                  </p>
                </div>
              ) : (
                <table className="w-full text-left border-collapse">
                  <thead>
                    <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 w-8">
                        #
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                        {t("common.team")}
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("common.played")}
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("common.won")}
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("tournaments.winRateShort")}
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("common.lost")}
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("common.gd")}
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                    {standings.map((entry, idx) => {
                      const isUser = entry.team_id === userTeamId;
                      const gd = entry.goals_for - entry.goals_against;
                      const winRate = entry.played > 0 ? Math.round((entry.won / entry.played) * 100) : 0;
                      return (
                        <tr
                          key={entry.team_id}
                          onClick={() => onSelectTeam(entry.team_id)}
                          className={`cursor-pointer transition-colors ${isUser ? "bg-primary-50 dark:bg-primary-500/10" : "hover:bg-gray-50 dark:hover:bg-navy-700/50"}`}
                        >
                          <td className="py-2 px-3 font-heading font-bold text-sm text-gray-400">
                            {idx + 1}
                          </td>
                          <td
                            className={`py-2 px-3 font-semibold text-sm ${isUser ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                          >
                            {getTeamName(gameState.teams, entry.team_id)}
                          </td>
                          <td className="py-2 px-3 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                            {entry.played}
                          </td>
                          <td className="py-2 px-3 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                            {entry.won}
                          </td>
                          <td className="py-2 px-3 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                            {winRate}%
                          </td>
                          <td className="py-2 px-3 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                            {entry.lost}
                          </td>
                          <td
                            className={`py-2 px-3 text-center text-sm font-semibold tabular-nums ${gd > 0 ? "text-primary-500" : gd < 0 ? "text-red-500" : "text-gray-500"}`}
                          >
                            {gd > 0 ? `+${gd}` : gd}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              )}
            </CardBody>
          </Card>

          <Card>
            <CardHeader>{t("schedule.fixtures")}</CardHeader>
            <CardBody className="p-0">
              {sortedMatchdays.length === 0 ? (
                <p className="p-4 text-sm text-gray-400 dark:text-gray-500 text-center">
                  {t("season.noOpener")}
                </p>
              ) : (
                <div className="divide-y divide-gray-100 dark:divide-navy-600">
                  {sortedMatchdays.slice(-5).map(([md, fixtures]) => {
                    const first = fixtures[0];
                    return (
                      <div key={`overview-md-${md}`} className="px-4 py-3">
                        <p className="text-[11px] uppercase tracking-wider text-gray-500 dark:text-gray-400 font-heading font-bold">
                          {first.competition === "Playoffs"
                            ? `${t("schedule.playoffs")} · ${t("schedule.round", { number: md })}`
                            : t("schedule.matchday", { number: md })}
                        </p>
                        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
                          {formatMatchDate(first.date)} · {fixtures.length} {t("tournaments.matches").toLowerCase()}
                        </p>
                      </div>
                    );
                  })}
                </div>
              )}
            </CardBody>
          </Card>
          </div>
        </div>
      )}

      {/* Full standings */}
      {view === "standings" &&
        (isPreseason ? (
          <Card>
            <CardBody>
              <div className="flex flex-col items-center gap-2 py-6 text-center">
                <Trophy className="w-8 h-8 text-gray-300 dark:text-navy-600" />
                <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-100">
                  {t("season.standingsLocked")}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400 max-w-md">
                  {t("season.tournamentsPreseasonHint")}
                </p>
              </div>
            </CardBody>
            </Card>
        ) : hasPlayoffsStarted ? (
          <PlayoffBracketBoard
            league={league}
            teams={gameState.teams}
            onSelectTeam={onSelectTeam}
            title={`${t("schedule.playoffs")} · ${t("tournaments.bracket")}`}
          />
        ) : (
          <Card>
            <div className="p-5 border-b border-gray-100 dark:border-navy-600 bg-gradient-to-r from-navy-700 to-navy-800 rounded-t-xl">
              <h3 className="text-lg font-heading font-bold text-white flex items-center gap-2 uppercase tracking-wide">
                <Trophy className="text-accent-400 w-5 h-5" />
                {league.name} —{" "}
                {t("schedule.season", { number: league.season })}
              </h3>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full text-left border-collapse">
                <thead>
                  <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 w-8">
                      #
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                      {t("common.team")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.played")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.won")}
                    </th>
                      <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("tournaments.winRateShort")}
                      </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.lost")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("tournaments.killsShort")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("tournaments.deathsShort")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.gd")}
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                  {standings.map((entry, idx) => {
                    const isUser = entry.team_id === userTeamId;
                    const gd = entry.goals_for - entry.goals_against;
                    const winRate = entry.played > 0 ? Math.round((entry.won / entry.played) * 100) : 0;
                    return (
                      <tr
                        key={entry.team_id}
                        onClick={() => onSelectTeam(entry.team_id)}
                        className={`cursor-pointer transition-colors ${isUser ? "bg-primary-50 dark:bg-primary-500/10" : "hover:bg-gray-50 dark:hover:bg-navy-700/50"}`}
                      >
                        <td className="py-3 px-4 font-heading font-bold text-sm text-gray-400">
                          {idx + 1}
                        </td>
                        <td
                          className={`py-3 px-4 font-semibold text-sm ${isUser ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                        >
                          {getTeamName(gameState.teams, entry.team_id)}
                        </td>
                        <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                          {entry.played}
                        </td>
                        <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                          {entry.won}
                        </td>
                        <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                          {winRate}%
                        </td>
                        <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                          {entry.lost}
                        </td>
                        <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                          {entry.goals_for}
                        </td>
                        <td className="py-3 px-4 text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums">
                          {entry.goals_against}
                        </td>
                        <td
                          className={`py-3 px-4 text-center text-sm font-semibold tabular-nums ${gd > 0 ? "text-primary-500" : gd < 0 ? "text-red-500" : "text-gray-500"}`}
                        >
                          {gd > 0 ? `+${gd}` : gd}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </Card>
        ))}

      {/* Fixtures */}
      {view === "fixtures" && (
        <div className="flex flex-col gap-4">
          {sortedMatchdays.map(([md, fixtures]) => (
            <Card key={md}>
              <div className="px-5 py-3 border-b border-gray-100 dark:border-navy-600 bg-gray-50 dark:bg-navy-800 rounded-t-xl">
                <h4 className="font-heading font-bold text-sm uppercase tracking-wider text-gray-600 dark:text-gray-300">
                  {fixtures[0].competition === "Playoffs"
                    ? `${t("schedule.playoffs")} · ${t("schedule.round", { number: md })}`
                    : t("schedule.matchday", { number: md })} — {formatMatchDate(fixtures[0].date)}
                </h4>
              </div>
              <CardBody className="p-0">
                <div className="divide-y divide-gray-100 dark:divide-navy-600">
                  {fixtures.map((f) => {
                    const isUserMatch =
                      f.home_team_id === userTeamId ||
                      f.away_team_id === userTeamId;
                    const completed = f.status === "Completed";
                    return (
                      <div
                        key={f.id}
                        className={`flex items-center px-5 py-3 transition-colors ${isUserMatch ? "bg-primary-50/50 dark:bg-primary-500/5" : ""}`}
                      >
                        <span
                          onClick={() => onSelectTeam(f.home_team_id)}
                          className={`flex-1 text-right font-semibold text-sm cursor-pointer hover:underline ${f.home_team_id === userTeamId ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                        >
                          {getTeamName(gameState.teams, f.home_team_id)}
                        </span>
                        <div className="w-24 text-center mx-3">
                          {completed && f.result ? (
                            <span className="font-heading font-bold text-lg text-gray-800 dark:text-gray-100">
                              {(f.result.home_wins ?? f.result.home_goals ?? 0)} - {(f.result.away_wins ?? f.result.away_goals ?? 0)}
                            </span>
                          ) : (
                            <Badge variant="neutral" size="sm">
                              vs
                            </Badge>
                          )}
                        </div>
                        <span
                          onClick={() => onSelectTeam(f.away_team_id)}
                          className={`flex-1 text-left font-semibold text-sm cursor-pointer hover:underline ${f.away_team_id === userTeamId ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                        >
                          {getTeamName(gameState.teams, f.away_team_id)}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </CardBody>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
