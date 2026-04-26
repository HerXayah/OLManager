import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { GameStateData, FixtureData } from "../../store/gameStore";
import { Card, CardHeader, CardBody, Badge } from "../ui";
import {
  Trophy,
  Calendar,
  TableProperties,
  Award,
  Star,
  Shield,
  Users,
  Zap,
} from "lucide-react";
import {
  getCompetitiveFixtures,
  getTeamName,
  formatMatchDate,
} from "../../lib/helpers";
import { resolveSeasonContext } from "../../lib/seasonContext";
import { useTranslation } from "react-i18next";
import PlayoffBracketBoard from "../playoffs/PlayoffBracketBoard";

interface AwardEntry {
  player_id: string;
  player_name: string;
  team_id: string;
  team_name: string;
  value: number;
}
interface SeasonAwards {
  golden_boot: AwardEntry[];
  assist_king: AwardEntry[];
  player_of_year: AwardEntry[];
  clean_sheet_king: AwardEntry[];
  most_appearances: AwardEntry[];
  young_player: AwardEntry[];
}

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
  const [view, setView] = useState<
    "overview" | "fixtures" | "standings" | "awards"
  >("overview");
  const [awards, setAwards] = useState<SeasonAwards | null>(null);

  useEffect(() => {
    if (view === "awards" && !awards) {
      invoke<SeasonAwards>("get_season_awards")
        .then(setAwards)
        .catch(() => {});
    }
  }, [view, awards]);

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

  const competitiveFixtures = getCompetitiveFixtures(league.fixtures);
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

  const topKda = (() => {
    const leagueTeamIds = new Set(league.standings.map((entry) => entry.team_id));
    return gameState.players
      .filter((player) => (player.team_id ? leagueTeamIds.has(player.team_id) : false))
      .map((player) => {
        const kills = Number(player.stats.kills ?? 0);
        const deaths = Number(player.stats.deaths ?? 0);
        const assists = Number(player.stats.assists ?? 0);
        const gamesPlayed = Number(player.stats.games_played ?? player.stats.appearances ?? 0);
        const kda = (kills + assists) / Math.max(1, deaths);
        return {
          player,
          kills,
          deaths,
          assists,
          gamesPlayed,
          kda,
        };
      })
      .filter((entry) => entry.gamesPlayed > 0 || entry.kills + entry.deaths + entry.assists > 0)
      .sort((a, b) => b.kda - a.kda || (b.kills + b.assists) - (a.kills + a.assists))
      .slice(0, 10);
  })();

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
        {(["overview", "standings", "fixtures", "awards"] as const).map((v) => (
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
            ) : v === "awards" ? (
              <>
                <Award className="w-4 h-4 inline mr-1.5 -mt-0.5" />
                Awards
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
              title={`${t("schedule.playoffs")} · Bracket`}
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
                        WR
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        {t("common.lost")}
                      </th>
                      <th className="py-2 px-3 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                        +/-
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

          {/* Top scorers */}
          <Card>
            <CardHeader>Top K/D/A</CardHeader>
            <CardBody className="p-0">
              {topKda.length === 0 ? (
                <p className="p-4 text-sm text-gray-400 dark:text-gray-500 text-center">
                  Sin datos de K/D/A todavía.
                </p>
              ) : (
                <div className="divide-y divide-gray-100 dark:divide-navy-600">
                  {topKda.map((entry, i) => (
                    <div
                      key={entry.player.id}
                      className="flex items-center px-4 py-2.5 gap-3"
                    >
                      <span className="font-heading font-bold text-sm text-gray-400 dark:text-gray-500 w-5 text-center">
                        {i + 1}
                      </span>
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-semibold text-gray-800 dark:text-gray-200 truncate">
                          {entry.player.match_name}
                        </p>
                        <p className="text-xs text-gray-400 dark:text-gray-500">
                          {getTeamName(
                            gameState.teams,
                            entry.player.team_id ?? "",
                          )}
                        </p>
                        <p className="text-[11px] text-gray-500 dark:text-gray-400 tabular-nums">
                          {entry.kills}/{entry.deaths}/{entry.assists}
                        </p>
                      </div>
                      <span className="font-heading font-bold text-lg text-accent-500 tabular-nums">
                        {entry.kda.toFixed(2)}
                      </span>
                    </div>
                  ))}
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
            title={`${t("schedule.playoffs")} · Bracket`}
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
                        WR
                      </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      {t("common.lost")}
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      K
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      D
                    </th>
                    <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                      +/-
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
      {/* Awards */}
      {view === "awards" && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
          {awards ? (
            <>
              <AwardCard
                icon={<Zap className="w-5 h-5 text-accent-500" />}
                title="Kill Leader"
                subtitle="Most Kills"
                entries={awards.golden_boot}
                unit="kills"
              />
              <AwardCard
                icon={<Star className="w-5 h-5 text-purple-500" />}
                title="Assist King"
                subtitle="Most Assists"
                entries={awards.assist_king}
                unit="assists"
              />
              <AwardCard
                icon={<Trophy className="w-5 h-5 text-primary-500" />}
                title="Split MVP"
                subtitle="Best Avg Rating (min 5 games)"
                entries={awards.player_of_year}
                unit="rating"
                decimal
              />
              <AwardCard
                icon={<Shield className="w-5 h-5 text-blue-500" />}
                title="Untouchable"
                subtitle="Most Deathless Games"
                entries={awards.clean_sheet_king}
                unit="games"
              />
              <AwardCard
                icon={<Users className="w-5 h-5 text-green-500" />}
                title="Grinder"
                subtitle="Most Games"
                entries={awards.most_appearances}
                unit="games"
              />
              <AwardCard
                icon={<Star className="w-5 h-5 text-amber-500" />}
                title="Rookie Star"
                subtitle="Best U21 Avg Rating (min 3 games)"
                entries={awards.young_player}
                unit="rating"
                decimal
              />
            </>
          ) : (
            <div className="col-span-full text-center py-12">
              <Award className="w-12 h-12 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
              <p className="text-sm text-gray-400 dark:text-gray-500">
                Loading awards...
              </p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function AwardCard({
  icon,
  title,
  subtitle,
  entries,
  unit,
  decimal,
}: {
  icon: React.ReactNode;
  title: string;
  subtitle: string;
  entries: AwardEntry[];
  unit: string;
  decimal?: boolean;
}) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          {icon}
          <div>
            <span>{title}</span>
            <p className="text-[10px] text-gray-400 dark:text-gray-500 font-normal normal-case tracking-normal">
              {subtitle}
            </p>
          </div>
        </div>
      </CardHeader>
      <CardBody className="p-0">
        {entries.length === 0 ? (
          <p className="p-4 text-sm text-gray-400 dark:text-gray-500 text-center">
            No data yet
          </p>
        ) : (
          <div className="divide-y divide-gray-100 dark:divide-navy-600">
            {entries.map((entry, i) => (
              <div
                key={entry.player_id}
                className="flex items-center px-4 py-2.5 gap-3"
              >
                <span
                  className={`font-heading font-bold text-sm w-5 text-center ${
                    i === 0
                      ? "text-accent-500"
                      : "text-gray-400 dark:text-gray-500"
                  }`}
                >
                  {i + 1}
                </span>
                <div className="flex-1 min-w-0">
                  <p
                    className={`text-sm font-semibold truncate ${
                      i === 0
                        ? "text-gray-900 dark:text-gray-100"
                        : "text-gray-700 dark:text-gray-300"
                    }`}
                  >
                    {entry.player_name}
                  </p>
                  <p className="text-xs text-gray-400 dark:text-gray-500">
                    {entry.team_name}
                  </p>
                </div>
                <span
                  className={`font-heading font-bold tabular-nums ${
                    i === 0
                      ? "text-lg text-accent-500"
                      : "text-sm text-gray-600 dark:text-gray-400"
                  }`}
                >
                  {decimal ? entry.value.toFixed(2) : entry.value}
                </span>
                <span className="text-[10px] text-gray-400 dark:text-gray-500 w-12">
                  {unit}
                </span>
              </div>
            ))}
          </div>
        )}
      </CardBody>
    </Card>
  );
}
