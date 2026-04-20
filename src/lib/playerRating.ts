import type { PlayerData } from "../store/gameStore";

const POSITION_ALIASES: Record<string, string> = {
    gk: "Goalkeeper",
    goalkeeper: "Goalkeeper",
    defender: "Defender",
    def: "Defender",
    midfielder: "Midfielder",
    mid: "Midfielder",
    forward: "Forward",
    fwd: "Forward",
    wingback: "Defender",
    winger: "Forward",
    rb: "RightBack",
    rightback: "RightBack",
    cb: "CenterBack",
    centerback: "CenterBack",
    centreback: "CenterBack",
    lb: "LeftBack",
    leftback: "LeftBack",
    rwb: "RightWingBack",
    rightwingback: "RightWingBack",
    lwb: "LeftWingBack",
    leftwingback: "LeftWingBack",
    dm: "DefensiveMidfielder",
    defensivemidfielder: "DefensiveMidfielder",
    cm: "CentralMidfielder",
    centralmidfielder: "CentralMidfielder",
    am: "AttackingMidfielder",
    attackingmidfielder: "AttackingMidfielder",
    rm: "RightMidfielder",
    rightmidfielder: "RightMidfielder",
    lm: "LeftMidfielder",
    leftmidfielder: "LeftMidfielder",
    rw: "RightWinger",
    rightwinger: "RightWinger",
    lw: "LeftWinger",
    leftwinger: "LeftWinger",
    st: "Striker",
    striker: "Striker",
};

const POSITION_GROUPS: Record<string, string> = {
    Goalkeeper: "Goalkeeper",
    Defender: "Defender",
    Midfielder: "Midfielder",
    Forward: "Forward",
    RightBack: "Defender",
    CenterBack: "Defender",
    LeftBack: "Defender",
    RightWingBack: "Defender",
    LeftWingBack: "Defender",
    DefensiveMidfielder: "Midfielder",
    CentralMidfielder: "Midfielder",
    AttackingMidfielder: "Midfielder",
    RightMidfielder: "Midfielder",
    LeftMidfielder: "Midfielder",
    RightWinger: "Forward",
    LeftWinger: "Forward",
    Striker: "Forward",
};

function normalisePositionKey(value: string): string {
    return value.toLowerCase().replace(/[^a-z]/g, "");
}

export function canonicalPosition(position: string): string {
    const trimmed = position.trim();
    if (!trimmed) {
        return trimmed;
    }
    return POSITION_ALIASES[normalisePositionKey(trimmed)] || trimmed;
}

function exactPosition(position: string): string {
    switch (canonicalPosition(position)) {
        case "Defender":
            return "CenterBack";
        case "Midfielder":
            return "CentralMidfielder";
        case "Forward":
            return "Striker";
        default:
            return canonicalPosition(position);
    }
}

function positionGroup(position: string): string {
    const canonical = canonicalPosition(position);
    return POSITION_GROUPS[canonical] || canonical;
}

function weightedAverage(values: Array<[number, number]>): number {
    return values.reduce((sum, [value, weight]) => sum + value * weight, 0) / 100;
}

function primaryPosition(player: PlayerData): string {
    const preferred = canonicalPosition(player.natural_position || player.position);
    if (["Defender", "Midfielder", "Forward", "Goalkeeper"].includes(preferred)) {
        return exactPosition(player.position || preferred);
    }
    return exactPosition(preferred);
}

function compatibilityPenalty(player: PlayerData, position: string): number {
    const exact = exactPosition(position);
    const primary = primaryPosition(player);
    if (primary === exact) {
        return 0;
    }

    const alternates = (player.alternate_positions || []).map(exactPosition);
    if (alternates.includes(exact)) {
        return 4;
    }
    if (positionGroup(primary) === positionGroup(exact)) {
        return 8;
    }
    return 14;
}

function sideForPosition(position: string): "Left" | "Right" | null {
    switch (exactPosition(position)) {
        case "LeftBack":
        case "LeftWingBack":
        case "LeftMidfielder":
        case "LeftWinger":
            return "Left";
        case "RightBack":
        case "RightWingBack":
        case "RightMidfielder":
        case "RightWinger":
            return "Right";
        default:
            return null;
    }
}

function footednessPenalty(player: PlayerData, position: string): number {
    const side = sideForPosition(position);
    if (!side) {
        return 0;
    }

    const footedness = player.footedness || "Right";
    if (footedness === "Both" || footedness === side) {
        return 0;
    }

    const weakFoot = Math.max(1, Math.min(5, player.weak_foot ?? 2));
    return Math.max(0, 10 - weakFoot * 2);
}

