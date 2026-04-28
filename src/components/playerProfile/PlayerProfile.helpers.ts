import type {
    PlayerData,
    TeamData,
} from "../../store/gameStore";
import type { TOptions } from "i18next";
import { annualAmountToWeeklyCommitment } from "../../lib/finance";
import { formatWeeklyAmount } from "../../lib/helpers";

type TranslateFn = (key: string, options?: TOptions) => string;

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
