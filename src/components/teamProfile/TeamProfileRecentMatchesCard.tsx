import { Card, CardBody, CardHeader } from "../ui";

import type { TeamProfileTranslate, TeamRecentMatchEntry } from "./TeamProfile.types";

interface TeamProfileRecentMatchesCardProps {
  matches: TeamRecentMatchEntry[];
  t: TeamProfileTranslate;
}

function resolveLabel(
  t: TeamProfileTranslate,
  key: string,
  fallback: string,
): string {
  const translated = t(key);
  return translated === key ? fallback : translated;
}

export default function TeamProfileRecentMatchesCard({
  matches,
  t,
}: TeamProfileRecentMatchesCardProps) {
  const title = resolveLabel(t, "teamProfile.recentMatches", "Recent Matches");
  const sideLabel = resolveLabel(t, "teamProfile.side", "Side");
  const scoreLabel = resolveLabel(t, "teamProfile.kda", "K / D / A");
  const economyLabel = resolveLabel(t, "teamProfile.economy", "Gold / Objectives");

  if (matches.length === 0) {
    return null;
  }

  return (
    <Card className="lg:col-span-3">
      <CardHeader>{title}</CardHeader>
      <CardBody>
        <div className="space-y-3">
          {matches.map((match) => (
            <div
              key={match.fixtureId}
              className="grid grid-cols-[minmax(0,1.4fr)_minmax(0,0.8fr)_minmax(0,0.9fr)_minmax(0,1fr)] gap-3 rounded-lg bg-gray-50 dark:bg-navy-700 px-3 py-2.5"
            >
              <div>
                <p className="font-heading font-bold text-sm uppercase tracking-wider text-gray-500 dark:text-gray-400">
                  {match.date}
                </p>
                <p className="font-heading font-bold text-base text-gray-800 dark:text-gray-100">
                  {match.opponentName}
                </p>
              </div>

              <div className="text-center">
                <p className="text-[11px] uppercase tracking-wider text-gray-400 dark:text-gray-500">
                  {sideLabel}
                </p>
                <p className="font-heading font-bold text-base text-gray-700 dark:text-gray-200 tabular-nums">
                  {match.side} · {match.result}
                </p>
              </div>

              <div className="text-center">
                <p className="text-[11px] uppercase tracking-wider text-gray-400 dark:text-gray-500">
                  {scoreLabel}
                </p>
                <p className="font-heading font-bold text-base text-gray-700 dark:text-gray-200 tabular-nums">
                  {match.kills} / {match.deaths} / {match.objectives}
                </p>
              </div>

              <div className="text-center">
                <p className="text-[11px] uppercase tracking-wider text-gray-400 dark:text-gray-500">
                  {economyLabel}
                </p>
                <p className="font-heading font-bold text-base text-gray-700 dark:text-gray-200 tabular-nums">
                  {match.goldEarned} / {match.damageToChampions}
                </p>
              </div>
            </div>
          ))}
        </div>
      </CardBody>
    </Card>
  );
}