function weightedPositionScore(player: PlayerData, position: string): number {
    const attributes = player.attributes;
    switch (exactPosition(position)) {
        case "Goalkeeper":
            return weightedAverage([
                [attributes.handling, 28],
                [attributes.reflexes, 28],
                [attributes.aerial, 14],
                [attributes.positioning, 10],
                [attributes.decisions, 10],
                [attributes.composure, 5],
                [attributes.strength, 5],
            ]);
        case "RightBack":
        case "LeftBack":
            return weightedAverage([
                [attributes.pace, 18],
                [attributes.stamina, 16],
                [attributes.tackling, 17],
                [attributes.defending, 16],
                [attributes.positioning, 12],
                [attributes.passing, 10],
                [attributes.dribbling, 6],
                [attributes.decisions, 5],
            ]);
        case "CenterBack":
            return weightedAverage([
                [attributes.defending, 24],
                [attributes.tackling, 18],
                [attributes.positioning, 18],
                [attributes.strength, 14],
                [attributes.aerial, 12],
                [attributes.decisions, 8],
                [attributes.composure, 6],
            ]);
        case "RightWingBack":
        case "LeftWingBack":
            return weightedAverage([
                [attributes.pace, 18],
                [attributes.stamina, 18],
                [attributes.tackling, 14],
                [attributes.defending, 12],
                [attributes.passing, 13],
                [attributes.dribbling, 11],
                [attributes.vision, 7],
                [attributes.decisions, 7],
            ]);
        case "DefensiveMidfielder":
            return weightedAverage([
                [attributes.tackling, 18],
                [attributes.positioning, 18],
                [attributes.decisions, 16],
                [attributes.passing, 14],
                [attributes.defending, 12],
                [attributes.stamina, 10],
                [attributes.vision, 7],
                [attributes.strength, 5],
            ]);
        case "CentralMidfielder":
            return weightedAverage([
                [attributes.passing, 20],
                [attributes.vision, 16],
                [attributes.decisions, 16],
                [attributes.stamina, 12],
                [attributes.dribbling, 10],
                [attributes.positioning, 9],
                [attributes.teamwork, 9],
                [attributes.tackling, 8],
            ]);
        case "AttackingMidfielder":
            return weightedAverage([
                [attributes.vision, 20],
                [attributes.passing, 18],
                [attributes.dribbling, 16],
                [attributes.decisions, 14],
                [attributes.shooting, 10],
                [attributes.positioning, 8],
                [attributes.composure, 8],
                [attributes.pace, 6],
            ]);
        case "RightMidfielder":
        case "LeftMidfielder":
            return weightedAverage([
                [attributes.pace, 17],
                [attributes.stamina, 16],
                [attributes.passing, 15],
                [attributes.dribbling, 14],
                [attributes.vision, 10],
                [attributes.decisions, 10],
                [attributes.positioning, 10],
                [attributes.tackling, 8],
            ]);
        case "RightWinger":
        case "LeftWinger":
            return weightedAverage([
                [attributes.pace, 22],
                [attributes.dribbling, 22],
                [attributes.passing, 14],
                [attributes.shooting, 12],
                [attributes.vision, 10],
                [attributes.decisions, 8],
                [attributes.positioning, 6],
                [attributes.stamina, 6],
            ]);
        case "Striker":
            return weightedAverage([
                [attributes.shooting, 26],
                [attributes.positioning, 18],
                [attributes.decisions, 14],
                [attributes.pace, 12],
                [attributes.dribbling, 10],
                [attributes.strength, 8],
                [attributes.composure, 8],
                [attributes.aerial, 4],
            ]);
        default:
            return weightedAverage([
                [attributes.pace, 10],
                [attributes.stamina, 10],
                [attributes.strength, 10],
                [attributes.passing, 10],
                [attributes.shooting, 10],
                [attributes.tackling, 10],
                [attributes.dribbling, 10],
                [attributes.defending, 10],
                [attributes.positioning, 10],
                [attributes.vision, 5],
                [attributes.decisions, 5],
            ]);
    }
}

function criticalPenalty(player: PlayerData, position: string): number {
    const attributes = player.attributes;
    let criticalMin = 50;

    switch (exactPosition(position)) {
        case "Goalkeeper":
            criticalMin = Math.min(attributes.handling, attributes.reflexes, attributes.positioning);
            break;
        case "RightBack":
        case "LeftBack":
            criticalMin = Math.min(attributes.tackling, attributes.defending, attributes.positioning);
            break;
        case "CenterBack":
            criticalMin = Math.min(attributes.defending, attributes.tackling, attributes.positioning);
            break;
        case "RightWingBack":
        case "LeftWingBack":
            criticalMin = Math.min(attributes.pace, attributes.stamina, attributes.tackling);
            break;
        case "DefensiveMidfielder":
            criticalMin = Math.min(attributes.tackling, attributes.positioning, attributes.passing);
            break;
        case "CentralMidfielder":
            criticalMin = Math.min(attributes.passing, attributes.vision, attributes.decisions);
            break;
        case "AttackingMidfielder":
            criticalMin = Math.min(attributes.vision, attributes.passing, attributes.dribbling);
            break;
        case "RightMidfielder":
        case "LeftMidfielder":
            criticalMin = Math.min(attributes.pace, attributes.passing, attributes.stamina);
            break;
        case "RightWinger":
        case "LeftWinger":
            criticalMin = Math.min(attributes.pace, attributes.dribbling, attributes.passing);
            break;
        case "Striker":
            criticalMin = Math.min(attributes.shooting, attributes.positioning, attributes.decisions);
            break;
    }

    return criticalMin >= 45 ? 0 : (45 - criticalMin) * 0.6;
}

export function calcOvr(player: PlayerData, position?: string): number {
    const targetPosition = position ? exactPosition(position) : primaryPosition(player);
    const weightedScore = weightedPositionScore(player, targetPosition);
    const penalty = criticalPenalty(player, targetPosition);
    const fitPenalty = position ? compatibilityPenalty(player, targetPosition) : 0;
    const sidePenalty = position ? footednessPenalty(player, targetPosition) : 0;

    return Math.round(
        Math.max(1, Math.min(99, weightedScore - penalty - fitPenalty - sidePenalty)),
    );
}

export function positionBadgeVariant(pos: string): "accent" | "primary" | "success" | "danger" {
    switch (pos) {
        case "Goalkeeper":
            return "accent";
        case "Defender":
        case "RightBack":
        case "CenterBack":
        case "LeftBack":
        case "RightWingBack":
        case "LeftWingBack":
            return "primary";
        case "Midfielder":
        case "DefensiveMidfielder":
        case "CentralMidfielder":
        case "AttackingMidfielder":
        case "RightMidfielder":
        case "LeftMidfielder":
            return "success";
        case "Forward":
        case "RightWinger":
        case "LeftWinger":
        case "Striker":
            return "danger";
        default:
            return "primary";
    }
}
