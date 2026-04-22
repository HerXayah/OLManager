import type { PlayerSeasonStats } from "../../store/gameStore";
import { Card, CardBody, CardHeader } from "../ui";

type TranslateFn = (key: string) => string;

function resolveLabel(t: TranslateFn, key: string, fallback: string): string {
    const translated = t(key);
    return translated === key ? fallback : translated;
}

interface PlayerProfileSeasonStatsCardProps {
    stats: PlayerSeasonStats;
    t: TranslateFn;
}

export default function PlayerProfileSeasonStatsCard({
    stats,
    t,
}: PlayerProfileSeasonStatsCardProps) {
    const timePlayedMinutes = Math.floor((stats.time_played_seconds ?? 0) / 60);

    return (
        <Card className="lg:col-span-2">
            <CardHeader>
                {resolveLabel(t, "playerProfile.seasonStats", "Season Stats")}
            </CardHeader>
            <CardBody>
                <div className="grid grid-cols-4 md:grid-cols-8 gap-3">
                    <StatBox
                        label={resolveLabel(t, "playerProfile.gamesPlayed", "Games")}
                        value={stats.games_played ?? 0}
                    />
                    <StatBox
                        label={resolveLabel(t, "playerProfile.wins", "Wins")}
                        value={stats.wins ?? 0}
                    />
                    <StatBox
                        label={resolveLabel(t, "playerProfile.losses", "Losses")}
                        value={stats.losses ?? 0}
                    />
                    <StatBox
                        label={resolveLabel(t, "playerProfile.kills", "Kills")}
                        value={stats.kills ?? 0}
                    />
                    <StatBox
                        label={resolveLabel(t, "playerProfile.deaths", "Deaths")}
                        value={stats.deaths ?? 0}
                    />
                    <StatBox
                        label={resolveLabel(t, "playerProfile.assists", "Assists")}
                        value={stats.assists ?? 0}
                    />
                    <StatBox
                        label={resolveLabel(t, "playerProfile.cs", "CS")}
                        value={stats.cs ?? 0}
                    />
                    <StatBox
                        label={resolveLabel(t, "playerProfile.timePlayed", "Time")}
                        value={timePlayedMinutes}
                    />
                </div>
            </CardBody>
        </Card>
    );
}

function StatBox({ label, value }: { label: string; value: number | string }) {
    return (
        <div className="text-center p-2.5 bg-gray-50 dark:bg-navy-700 rounded-lg">
            <p className="font-heading font-bold text-lg text-gray-800 dark:text-gray-100 tabular-nums">
                {value}
            </p>
            <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
                {label}
            </p>
        </div>
    );
}
