import type {
    PlayerData,
    PlayerSeasonStats,
    TeamData,
} from "../../store/gameStore";
import type { TOptions } from "i18next";
import { annualAmountToWeeklyCommitment } from "../../lib/finance";
import { formatWeeklyAmount } from "../../lib/helpers";

type TranslateFn = (key: string, options?: TOptions) => string;

interface PlayerAdvancedMetric {
    total: number;
    perGame: number | null;
    percentile: number | null;
}

export interface PlayerAdvancedStatsSummary {
    percentileEligible: boolean;
    metrics: {
        kills: PlayerAdvancedMetric;
        deaths: PlayerAdvancedMetric;
        assists: PlayerAdvancedMetric;
        cs: PlayerAdvancedMetric;
        damageToChampions: PlayerAdvancedMetric;
        visionScore: PlayerAdvancedMetric;
    };
}

interface BuildPlayerAdvancedStatsOptions {
    minimumMinutes?: number;
    minimumCohortSize?: number;
}

const DEFAULT_MINIMUM_MINUTES = 600;
const DEFAULT_MINIMUM_COHORT_SIZE = 3;

export function getPlayerTeamName(
    teams: TeamData[],
    teamId: string | null,
    labels: {
        freeAgent: string;
        unknown: string;
    },
): string {
    if (!teamId) {
        return labels.freeAgent;
    }

    return teams.find((team) => team.id === teamId)?.name ?? labels.unknown;
}

export function getPlayerAge(
    dateOfBirth: string,
    asOfDate: string = "2026-07-01",
): number {
    const birthDate = new Date(dateOfBirth);
    const currentDate = new Date(asOfDate);
    let age = currentDate.getFullYear() - birthDate.getFullYear();

    if (
        currentDate.getMonth() < birthDate.getMonth() ||
        (currentDate.getMonth() === birthDate.getMonth() &&
            currentDate.getDate() < birthDate.getDate())
    ) {
        age -= 1;
    }

    return age;
}

export function formatPlayerMarketValue(value: number): string {
    if (value >= 1_000_000) {
        return `€${(value / 1_000_000).toFixed(1)}M`;
    }
    if (value >= 1_000) {
        return `€${(value / 1_000).toFixed(0)}K`;
    }
    return `€${value}`;
}

export function formatPlayerWage(
    annualWage: number,
    weeklySuffix: string,
): string {
    const weeklyWage = annualAmountToWeeklyCommitment(annualWage);
    return formatWeeklyAmount(`€${weeklyWage.toLocaleString()}`, weeklySuffix);
}

export function getAttributeColorClass(value: number): string {
    if (value >= 80) {
        return "text-primary-500 dark:text-primary-400";
    }
    if (value >= 60) {
        return "text-accent-600 dark:text-accent-400";
    }
    if (value >= 40) {
        return "text-gray-600 dark:text-gray-400";
    }
    return "text-red-500 dark:text-red-400";
}

export function resolvePlayerInjuryName(
    injuryName: string,
    translate: TranslateFn,
): string {
    if (injuryName.includes(".")) {
        return translate(injuryName, { defaultValue: injuryName });
    }

    return translate(`common.injuries.${injuryName}`, {
        defaultValue: injuryName,
    });
}

function statValue(value: number | undefined): number {
    return value ?? 0;
}

function roundTo(value: number, digits: number): number {
    const factor = 10 ** digits;
    return Math.round(value * factor) / factor;
}

function calculatePerGame(total: number, gamesPlayed: number): number | null {
    if (gamesPlayed <= 0) {
        return null;
    }

    return roundTo(total / gamesPlayed, 1);
}

function positionKey(player: PlayerData): string {
    return player.natural_position || player.position;
}

function percentileRank(values: number[], target: number | null): number | null {
    if (target === null || values.length === 0) {
        return null;
    }

    const rankedCount = values.filter((value) => value <= target).length;
    return Math.round((rankedCount / values.length) * 100);
}

