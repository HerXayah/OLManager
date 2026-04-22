import { Card, CardBody, CardHeader } from "../ui";

type TranslateFn = (key: string) => string;

export interface PlayerRecentMatchEntry {
    fixtureId: string;
    date: string;
    competition: string;
    matchday: number;
    opponentTeamId: string;
    opponentName: string;
    side: "Blue" | "Red";
    result: "Win" | "Loss";
    championId?: string | null;
    gameDurationSeconds: number;
    kills: number;
    deaths: number;
    assists: number;
    cs: number;
}

function resolveLabel(t: TranslateFn, key: string, fallback: string): string {
    const translated = t(key);
    return translated === key ? fallback : translated;
}

interface PlayerProfileRecentMatchesCardProps {
    matches: PlayerRecentMatchEntry[];
    t: TranslateFn;
}

export default function PlayerProfileRecentMatchesCard({
    matches,
    t,
}: PlayerProfileRecentMatchesCardProps) {
    const title = resolveLabel(t, "playerProfile.recentMatches", "Recent Matches");

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
                            className="grid grid-cols-[minmax(0,1.4fr)_minmax(0,0.8fr)_minmax(0,0.8fr)_minmax(0,0.8fr)] gap-3 rounded-lg bg-gray-50 dark:bg-navy-700 px-3 py-2.5"
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
                                    Side / Result
                                </p>
                                <p className="font-heading font-bold text-base text-gray-700 dark:text-gray-200 tabular-nums">
                                    {match.side} · {match.result}
                                </p>
                            </div>

                            <div className="text-center">
                                <p className="text-[11px] uppercase tracking-wider text-gray-400 dark:text-gray-500">
                                    K / D / A
                                </p>
                                <p className="font-heading font-bold text-base text-gray-700 dark:text-gray-200 tabular-nums">
                                    {match.kills} / {match.deaths} / {match.assists}
                                </p>
                            </div>

                            <div className="text-center">
                                <p className="text-[11px] uppercase tracking-wider text-gray-400 dark:text-gray-500">
                                    CS
                                </p>
                                <p className="font-heading font-bold text-base text-gray-700 dark:text-gray-200 tabular-nums">
                                    {match.cs}
                                </p>
                            </div>
                        </div>
                    ))}
                </div>
            </CardBody>
        </Card>
    );
}
