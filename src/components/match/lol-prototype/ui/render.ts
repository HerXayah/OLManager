import { MAP_IMAGE_PATH } from "../assets/map";
import type { MatchState, NeutralTimerKey } from "../engine/types";
import {
  JUNGLE_CAMP_ICON_PATH,
  JUNGLE_CAMPS_LAYOUT,
  LOL_MAP_JUNGLE_ICON_SIZE,
  LOL_MAP_OBJECTIVE_ICON_SIZE,
  LOL_MAP_STRUCTURE_ICON_SIZE,
  NEUTRAL_OBJECTIVE_ICON_PATH,
  STRUCTURE_ICON_PATH,
  STRUCTURES_LAYOUT,
} from "../../../../lib/lolMapLayout";

let cachedImage: HTMLImageElement | null = null;
const iconCache = new Map<string, HTMLImageElement>();
const structureMetaById = new Map(STRUCTURES_LAYOUT.map((s) => [s.id, s]));
const CHAMPION_DRAW_RADIUS = 11.8;
const CHAMPION_HP_BAR_WIDTH = 26;
const CHAMPION_HP_BAR_Y_OFFSET = 15;
const ENTITY_HP_BAR_HEIGHT = 3;
const ENTITY_HP_BAR_BG = "rgba(0,0,0,0.62)";

const CAMP_LAYOUT_TO_TIMER_KEY: Partial<Record<string, NeutralTimerKey>> = {
  "blue-blue-buff": "blue-buff-blue",
  "red-blue-buff": "blue-buff-red",
  "blue-red-buff": "red-buff-blue",
  "red-red-buff": "red-buff-red",
  "blue-wolves": "wolves-blue",
  "red-wolves": "wolves-red",
  "blue-raptors": "raptors-blue",
  "red-raptors": "raptors-red",
  "blue-gromp": "gromp-blue",
  "red-gromp": "gromp-red",
  "blue-krugs": "krugs-blue",
  "red-krugs": "krugs-red",
  "river-scuttle-top": "scuttle-top",
  "river-scuttle-bot": "scuttle-bot",
};

const NEUTRAL_TIMER_ICON: Partial<Record<NeutralTimerKey, keyof typeof NEUTRAL_OBJECTIVE_ICON_PATH>> = {
  dragon: "dragon",
  elder: "dragon",
  baron: "baron",
  herald: "riftherald",
  voidgrubs: "grub",
};

function getMapImage() {
  if (cachedImage) return cachedImage;
  const img = new Image();
  img.src = MAP_IMAGE_PATH;
  cachedImage = img;
  return img;
}

function getIcon(src: string) {
  const cached = iconCache.get(src);
  if (cached) return cached;
  const img = new Image();
  img.src = src;
  iconCache.set(src, img);
  return img;
}

function drawIcon(ctx: CanvasRenderingContext2D, src: string, x: number, y: number, size: number) {
  const icon = getIcon(src);
  if (!icon.complete || icon.naturalWidth <= 0 || icon.naturalHeight <= 0) return false;
  const half = size / 2;
  ctx.drawImage(icon, x - half, y - half, size, size);
  return true;
}

function drawHpBar(ctx: CanvasRenderingContext2D, x: number, y: number, width: number, ratio: number, color: string) {
  const clampedRatio = Number.isFinite(ratio) ? Math.max(0, Math.min(1, ratio)) : 0;
  ctx.fillStyle = ENTITY_HP_BAR_BG;
  ctx.fillRect(x - width / 2, y, width, ENTITY_HP_BAR_HEIGHT);
  ctx.fillStyle = color;
  ctx.fillRect(x - width / 2, y, width * clampedRatio, ENTITY_HP_BAR_HEIGHT);
}

function championIconUrl(championId: string | undefined) {
  if (!championId) return null;
  return `https://ddragon.leagueoflegends.com/cdn/14.24.1/img/champion/${championId}.png`;
}

function initials(name: string): string {
  const chunks = name.trim().split(/\s+/).filter(Boolean);
  if (chunks.length >= 2) return `${chunks[0][0]}${chunks[1][0]}`.toUpperCase();
  return name.slice(0, 2).toUpperCase();
}