function eligiblePeers(
    player: PlayerData,
    players: PlayerData[],
    minimumMinutes: number,
): PlayerData[] {
    const targetPosition = positionKey(player);

    return players.filter((candidate) => {
        return (
            positionKey(candidate) === targetPosition &&
            statValue(candidate.stats.time_played_seconds) >= minimumMinutes
        );
    });
}

function metricPercentile(
    peers: PlayerData[],
    selector: (stats: PlayerSeasonStats) => number | null,
    playerStats: PlayerSeasonStats,
): number | null {
    const peerValues = peers
        .map((candidate) => selector(candidate.stats))
        .filter((value): value is number => value !== null);

    return percentileRank(peerValues, selector(playerStats));
}

export function buildPlayerAdvancedStats(
    player: PlayerData,
    players: PlayerData[],
    options: BuildPlayerAdvancedStatsOptions = {},
): PlayerAdvancedStatsSummary {
    const minimumMinutes = options.minimumMinutes ?? DEFAULT_MINIMUM_MINUTES;
    const minimumCohortSize =
        options.minimumCohortSize ?? DEFAULT_MINIMUM_COHORT_SIZE;
    const timePlayedSeconds = statValue(player.stats.time_played_seconds);
    const gamesPlayed = statValue(player.stats.games_played);
    const percentileEligible = timePlayedSeconds >= minimumMinutes;
    const peers = eligiblePeers(player, players, minimumMinutes);
    const canComputePercentiles =
        percentileEligible && peers.length >= minimumCohortSize;

    const kills = statValue(player.stats.kills);
    const deaths = statValue(player.stats.deaths);
    const assists = statValue(player.stats.assists);
    const cs = statValue(player.stats.cs);
    const damageToChampions = statValue(player.stats.damage_to_champions);
    const visionScore = statValue(player.stats.vision_score);

    return {
        percentileEligible: canComputePercentiles,
        metrics: {
            kills: {
                total: kills,
                perGame: calculatePerGame(kills, gamesPlayed),
                percentile: canComputePercentiles
                    ? metricPercentile(
                        peers,
                        (stats) =>
                            calculatePerGame(
                                statValue(stats.kills),
                                statValue(stats.games_played),
                            ),
                        player.stats,
                    )
                    : null,
            },
            deaths: {
                total: deaths,
                perGame: calculatePerGame(deaths, gamesPlayed),
                percentile: canComputePercentiles
                    ? metricPercentile(
                        peers,
                        (stats) =>
                            calculatePerGame(
                                statValue(stats.deaths),
                                statValue(stats.games_played),
                            ),
                        player.stats,
                    )
                    : null,
            },
            assists: {
                total: assists,
                perGame: calculatePerGame(assists, gamesPlayed),
                percentile: canComputePercentiles
                    ? metricPercentile(
                        peers,
                        (stats) =>
                            calculatePerGame(
                                statValue(stats.assists),
                                statValue(stats.games_played),
                            ),
                        player.stats,
                    )
                    : null,
            },
            cs: {
                total: cs,
                perGame: calculatePerGame(cs, gamesPlayed),
                percentile: canComputePercentiles
                    ? metricPercentile(
                        peers,
                        (stats) =>
                            calculatePerGame(
                                statValue(stats.cs),
                                statValue(stats.games_played),
                            ),
                        player.stats,
                    )
                    : null,
            },
            damageToChampions: {
                total: damageToChampions,
                perGame: calculatePerGame(damageToChampions, gamesPlayed),
                percentile: canComputePercentiles
                    ? metricPercentile(
                        peers,
                        (stats) =>
                            calculatePerGame(
                                statValue(stats.damage_to_champions),
                                statValue(stats.games_played),
                            ),
                        player.stats,
                    )
                    : null,
            },
            visionScore: {
                total: visionScore,
                perGame: calculatePerGame(visionScore, gamesPlayed),
                percentile: canComputePercentiles
                    ? metricPercentile(
                        peers,
                        (stats) =>
                            calculatePerGame(
                                statValue(stats.vision_score),
                                statValue(stats.games_played),
                            ),
                        player.stats,
                    )
                    : null,
            },
        },
    };
}
