import { Card, CardBody, CardHeader } from "../ui";
import type { PlayerAdvancedStatsSummary } from "./PlayerProfile.helpers";

type TranslateFn = (key: string) => string;

interface PlayerProfileAdvancedStatsCardProps {
    summary: PlayerAdvancedStatsSummary;
    t: TranslateFn;
}

function resolveLabel(t: TranslateFn, key: string, fallback: string): string {
    const translated = t(key);
    return translated === key ? fallback : translated;
}

function formatRate(value: number | null): string {
    if (value === null) {
        return "-";
    }

    return value.toFixed(2);
}

function formatOrdinal(value: number | null, unavailableLabel: string): string {
    if (value === null) {
        return unavailableLabel;
    }

    const mod100 = value % 100;
    if (mod100 >= 11 && mod100 <= 13) {
        return `${value}th`;
    }

    switch (value % 10) {
        case 1:
            return `${value}st`;
        case 2:
            return `${value}nd`;
        case 3:
            return `${value}rd`;
        default:
            return `${value}th`;
    }
}

function AdvancedStatRow({
    label,
    primaryValue,
    secondaryLabel,
    secondaryValue,
    percentile,
    t,
}: {
    label: string;
    primaryValue: string;
    secondaryLabel: string;
    secondaryValue: string;
    percentile: number | null;
    t: TranslateFn;
}) {
    return (
        <div className="grid grid-cols-[minmax(0,1.4fr)_minmax(0,0.9fr)_minmax(0,0.9fr)] gap-3 items-center rounded-lg bg-gray-50 dark:bg-navy-700 px-3 py-2.5">
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

            <div className="text-center">
                <p className="text-[11px] uppercase tracking-wider text-gray-400 dark:text-gray-500">
                    {t("playerProfile.percentile")}
                </p>
                <p className="font-heading font-bold text-base text-gray-700 dark:text-gray-200 tabular-nums">
                    {formatOrdinal(
                        percentile,
                        t("playerProfile.percentileUnavailable"),
                    )}
                </p>
            </div>
        </div>
    );
}

export default function PlayerProfileAdvancedStatsCard({
    summary,
    t,
}: PlayerProfileAdvancedStatsCardProps) {
    const labels = {
        title: resolveLabel(t, "playerProfile.advancedStats", "Advanced Stats"),
        kills: resolveLabel(t, "playerProfile.kills", "Kills"),
        deaths: resolveLabel(t, "playerProfile.deaths", "Deaths"),
        assists: resolveLabel(t, "playerProfile.assists", "Assists"),
        cs: resolveLabel(t, "playerProfile.cs", "CS"),
        damageToChampions: resolveLabel(
            t,
            "playerProfile.damageToChampions",
            "Damage to Champions",
        ),
        visionScore: resolveLabel(t, "playerProfile.visionScore", "Vision Score"),
        perGame: resolveLabel(t, "playerProfile.perGame", "Per Game"),
        percentile: resolveLabel(t, "playerProfile.percentile", "Percentile"),
        percentileUnavailable: resolveLabel(
            t,
            "playerProfile.percentileUnavailable",
            "Percentile unavailable",
        ),
    };

    return (
        <Card className="lg:col-span-2">
            <CardHeader>{labels.title}</CardHeader>
            <CardBody>
                <div className="space-y-3">
                    <AdvancedStatRow
                        label={labels.kills}
                        primaryValue={String(summary.metrics.kills.total)}
                        secondaryLabel={labels.perGame}
                        secondaryValue={formatRate(summary.metrics.kills.perGame)}
                        percentile={summary.metrics.kills.percentile}
                        t={(key: string) =>
                            key === "playerProfile.percentile"
                                ? labels.percentile
                                : labels.percentileUnavailable
                        }
                    />
                    <AdvancedStatRow
                        label={labels.deaths}
                        primaryValue={String(summary.metrics.deaths.total)}
                        secondaryLabel={labels.perGame}
                        secondaryValue={formatRate(summary.metrics.deaths.perGame)}
                        percentile={summary.metrics.deaths.percentile}
                        t={(key: string) =>
                            key === "playerProfile.percentile"
                                ? labels.percentile
                                : labels.percentileUnavailable
                        }
                    />
                    <AdvancedStatRow
                        label={labels.assists}
                        primaryValue={String(summary.metrics.assists.total)}
                        secondaryLabel={labels.perGame}
                        secondaryValue={formatRate(summary.metrics.assists.perGame)}
                        percentile={summary.metrics.assists.percentile}
                        t={(key: string) =>
                            key === "playerProfile.percentile"
                                ? labels.percentile
                                : labels.percentileUnavailable
                        }
                    />
                    <AdvancedStatRow
                        label={labels.cs}
                        primaryValue={String(summary.metrics.cs.total)}
                        secondaryLabel={labels.perGame}
                        secondaryValue={formatRate(summary.metrics.cs.perGame)}
                        percentile={summary.metrics.cs.percentile}
                        t={(key: string) =>
                            key === "playerProfile.percentile"
                                ? labels.percentile
                                : labels.percentileUnavailable
                        }
                    />
                    <AdvancedStatRow
                        label={labels.damageToChampions}
                        primaryValue={String(summary.metrics.damageToChampions.total)}
                        secondaryLabel={labels.perGame}
                        secondaryValue={formatRate(summary.metrics.damageToChampions.perGame)}
                        percentile={summary.metrics.damageToChampions.percentile}
                        t={(key: string) =>
                            key === "playerProfile.percentile"
                                ? labels.percentile
                                : labels.percentileUnavailable
                        }
                    />
                    <AdvancedStatRow
                        label={labels.visionScore}
                        primaryValue={String(summary.metrics.visionScore.total)}
                        secondaryLabel={labels.perGame}
                        secondaryValue={formatRate(summary.metrics.visionScore.perGame)}
                        percentile={summary.metrics.visionScore.percentile}
                        t={(key: string) =>
                            key === "playerProfile.percentile"
                                ? labels.percentile
                                : labels.percentileUnavailable
                        }
                    />
                </div>
            </CardBody>
        </Card>
    );
}
