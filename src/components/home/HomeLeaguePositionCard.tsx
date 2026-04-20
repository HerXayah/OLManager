import { Trophy } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge, Card, CardBody, CardHeader } from "../ui";
import type { TeamData } from "../../store/gameStore";

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
  sortedStandings: LeagueStandingSnapshot[];
  teams: TeamData[];
  myTeamId: string | null;
  onNavigate?: (tab: string) => void;
}

function teamLogoUrl(team: TeamData | undefined): string | null {
  if (!team) return null;
  const slug = team.id.replace(/^lec-/, "");

  const aliases: Record<string, string> = {
    shifters: "team-bds",
  };

  const file = aliases[slug] ?? slug;
  return `/team-logos/${file}.png`;
}

export default function HomeLeaguePositionCard({
  isPreseason,
  phase,
  seasonStartLabel,
  sortedStandings,
  teams,
  myTeamId,
  onNavigate,
}: HomeLeaguePositionCardProps) {
  const { t } = useTranslation();

  return (
    <Card accent="accent">
      <CardHeader
        action={
          <button
            onClick={() => onNavigate?.("Schedule")}
            className="text-primary-500 dark:text-primary-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-primary-600 dark:hover:text-primary-300 transition-colors"
          >
            {t("home.standings")}
          </button>
        }
      >
        {t("home.leaguePosition")}
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
