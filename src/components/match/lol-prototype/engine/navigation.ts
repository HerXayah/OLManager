import type { Vec2 } from "./types";

type Wall = { id: string; points: Vec2[] };
type PathNode = { cx: number; cy: number; g: number; f: number; p: string | null };

function clamp(v: number, min: number, max: number) {
  return Math.max(min, Math.min(max, v));
}

function pointInPolygon(point: Vec2, points: Vec2[]) {
  let inside = false;
  for (let i = 0, j = points.length - 1; i < points.length; j = i, i += 1) {
    const xi = points[i].x;
    const yi = points[i].y;
    const xj = points[j].x;
    const yj = points[j].y;
    const hit = yi > point.y !== yj > point.y && point.x < ((xj - xi) * (point.y - yi)) / (yj - yi + 1e-9) + xi;
    if (hit) inside = !inside;
  }
  return inside;
}

export class NavGrid {
  private blocked: Uint8Array;
  constructor(private walls: Wall[], private gridSize = 120) {
    this.blocked = new Uint8Array(gridSize * gridSize);
    this.buildBlocked();
  }

  private idx(cx: number, cy: number) {
    return cy * this.gridSize + cx;
  }
  private inBounds(cx: number, cy: number) {
    return cx >= 0 && cy >= 0 && cx < this.gridSize && cy < this.gridSize;
  }
  private toCell(v: number) {
    return clamp(Math.floor(v * this.gridSize), 0, this.gridSize - 1);
  }
  private toNorm(c: number) {
    return (c + 0.5) / this.gridSize;
  }
  private isBlockedCell(cx: number, cy: number) {
    if (!this.inBounds(cx, cy)) return true;
    return this.blocked[this.idx(cx, cy)] === 1;
  }

  private buildBlocked() {
    for (let y = 0; y < this.gridSize; y += 1) {
      for (let x = 0; x < this.gridSize; x += 1) {
        const p = { x: this.toNorm(x), y: this.toNorm(y) };
        this.blocked[this.idx(x, y)] = this.walls.some((w) => pointInPolygon(p, w.points)) ? 1 : 0;
      }
    }
  }

  private nearestFreeCell(cx: number, cy: number) {
    if (!this.isBlockedCell(cx, cy)) return { cx, cy };
    const q = [{ cx, cy }];
    const seen = new Set([`${cx},${cy}`]);
    const dirs: Array<[number, number]> = [[1, 0], [-1, 0], [0, 1], [0, -1], [1, 1], [1, -1], [-1, 1], [-1, -1]];
    while (q.length) {
      const cur = q.shift()!;
      if (!this.isBlockedCell(cur.cx, cur.cy)) return cur;
      for (const [dx, dy] of dirs) {
        const nx = cur.cx + dx;
        const ny = cur.cy + dy;
        const k = `${nx},${ny}`;
        if (!this.inBounds(nx, ny) || seen.has(k)) continue;
        seen.add(k);
        q.push({ cx: nx, cy: ny });
      }
    }
    return { cx, cy };
  }

  findPath(start: Vec2, end: Vec2): Vec2[] {
    const s = this.nearestFreeCell(this.toCell(start.x), this.toCell(start.y));
    const e = this.nearestFreeCell(this.toCell(end.x), this.toCell(end.y));
    const key = (cx: number, cy: number) => `${cx},${cy}`;
    const h = (cx: number, cy: number) => Math.hypot(e.cx - cx, e.cy - cy);
    const open = [key(s.cx, s.cy)];
    const nodes = new Map<string, PathNode>();
    nodes.set(key(s.cx, s.cy), { cx: s.cx, cy: s.cy, g: 0, f: h(s.cx, s.cy), p: null });
    const closed = new Set<string>();
    const dirs: Array<[number, number, number]> = [
      [1, 0, 1],
      [-1, 0, 1],
      [0, 1, 1],
      [0, -1, 1],
      [1, 1, 1.414],
      [-1, -1, 1.414],
      [1, -1, 1.414],
      [-1, 1, 1.414],
    ];

    while (open.length) {
      open.sort((a, b) => nodes.get(a)!.f - nodes.get(b)!.f);
      const ck = open.shift()!;
      const cur = nodes.get(ck)!;
      if (cur.cx === e.cx && cur.cy === e.cy) {
        const out: Vec2[] = [];
        let at: string | null = ck;
        while (at) {
          const n: PathNode = nodes.get(at)!;
          out.push({ x: this.toNorm(n.cx), y: this.toNorm(n.cy) });
          at = n.p;
        }
        return this.smoothPath(out.reverse());
      }
      closed.add(ck);
      for (const [dx, dy, cost] of dirs) {
        const nx = cur.cx + Number(dx);
        const ny = cur.cy + Number(dy);
        const isDiagonal = dx !== 0 && dy !== 0;
        if (isDiagonal) {
          const sideXBlocked = this.isBlockedCell(cur.cx + Number(dx), cur.cy);
          const sideYBlocked = this.isBlockedCell(cur.cx, cur.cy + Number(dy));
          if (sideXBlocked || sideYBlocked) continue;
        }
        if (this.isBlockedCell(nx, ny)) continue;
        const nk = key(nx, ny);
        if (closed.has(nk)) continue;
        const g = cur.g + Number(cost);
        const f = g + h(nx, ny);
        const old = nodes.get(nk);
        if (!old || g < old.g) {
          nodes.set(nk, { cx: nx, cy: ny, g, f, p: ck });
          if (!open.includes(nk)) open.push(nk);
        }
      }
    }

    return [start, end];
  }

  private smoothPath(path: Vec2[]) {
    if (path.length <= 2) return path;
    const out = [path[0]];
    let i = 0;
    while (i < path.length - 1) {
      let j = path.length - 1;
      for (; j > i + 1; j -= 1) {
        if (this.hasLineOfSight(path[i], path[j])) break;
      }
      out.push(path[j]);
      i = j;
    }
    return out;
  }

  private hasLineOfSight(a: Vec2, b: Vec2) {
    const cellAx = this.toCell(a.x);
    const cellAy = this.toCell(a.y);
    const cellBx = this.toCell(b.x);
    const cellBy = this.toCell(b.y);
    const cellDistance = Math.hypot(cellBx - cellAx, cellBy - cellAy);
    const steps = Math.max(6, Math.ceil(cellDistance * 2));
    for (let i = 0; i <= steps; i += 1) {
      const t = i / steps;
      const p = { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
      if (this.isBlockedCell(this.toCell(p.x), this.toCell(p.y))) return false;
    }
    return true;
  }
}
