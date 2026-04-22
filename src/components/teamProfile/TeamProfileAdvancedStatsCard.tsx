import { Card, CardBody, CardHeader } from "../ui";

import type { TeamProfileTranslate, TeamStatsOverview } from "./TeamProfile.types";

interface TeamProfileAdvancedStatsCardProps {
  overview: TeamStatsOverview;
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

function formatRate(value: number | null): string {
  if (value === null) {
    return "-";
  }

  return value.toFixed(1);
}

function formatSeconds(value: number | null): string {
  if (value === null) {
    return "-";
  }

  return `${value.toFixed(0)}s`;
}

function SummaryStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg bg-gray-50 dark:bg-navy-700 px-3 py-2.5 text-center">
      <p className="text-[11px] uppercase tracking-wider text-gray-400 dark:text-gray-500">
        {label}
      </p>
      <p className="font-heading font-bold text-lg text-gray-800 dark:text-gray-100 tabular-nums">
        {value}
      </p>
    </div>
  );
}

function MetricRow({
  label,
  primaryValue,
  secondaryLabel,
  secondaryValue,
}: {
  label: string;
  primaryValue: string;
  secondaryLabel: string;
  secondaryValue: string;
}) {
  return (
    <div className="grid grid-cols-[minmax(0,1.4fr)_minmax(0,0.9fr)] gap-3 items-center rounded-lg bg-gray-50 dark:bg-navy-700 px-3 py-2.5">
      <div>
        <p className="font-heading font-bold text-sm uppercase tracking-wider text-gray-500 dark:text-gray-400">
          {label}
        </p>
        <p className="font-heading font-bold text-lg text-gray-800 dark:text-gray-100 tabular-nums">
          {primaryValue}
        </p>
      </div>

      <div className="text-center">
        <p className="text-[11px] uppercase tracking-wider text-gray-400 dark:text-gray-500">
          {secondaryLabel}
        </p>
        <p className="font-heading font-bold text-base text-gray-700 dark:text-gray-200 tabular-nums">
          {secondaryValue}
        </p>
      </div>
    </div>
  );
}

export default function TeamProfileAdvancedStatsCard({
  overview,
  t,
}: TeamProfileAdvancedStatsCardProps) {
  const labels = {
    title: resolveLabel(t, "teamProfile.advancedStats", "Team Stats"),
    matchesPlayed: resolveLabel(t, "teamProfile.matchesPlayed", "Matches"),
    wins: resolveLabel(t, "teamProfile.wins", "Wins"),
    losses: resolveLabel(t, "teamProfile.losses", "Losses"),
    draws: resolveLabel(t, "teamProfile.draws", "Draws"),
    kills: resolveLabel(t, "teamProfile.kills", "Kills"),
    deaths: resolveLabel(t, "teamProfile.deaths", "Deaths"),
    goldEarned: resolveLabel(t, "teamProfile.goldEarned", "Gold Earned"),
    damageToChampions: resolveLabel(
      t,
      "teamProfile.damageToChampions",
      "Damage To Champions",
    ),
    objectives: resolveLabel(t, "teamProfile.objectives", "Objectives"),
    averageGameDuration: resolveLabel(
      t,
      "teamProfile.averageGameDuration",
      "Average Game Duration",
    ),
    perMatch: resolveLabel(t, "teamProfile.perMatch", "Per Match"),
  };

  return (
    <Card className="lg:col-span-3">
      <CardHeader>{labels.title}</CardHeader>
      <CardBody>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-4">
          <SummaryStat label={labels.matchesPlayed} value={String(overview.matchesPlayed)} />
          <SummaryStat label={labels.wins} value={String(overview.wins)} />
          <SummaryStat label={labels.losses} value={String(overview.losses)} />
          <SummaryStat label={labels.draws} value={String(overview.draws)} />
        </div>

        <div className="space-y-3">
          <MetricRow
            label={labels.kills}
            primaryValue={String(overview.metrics.kills.total)}
            secondaryLabel={labels.perMatch}
            secondaryValue={formatRate(overview.metrics.kills.perMatch)}
          />
          <MetricRow
            label={labels.deaths}
            primaryValue={String(overview.metrics.deaths.total)}
            secondaryLabel={labels.perMatch}
            secondaryValue={formatRate(overview.metrics.deaths.perMatch)}
          />
          <MetricRow
            label={labels.goldEarned}
            primaryValue={String(overview.metrics.goldEarned.total)}
            secondaryLabel={labels.perMatch}
            secondaryValue={formatRate(overview.metrics.goldEarned.perMatch)}
          />
          <MetricRow
            label={labels.damageToChampions}
            primaryValue={String(overview.metrics.damageToChampions.total)}
            secondaryLabel={labels.perMatch}
            secondaryValue={formatRate(overview.metrics.damageToChampions.perMatch)}
          />
          <MetricRow
            label={labels.objectives}
            primaryValue={String(overview.metrics.objectives.total)}
            secondaryLabel={labels.perMatch}
            secondaryValue={formatRate(overview.metrics.objectives.perMatch)}
          />
          <MetricRow
            label={labels.averageGameDuration}
            primaryValue={formatSeconds(overview.metrics.averageGameDurationSeconds.perMatch)}
            secondaryLabel={labels.perMatch}
            secondaryValue={formatRate(overview.metrics.averageGameDurationSeconds.perMatch)}
          />
        </div>
      </CardBody>
    </Card>
  );
}