export function renderSimulation(
  canvas: HTMLCanvasElement,
  state: MatchState,
  walls: Array<{ id: string; points: { x: number; y: number }[] }>,
  championByPlayerId?: Record<string, string>,
) {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const { width, height } = canvas;
  ctx.clearRect(0, 0, width, height);

  const map = getMapImage();
  if (map.complete) {
    ctx.drawImage(map, 0, 0, width, height);
  } else {
    ctx.fillStyle = "#0b1226";
    ctx.fillRect(0, 0, width, height);
  }

  if (state.showWalls) {
    ctx.strokeStyle = "rgba(56, 189, 248, 0.45)";
    ctx.fillStyle = "rgba(2, 132, 199, 0.12)";
    ctx.lineWidth = 1.5;
    walls.forEach((w) => {
      if (!w.points.length) return;
      ctx.beginPath();
      ctx.moveTo(w.points[0].x * width, w.points[0].y * height);
      for (let i = 1; i < w.points.length; i += 1) {
        ctx.lineTo(w.points[i].x * width, w.points[i].y * height);
      }
      ctx.closePath();
      ctx.fill();
      ctx.stroke();
    });
  }

  JUNGLE_CAMPS_LAYOUT.forEach((camp) => {
    const timerKey = CAMP_LAYOUT_TO_TIMER_KEY[camp.id];
    if (!timerKey) return;
    const timer = state.neutralTimers.entities[timerKey];
    if (!timer?.alive) return;
    const px = camp.x * width;
    const py = camp.y * height;
    if (!drawIcon(ctx, JUNGLE_CAMP_ICON_PATH[camp.icon], px, py, LOL_MAP_JUNGLE_ICON_SIZE)) {
      ctx.beginPath();
      ctx.fillStyle = "rgba(163, 230, 53, 0.8)";
      ctx.arc(px, py, 3, 0, Math.PI * 2);
      ctx.fill();
    }
    drawHpBar(ctx, px, py - LOL_MAP_JUNGLE_ICON_SIZE / 2 - 5, 26, timer.hp / timer.maxHp, "#84cc16");
  });

  Object.values(state.neutralTimers.entities)
    .filter((timer) => timer.alive && NEUTRAL_TIMER_ICON[timer.key])
    .forEach((timer) => {
      const iconType = NEUTRAL_TIMER_ICON[timer.key];
      if (!iconType) return;
      const px = timer.pos.x * width;
      const py = timer.pos.y * height;
      if (!drawIcon(ctx, NEUTRAL_OBJECTIVE_ICON_PATH[iconType], px, py, LOL_MAP_OBJECTIVE_ICON_SIZE)) {
        ctx.beginPath();
        ctx.fillStyle = "rgba(250, 204, 21, 0.85)";
        ctx.arc(px, py, 5.2, 0, Math.PI * 2);
        ctx.fill();
      }
      drawHpBar(ctx, px, py - LOL_MAP_OBJECTIVE_ICON_SIZE / 2 - 6, 36, timer.hp / timer.maxHp, "#f59e0b");
    });

  state.structures.filter((s) => s.alive).forEach((s) => {
    const px = s.pos.x * width;
    const py = s.pos.y * height;
    const structureMeta = structureMetaById.get(s.id);
    if (!(structureMeta && drawIcon(ctx, STRUCTURE_ICON_PATH[structureMeta.icon], px, py, LOL_MAP_STRUCTURE_ICON_SIZE))) {
      ctx.beginPath();
      ctx.fillStyle = s.team === "blue" ? "#38bdf8" : "#fb7185";
      ctx.strokeStyle = "#111827";
      ctx.lineWidth = 1.5;
      ctx.arc(px, py, s.kind === "nexus" ? 6.5 : 4.5, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }
    drawHpBar(ctx, px, py - LOL_MAP_STRUCTURE_ICON_SIZE / 2 - 5, 30, s.hp / s.maxHp, s.team === "blue" ? "#22d3ee" : "#fb7185");
  });

  state.minions.forEach((m) => {
    ctx.beginPath();
    ctx.fillStyle = m.team === "blue" ? "#67e8f9" : "#fda4af";
    ctx.arc(m.pos.x * width, m.pos.y * height, 2.1, 0, Math.PI * 2);
    ctx.fill();

    ctx.fillStyle = "rgba(0,0,0,0.6)";
    ctx.fillRect(m.pos.x * width - 4, m.pos.y * height - 7, 8, 2);
    ctx.fillStyle = "#22c55e";
    ctx.fillRect(m.pos.x * width - 4, m.pos.y * height - 7, 8 * (m.hp / m.maxHp), 2);
  });

  state.champions.forEach((c) => {
    ctx.save();
    ctx.globalAlpha = c.alive ? 1 : 0.35;
    const px = c.pos.x * width;
    const py = c.pos.y * height;
    ctx.beginPath();
    ctx.fillStyle = c.team === "blue" ? "#0ea5e9" : "#e11d48";
    ctx.strokeStyle = "#f8fafc";
    ctx.lineWidth = 2;
    ctx.arc(px, py, CHAMPION_DRAW_RADIUS, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();

    const championId = championByPlayerId?.[c.id];
    const icon = championIconUrl(championId ?? undefined);
    let drewChampionIcon = false;
    if (icon) {
      const championImg = getIcon(icon);
      if (championImg.complete && championImg.naturalWidth > 0 && championImg.naturalHeight > 0) {
        ctx.save();
        ctx.beginPath();
        ctx.arc(px, py, CHAMPION_DRAW_RADIUS - 1.4, 0, Math.PI * 2);
        ctx.clip();
        ctx.drawImage(
          championImg,
          px - (CHAMPION_DRAW_RADIUS - 1.4),
          py - (CHAMPION_DRAW_RADIUS - 1.4),
          (CHAMPION_DRAW_RADIUS - 1.4) * 2,
          (CHAMPION_DRAW_RADIUS - 1.4) * 2,
        );
        ctx.restore();
        drewChampionIcon = true;
      }
    }

    ctx.fillStyle = "rgba(0,0,0,0.7)";
    ctx.fillRect(px - CHAMPION_HP_BAR_WIDTH / 2, py - CHAMPION_HP_BAR_Y_OFFSET, CHAMPION_HP_BAR_WIDTH, 3);
    ctx.fillStyle = c.team === "blue" ? "#22d3ee" : "#fb7185";
    ctx.fillRect(
      px - CHAMPION_HP_BAR_WIDTH / 2,
      py - CHAMPION_HP_BAR_Y_OFFSET,
      CHAMPION_HP_BAR_WIDTH * (c.hp / c.maxHp),
      3,
    );

    if (!drewChampionIcon) {
      ctx.fillStyle = "#e2e8f0";
      ctx.font = "7px Inter, sans-serif";
      ctx.textAlign = "center";
      ctx.fillText(initials(c.name), px, py + 2.4);
    }
    ctx.restore();
  });
}
