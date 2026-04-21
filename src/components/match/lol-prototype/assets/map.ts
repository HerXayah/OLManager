import { STRUCTURES_LAYOUT } from "../../../../lib/lolMapLayout";
import fullWalls from "./lol_walls_full.json";
import type { LaneId, TeamId, Vec2 } from "../engine/types";

export const MAP_IMAGE_PATH = "/map.webp";

export const BASE_POSITION: Record<TeamId, Vec2> = {
  blue: { x: 0.115, y: 0.882 },
  red: { x: 0.891, y: 0.117 },
};

export const ROLE_OFFSET = {
  TOP: { x: -0.014, y: -0.012 },
  JGL: { x: 0.014, y: -0.01 },
  MID: { x: 0.011, y: 0.011 },
  ADC: { x: -0.012, y: 0.018 },
  SUP: { x: 0.004, y: 0.021 },
} as const;

export const LANE_PATH_BLUE: Record<LaneId, Vec2[]> = {
  top: [
    { x: 0.12, y: 0.88 },
    { x: 0.1, y: 0.73 },
    { x: 0.09, y: 0.56 },
    { x: 0.085, y: 0.3 },
    { x: 0.28, y: 0.08 },
    { x: 0.53, y: 0.08 },
    { x: 0.89, y: 0.12 },
  ],
  mid: [
    { x: 0.12, y: 0.88 },
    { x: 0.22, y: 0.78 },
    { x: 0.34, y: 0.67 },
    { x: 0.46, y: 0.54 },
    { x: 0.58, y: 0.42 },
    { x: 0.7, y: 0.3 },
    { x: 0.89, y: 0.12 },
  ],
  bot: [
    { x: 0.12, y: 0.88 },
    { x: 0.29, y: 0.9 },
    { x: 0.49, y: 0.91 },
    { x: 0.72, y: 0.92 },
    { x: 0.9, y: 0.7 },
    { x: 0.91, y: 0.45 },
    { x: 0.89, y: 0.12 },
  ],
};

export const JUNGLE_ROUTE: Record<TeamId, Vec2[]> = {
  blue: [
    { x: 0.25, y: 0.46 },
    { x: 0.26, y: 0.56 },
    { x: 0.48, y: 0.64 },
    { x: 0.53, y: 0.74 },
  ],
  red: [
    { x: 0.48, y: 0.26 },
    { x: 0.53, y: 0.35 },
    { x: 0.75, y: 0.43 },
    { x: 0.75, y: 0.54 },
  ],
};

export const OBJECTIVES = {
  dragon: { x: 0.674, y: 0.703 },
  baron: { x: 0.327, y: 0.298 },
};

export const FALLBACK_WALLS = [
  { id: "baron", points: [{ x: 0.249, y: 0.254 }, { x: 0.306, y: 0.21 }, { x: 0.387, y: 0.271 }, { x: 0.42, y: 0.358 }, { x: 0.321, y: 0.337 }] },
  { id: "dragon", points: [{ x: 0.588, y: 0.656 }, { x: 0.609, y: 0.628 }, { x: 0.676, y: 0.659 }, { x: 0.755, y: 0.742 }, { x: 0.692, y: 0.777 }, { x: 0.636, y: 0.758 }] },
  { id: "river-top", points: [{ x: 0.418, y: 0.275 }, { x: 0.428, y: 0.245 }, { x: 0.492, y: 0.29 }, { x: 0.458, y: 0.327 }, { x: 0.428, y: 0.314 }] },
  { id: "river-bot", points: [{ x: 0.489, y: 0.565 }, { x: 0.536, y: 0.607 }, { x: 0.57, y: 0.609 }, { x: 0.505, y: 0.555 }] },
];

export function getWalls() {
  const parsed = (fullWalls as { walls?: Array<{ id: string; points: Vec2[] }> })?.walls;
  return parsed && parsed.length >= 40 ? parsed : FALLBACK_WALLS;
}

export function getStructures() {
  return STRUCTURES_LAYOUT.map((s) => ({
    id: s.id,
    lane: s.lane,
    team: s.side,
    kind: s.icon.includes("nexus") ? "nexus" : s.icon.includes("inhibitor") ? "inhib" : "tower",
    pos: { x: s.x, y: s.y },
  }));
}
