const MAP_SIZE = 1024;
const TEAM_BLUE = "blue";
const TEAM_RED = "red";
const ROLES = ["TOP", "JGL", "MID", "ADC", "SUP"];

const TEAM_STYLE = { blue: "#3b82f6", red: "#ef4444" };

const BASE_POSITION = {
  blue: { x: 0.115, y: 0.882 },
  red: { x: 0.891, y: 0.117 },
};

const ROLE_OFFSET = {
  TOP: { x: -0.014, y: -0.012 },
  JGL: { x: 0.014, y: -0.01 },
  MID: { x: 0.011, y: 0.011 },
  ADC: { x: -0.012, y: 0.018 },
  SUP: { x: 0.004, y: 0.021 },
};

const LANE_PATH_BLUE = {
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

const LANE_STAGE_INDEX = {
  blue: {
    top: { safe: 2, hold: 3, push: 4 },
    mid: { safe: 2, hold: 3, push: 4 },
    bot: { safe: 2, hold: 3, push: 4 },
  },
  red: {
    top: { safe: 2, hold: 3, push: 4 },
    mid: { safe: 2, hold: 3, push: 4 },
    bot: { safe: 2, hold: 3, push: 4 },
  },
};

const JUNGLE_POINTS = {
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

const DRAGON_PIT = { x: 0.674, y: 0.703 };
const BARON_PIT = { x: 0.327, y: 0.298 };

const FALLBACK_WALLS = [
  { id: "baron", points: [{ x: 0.249, y: 0.254 }, { x: 0.306, y: 0.21 }, { x: 0.387, y: 0.271 }, { x: 0.42, y: 0.358 }, { x: 0.321, y: 0.337 }] },
  { id: "dragon", points: [{ x: 0.588, y: 0.656 }, { x: 0.609, y: 0.628 }, { x: 0.676, y: 0.659 }, { x: 0.755, y: 0.742 }, { x: 0.692, y: 0.777 }, { x: 0.636, y: 0.758 }] },
  { id: "river-top", points: [{ x: 0.418, y: 0.275 }, { x: 0.428, y: 0.245 }, { x: 0.492, y: 0.29 }, { x: 0.458, y: 0.327 }, { x: 0.428, y: 0.314 }] },
  { id: "river-bot", points: [{ x: 0.489, y: 0.565 }, { x: 0.536, y: 0.607 }, { x: 0.57, y: 0.609 }, { x: 0.505, y: 0.555 }] },
  { id: "mid-blue", points: [{ x: 0.462, y: 0.622 }, { x: 0.447, y: 0.645 }, { x: 0.462, y: 0.672 }, { x: 0.406, y: 0.639 }, { x: 0.469, y: 0.582 }] },
  { id: "mid-red", points: [{ x: 0.475, y: 0.38 }, { x: 0.475, y: 0.352 }, { x: 0.535, y: 0.378 }, { x: 0.6, y: 0.355 }, { x: 0.536, y: 0.426 }] },
];

const STRUCTURES = [
  { id: "blue-top-1", team: TEAM_BLUE, lane: "top", x: 0.072, y: 0.284, hp: 1400 },
  { id: "blue-top-2", team: TEAM_BLUE, lane: "top", x: 0.099, y: 0.553, hp: 1500 },
  { id: "blue-mid-1", team: TEAM_BLUE, lane: "mid", x: 0.402, y: 0.576, hp: 1400 },
  { id: "blue-mid-2", team: TEAM_BLUE, lane: "mid", x: 0.347, y: 0.671, hp: 1500 },
  { id: "blue-bot-1", team: TEAM_BLUE, lane: "bot", x: 0.721, y: 0.923, hp: 1400 },
  { id: "blue-bot-2", team: TEAM_BLUE, lane: "bot", x: 0.472, y: 0.896, hp: 1500 },
  { id: "blue-nexus", team: TEAM_BLUE, lane: "base", x: 0.115, y: 0.882, hp: 2300 },
  { id: "red-top-1", team: TEAM_RED, lane: "top", x: 0.275, y: 0.072, hp: 1400 },
  { id: "red-top-2", team: TEAM_RED, lane: "top", x: 0.533, y: 0.082, hp: 1500 },
  { id: "red-mid-1", team: TEAM_RED, lane: "mid", x: 0.657, y: 0.332, hp: 1400 },
  { id: "red-mid-2", team: TEAM_RED, lane: "mid", x: 0.596, y: 0.441, hp: 1500 },
  { id: "red-bot-1", team: TEAM_RED, lane: "bot", x: 0.93, y: 0.706, hp: 1400 },
  { id: "red-bot-2", team: TEAM_RED, lane: "bot", x: 0.902, y: 0.449, hp: 1500 },
  { id: "red-nexus", team: TEAM_RED, lane: "base", x: 0.891, y: 0.117, hp: 2300 },
].map((s) => ({ ...s, alive: true, towerAttackAt: 0, towerCadence: 1.0 }));

const canvas = document.getElementById("mapCanvas");
const ctx = canvas.getContext("2d");
const scoreboardEl = document.getElementById("scoreboard");
const eventsListEl = document.getElementById("eventsList");

const mapImage = new Image();
mapImage.src = "./map.webp";
mapImage.onerror = () => {
  mapImage.src = "./map.png";
};

function clamp(v, min, max) {
  return Math.max(min, Math.min(max, v));
}

function dist(a, b) {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

function formatTime(seconds) {
  const m = Math.floor(seconds / 60)
    .toString()
    .padStart(2, "0");
  const s = Math.floor(seconds % 60)
    .toString()
    .padStart(2, "0");
  return `${m}:${s}`;
}

function pointInPolygon(point, polygon) {
  let inside = false;
  for (let i = 0, j = polygon.points.length - 1; i < polygon.points.length; j = i, i += 1) {
    const xi = polygon.points[i].x;
    const yi = polygon.points[i].y;
    const xj = polygon.points[j].x;
    const yj = polygon.points[j].y;
    const intersect = yi > point.y !== yj > point.y && point.x < ((xj - xi) * (point.y - yi)) / (yj - yi + 1e-9) + xi;
    if (intersect) inside = !inside;
  }
  return inside;
}

class NavGrid {
  constructor(walls, gridSize = 120) {
    this.walls = walls;
    this.gridSize = gridSize;
    this.blocked = new Uint8Array(gridSize * gridSize);
    this.buildBlocked();
  }

  idx(cx, cy) {
    return cy * this.gridSize + cx;
  }

  inBounds(cx, cy) {
    return cx >= 0 && cx < this.gridSize && cy >= 0 && cy < this.gridSize;
  }

  normToCell(v) {
    return clamp(Math.floor(v * this.gridSize), 0, this.gridSize - 1);
  }

  cellToNorm(c) {
    return (c + 0.5) / this.gridSize;
  }

  isBlockedCell(cx, cy) {
    if (!this.inBounds(cx, cy)) return true;
    return this.blocked[this.idx(cx, cy)] === 1;
  }

  isBlockedPos(x, y) {
    return this.isBlockedCell(this.normToCell(x), this.normToCell(y));
  }

  buildBlocked() {
    for (let y = 0; y < this.gridSize; y += 1) {
      for (let x = 0; x < this.gridSize; x += 1) {
        const p = { x: this.cellToNorm(x), y: this.cellToNorm(y) };
        this.blocked[this.idx(x, y)] = this.walls.some((w) => pointInPolygon(p, w)) ? 1 : 0;
      }
    }
  }

  nearestFreeCell(cx, cy) {
    if (!this.isBlockedCell(cx, cy)) return { cx, cy };
    const q = [{ cx, cy }];
    const seen = new Set([`${cx},${cy}`]);
    const dirs = [
      [1, 0],
      [-1, 0],
      [0, 1],
      [0, -1],
      [1, 1],
      [1, -1],
      [-1, 1],
      [-1, -1],
    ];
    while (q.length > 0) {
      const cur = q.shift();
      if (!this.isBlockedCell(cur.cx, cur.cy)) return cur;
      for (const [dx, dy] of dirs) {
        const nx = cur.cx + dx;
        const ny = cur.cy + dy;
        const key = `${nx},${ny}`;
        if (!this.inBounds(nx, ny) || seen.has(key)) continue;
        seen.add(key);
        q.push({ cx: nx, cy: ny });
      }
    }
    return { cx, cy };
  }

  findPath(start, end) {
    const startCell = this.nearestFreeCell(this.normToCell(start.x), this.normToCell(start.y));
    const endCell = this.nearestFreeCell(this.normToCell(end.x), this.normToCell(end.y));
    const key = (cx, cy) => `${cx},${cy}`;
    const h = (cx, cy) => Math.hypot(endCell.cx - cx, endCell.cy - cy);
    const open = [key(startCell.cx, startCell.cy)];
    const nodes = new Map();
    const closed = new Set();
    nodes.set(key(startCell.cx, startCell.cy), { cx: startCell.cx, cy: startCell.cy, g: 0, f: h(startCell.cx, startCell.cy), p: null });
    const dirs = [
      [1, 0, 1],
      [-1, 0, 1],
      [0, 1, 1],
      [0, -1, 1],
      [1, 1, 1.414],
      [1, -1, 1.414],
      [-1, 1, 1.414],
      [-1, -1, 1.414],
    ];

    while (open.length > 0) {
      open.sort((a, b) => nodes.get(a).f - nodes.get(b).f);
      const ck = open.shift();
      const cur = nodes.get(ck);
      if (cur.cx === endCell.cx && cur.cy === endCell.cy) {
        const out = [];
        let t = cur;
        while (t) {
          out.push({ x: this.cellToNorm(t.cx), y: this.cellToNorm(t.cy) });
          t = t.p;
        }
        return out.reverse();
      }
      closed.add(ck);

      for (const [dx, dy, cost] of dirs) {
        const nx = cur.cx + dx;
        const ny = cur.cy + dy;
        if (this.isBlockedCell(nx, ny)) continue;
        const nk = key(nx, ny);
        if (closed.has(nk)) continue;
        const g = cur.g + cost;
        const f = g + h(nx, ny);
        const ex = nodes.get(nk);
        if (!ex || g < ex.g) {
          nodes.set(nk, { cx: nx, cy: ny, g, f, p: cur });
          if (!open.includes(nk)) open.push(nk);
        }
      }
    }
    return [start, end];
  }
}

class MatchSimulator {
  constructor(nav, walls) {
    this.nav = nav;
    this.walls = walls;
    this.time = 0;
    this.lastTick = performance.now();
    this.speed = 3;
    this.events = [];
    this.champions = [];
    this.minions = [];
    this.structures = STRUCTURES.map((s) => ({ ...s }));
    this.spawnWaveAt = 5;
    this.nextDragonAt = 5 * 60;
    this.nextBaronAt = 20 * 60;
    this.winner = null;
    this.score = {
      blue: { kills: 0, towers: 0, dragons: 0, barons: 0 },
      red: { kills: 0, towers: 0, dragons: 0, barons: 0 },
    };

    this.createTeams();
    this.logEvent("Arranca la partida.");
  }

  logEvent(text) {
    this.events.unshift({ t: formatTime(this.time), text });
    this.events = this.events.slice(0, 18);
  }

  roleLane(role) {
    if (role === "TOP") return "top";
    if (role === "MID") return "mid";
    if (role === "ADC" || role === "SUP") return "bot";
    return "mid";
  }

  getLanePath(team, lane) {
    return team === TEAM_BLUE ? LANE_PATH_BLUE[lane] : [...LANE_PATH_BLUE[lane]].reverse();
  }

  getLaneStageNode(team, lane, stage) {
    const idx = LANE_STAGE_INDEX[team][lane][stage];
    return this.getLanePath(team, lane)[idx];
  }

  createTeams() {
    const createChamp = (team, role) => {
      const b = BASE_POSITION[team];
      const o = ROLE_OFFSET[role];
      return {
        id: `${team}-${role}`,
        team,
        role,
        homeLane: this.roleLane(role),
        x: b.x + o.x,
        y: b.y + o.y,
        hp: 140,
        maxHp: 140,
        alive: true,
        state: "idle",
        path: [],
        pathIndex: 0,
        speed: 0.043 + Math.random() * 0.01,
        target: null,
        decisionAt: 2 + Math.random() * 2,
        recallUntil: 0,
        disengageUntil: 0,
        combatLockUntil: 0,
        attackDamage: 15 + Math.random() * 4,
        attackCadence: 0.9 + Math.random() * 0.2,
        attackAt: 0,
        respawnAt: 0,
      };
    };

    for (const role of ROLES) {
      this.champions.push(createChamp(TEAM_BLUE, role));
      this.champions.push(createChamp(TEAM_RED, role));
    }
  }

  laneProgress(lane, team) {
    const mine = this.minions.filter((m) => m.team === team && m.lane === lane);
    const theirs = this.minions.filter((m) => m.team !== team && m.lane === lane);
    const pathLen = this.getLanePath(team, lane).length - 1;
    const progress = (u) => clamp((u.pathIndex - 1) / pathLen, 0, 1);
    const my = mine.length ? mine.reduce((a, m) => a + progress(m), 0) / mine.length : 0;
    const en = theirs.length ? theirs.reduce((a, m) => a + progress(m), 0) / theirs.length : 0;
    return my - en;
  }

  laneHasFrontline(team, lane) {
    return this.minions.filter((m) => m.team === team && m.lane === lane && m.pathIndex >= 3).length >= 2;
  }

  shouldJoinObjective(champ, objectiveType) {
    if (objectiveType === "dragon") {
      if (champ.role === "JGL" || champ.role === "MID" || champ.role === "SUP") return true;
      if (champ.role === "ADC") return this.time > 7 * 60;
      return this.time > 22 * 60;
    }
    if (objectiveType === "baron") {
      return champ.role !== "TOP" || this.time > 25 * 60;
    }
    return false;
  }

  chooseTeamCall(team) {
    const alive = this.champions.filter((c) => c.team === team && c.alive).length;
    if (alive < 3) return { type: "farm" };
    if (this.nextDragonAt - this.time < 30) return { type: "dragon" };
    if (this.time > 20 * 60 && this.nextBaronAt - this.time < 35) return { type: "baron" };
    return { type: "farm" };
  }

  getEnemyStructureInLane(team, lane) {
    return this.structures.find((s) => s.alive && s.team !== team && s.lane === lane) || null;
  }

  isSafeToHitTower(champ, structure) {
    if (!structure || !structure.alive) return false;
    const cover = this.minions.filter((m) => m.team === champ.team && dist(m, structure) < 0.085).length;
    const allies = this.champions.filter((c) => c.alive && c.team === champ.team && dist(c, structure) < 0.1).length;
    const enemies = this.champions.filter((c) => c.alive && c.team !== champ.team && dist(c, structure) < 0.1).length;
    if (cover < 2) return false;
    if (champ.hp / champ.maxHp < 0.55) return false;
    return allies >= enemies;
  }

  shouldTakeTrade(champ, enemy) {
    const hp = champ.hp / champ.maxHp;
    const ehp = enemy.hp / enemy.maxHp;
    const allies = this.champions.filter((c) => c.alive && c.team === champ.team && dist(c, champ) < 0.08).length;
    const enemies = this.champions.filter((c) => c.alive && c.team !== champ.team && dist(c, champ) < 0.08).length;
    if (hp < 0.42) return false;
    if (allies < enemies) return false;
    if (ehp > hp + 0.15) return false;
    return Math.random() < 0.22;
  }

  shouldDisengage(champ, enemy) {
    const hp = champ.hp / champ.maxHp;
    const ehp = enemy.hp / enemy.maxHp;
    const allies = this.champions.filter((c) => c.alive && c.team === champ.team && dist(c, champ) < 0.085).length;
    const enemies = this.champions.filter((c) => c.alive && c.team !== champ.team && dist(c, champ) < 0.085).length;
    if (hp < 0.3) return true;
    if (enemies - allies >= 2 && hp < 0.58) return true;
    if (ehp > hp + 0.24) return true;
    return false;
  }

  setPath(champ, target, reason) {
    const dest = { x: clamp(target.x, 0.03, 0.97), y: clamp(target.y, 0.03, 0.97) };
    champ.path = this.nav.findPath(champ, dest);
    champ.pathIndex = 0;
    champ.target = dest;
    champ.state = "moving";
    champ.reason = reason;
  }

  startRecall(champ) {
    champ.state = "recalling";
    champ.recallUntil = this.time + 8;
    champ.path = [];
    this.logEvent(`${champ.id} empieza back.`);
  }

  chooseLanePlan(champ) {
    const lane = champ.homeLane;
    const pressure = this.laneProgress(lane, champ.team);
    const hpRatio = champ.hp / champ.maxHp;
    const hasWave = this.laneHasFrontline(champ.team, lane);

    if (hpRatio < 0.38) return this.getLaneStageNode(champ.team, lane, "safe");

    const tower = this.getEnemyStructureInLane(champ.team, lane);
    if (hasWave && pressure > 0.1 && tower && this.isSafeToHitTower(champ, tower)) return tower;

    if (!hasWave) return this.getLaneStageNode(champ.team, lane, "hold");
    if (pressure > 0.12) return this.getLaneStageNode(champ.team, lane, "push");
    return this.getLaneStageNode(champ.team, lane, "hold");
  }

  chooseJunglePlan(champ, call) {
    if (call.type === "dragon") return DRAGON_PIT;
    if (call.type === "baron") return BARON_PIT;
    const camps = champ.team === TEAM_BLUE ? JUNGLE_POINTS.blue : JUNGLE_POINTS.red;
    if (Math.random() < 0.25) {
      const lanes = ["top", "mid", "bot"];
      const weak = lanes.map((l) => ({ l, p: this.laneProgress(l, champ.team) })).sort((a, b) => a.p - b.p)[0].l;
      return this.getLaneStageNode(champ.team, weak, "hold");
    }
    return camps[Math.floor(Math.random() * camps.length)];
  }

  chooseTarget(champ) {
    const call = this.chooseTeamCall(champ.team);
    const hpRatio = champ.hp / champ.maxHp;

    if (this.time < champ.disengageUntil) {
      this.setPath(champ, this.getLaneStageNode(champ.team, champ.homeLane, "safe"), "disengage");
      return;
    }

    if (hpRatio < 0.3 && Math.random() < 0.92) {
      this.startRecall(champ);
      return;
    }

    if (call.type !== "farm" && this.shouldJoinObjective(champ, call.type)) {
      this.setPath(champ, call.type === "dragon" ? DRAGON_PIT : BARON_PIT, `macro-${call.type}`);
      return;
    }

    if (champ.role !== "JGL") {
      const laneEnemy = this.champions
        .filter((e) => e.alive && e.team !== champ.team && e.homeLane === champ.homeLane && e.state !== "recalling" && dist(e, champ) < 0.06)
        .sort((a, b) => dist(champ, a) - dist(champ, b))[0];
      if (laneEnemy && this.shouldTakeTrade(champ, laneEnemy)) {
        this.setPath(champ, laneEnemy, "lane-trade");
        return;
      }
    }

    if (champ.role === "JGL") {
      this.setPath(champ, this.chooseJunglePlan(champ, call), "jungle");
      return;
    }

    this.setPath(champ, this.chooseLanePlan(champ), "lane");
  }

  killChampion(victim, killer) {
    if (!victim || !victim.role) return;
    victim.alive = false;
    victim.state = "dead";
    victim.path = [];
    victim.hp = 0;
    victim.respawnAt = this.time + Math.min(10 + Math.floor(this.time / 60) * 2, 58);
    this.score[killer.team].kills += 1;
    this.logEvent(`${killer.id} mata a ${victim.id}.`);
  }

  respawnChampion(champ) {
    const b = BASE_POSITION[champ.team];
    const o = ROLE_OFFSET[champ.role];
    champ.x = b.x + o.x;
    champ.y = b.y + o.y;
    champ.hp = champ.maxHp;
    champ.alive = true;
    champ.state = "idle";
    champ.decisionAt = this.time + 2;
    this.logEvent(`${champ.id} reaparece.`);
  }

  spawnMinionWave() {
    const lanes = ["top", "mid", "bot"];
    for (const lane of lanes) {
      for (let i = 0; i < 4; i += 1) {
        this.minions.push(this.createMinion(TEAM_BLUE, lane, i));
        this.minions.push(this.createMinion(TEAM_RED, lane, i));
      }
    }
    this.logEvent("Nueva wave de minions.");
  }

  createMinion(team, lane, idx) {
    const path = this.getLanePath(team, lane);
    return {
      id: `m-${team}-${lane}-${Math.random().toString(36).slice(2, 7)}`,
      team,
      lane,
      x: path[0].x,
      y: path[0].y,
      hp: 40,
      speed: 0.028,
      attackDamage: 20,
      attackCadence: 1.1,
      attackAt: 0,
      path,
      pathIndex: 1,
      offset: idx * 0.004,
      alive: true,
    };
  }

  updateMinions(dt) {
    const alive = [];
    for (const m of this.minions) {
      const enemyTower = this.structures
        .filter((s) => s.alive && s.team !== m.team && (s.lane === m.lane || s.lane === "base") && dist(m, s) < 0.07)
        .sort((a, b) => dist(m, a) - dist(m, b))[0];

      const enemyMinion = this.minions.find((o) => o.alive && o.team !== m.team && dist(m, o) < 0.02);

      if (enemyTower && this.time >= m.attackAt) {
        enemyTower.hp -= m.attackDamage;
        m.attackAt = this.time + m.attackCadence;
      } else if (enemyMinion && this.time >= m.attackAt) {
        enemyMinion.hp -= m.attackDamage * 0.65;
        m.attackAt = this.time + 0.85;
      } else {
        const t = m.path[m.pathIndex];
        if (t) {
          const tx = t.x + (m.team === TEAM_BLUE ? m.offset : -m.offset);
          const ty = t.y + (m.team === TEAM_BLUE ? m.offset : -m.offset);
          const d = Math.hypot(tx - m.x, ty - m.y);
          if (d < m.speed * dt) {
            m.x = tx;
            m.y = ty;
            m.pathIndex += 1;
          } else {
            m.x += ((tx - m.x) / d) * m.speed * dt;
            m.y += ((ty - m.y) / d) * m.speed * dt;
          }
        }
      }

      m.alive = m.hp > 0;
      if (m.alive && m.pathIndex <= m.path.length) alive.push(m);
    }
    this.minions = alive;
  }

  handleObjectives() {
    if (this.time >= this.nextDragonAt) {
      const b = this.champions.filter((c) => c.alive && c.team === TEAM_BLUE && dist(c, DRAGON_PIT) < 0.075).length;
      const r = this.champions.filter((c) => c.alive && c.team === TEAM_RED && dist(c, DRAGON_PIT) < 0.075).length;
      if (b + r >= 3) {
        const w = b >= r ? TEAM_BLUE : TEAM_RED;
        this.score[w].dragons += 1;
        this.nextDragonAt = this.time + 5 * 60;
        this.logEvent(`${w === TEAM_BLUE ? "BLUE" : "RED"} asegura dragon.`);
      }
    }
    if (this.time >= this.nextBaronAt) {
      const b = this.champions.filter((c) => c.alive && c.team === TEAM_BLUE && dist(c, BARON_PIT) < 0.075).length;
      const r = this.champions.filter((c) => c.alive && c.team === TEAM_RED && dist(c, BARON_PIT) < 0.075).length;
      if (b + r >= 3) {
        const w = b >= r ? TEAM_BLUE : TEAM_RED;
        this.score[w].barons += 1;
        this.nextBaronAt = this.time + 6 * 60;
        this.logEvent(`${w === TEAM_BLUE ? "BLUE" : "RED"} asegura baron.`);
      }
    }
  }

  updateCombat() {
    for (const champ of this.champions) {
      if (!champ.alive || champ.state === "recalling") continue;
      const enemy = this.champions
        .filter((e) => e.alive && e.team !== champ.team && e.state !== "recalling" && dist(e, champ) < 0.032)
        .sort((a, b) => dist(champ, a) - dist(champ, b))[0];
      if (!enemy) continue;

      if (this.shouldDisengage(champ, enemy)) {
        champ.disengageUntil = this.time + 1.8;
        champ.state = "idle";
        this.setPath(champ, this.getLaneStageNode(champ.team, champ.homeLane, "safe"), "retreat");
        continue;
      }

      if (dist(champ, enemy) < 0.029 && this.time >= champ.attackAt) {
        enemy.hp -= champ.attackDamage * (0.9 + Math.random() * 0.2);
        champ.attackAt = this.time + champ.attackCadence;
        champ.state = "fighting";
        champ.combatLockUntil = this.time + 0.8;

        if (enemy.hp <= 0 && enemy.alive) {
          this.killChampion(enemy, champ);
          champ.state = "idle";
          champ.decisionAt = Math.min(champ.decisionAt, this.time + 0.5);
        } else if (Math.random() < 0.34) {
          champ.disengageUntil = this.time + 0.9;
          champ.state = "idle";
          this.setPath(champ, this.getLaneStageNode(champ.team, champ.homeLane, "hold"), "short-trade-reset");
        }
      }
    }

    for (const champ of this.champions) {
      if (champ.alive && champ.state === "fighting" && this.time > champ.combatLockUntil) {
        champ.state = "idle";
      }
    }
  }

  updateStructures(dt) {
    for (const s of this.structures) {
      if (!s.alive) continue;
      const attackers = this.champions.filter((c) => c.alive && c.team !== s.team && dist(c, s) < 0.075);
      const attackerMinions = this.minions.filter((m) => m.alive && m.team !== s.team && dist(m, s) < 0.075);
      const defenders = this.champions.filter((c) => c.alive && c.team === s.team && dist(c, s) < 0.09);
      const towerTargets = attackerMinions.length > 0 ? attackerMinions : attackers;

      if (towerTargets.length > 0 && this.time >= s.towerAttackAt) {
        const t = towerTargets[0];
        const damage = attackerMinions.length > 0 ? 52 : 85;
        t.hp -= damage;
        s.towerAttackAt = this.time + s.towerCadence;
        if (t.hp <= 0) {
          if (t.role && t.alive) {
            const killer = this.champions.find((c) => c.alive && c.team === s.team) || t;
            this.killChampion(t, killer);
          } else {
            t.alive = false;
            t.hp = 0;
          }
        }
      }

      if (attackerMinions.length > 0) s.hp -= dt * (2 + attackerMinions.length * 1.2);
      if (attackers.length > 0 && (attackerMinions.length >= 2 || defenders.length === 0)) {
        s.hp -= dt * (2.4 + attackers.length * 1.6 + attackerMinions.length * 0.9);
      }

      if (s.hp <= 0) {
        s.alive = false;
        const winner = s.team === TEAM_BLUE ? TEAM_RED : TEAM_BLUE;
        this.score[winner].towers += 1;
        this.logEvent(`${winner === TEAM_BLUE ? "BLUE" : "RED"} destruye ${s.id}.`);
        if (s.id === "blue-nexus") this.winner = TEAM_RED;
        if (s.id === "red-nexus") this.winner = TEAM_BLUE;
      }
    }
  }

  updateChampions(dt) {
    for (const c of this.champions) {
      if (!c.alive) {
        if (this.time >= c.respawnAt) this.respawnChampion(c);
        continue;
      }

      if (c.state === "recalling") {
        if (this.time >= c.recallUntil) {
          const b = BASE_POSITION[c.team];
          c.x = b.x;
          c.y = b.y;
          c.hp = c.maxHp;
          c.state = "idle";
          this.logEvent(`${c.id} completa back.`);
        }
        continue;
      }

      const towerThreat = this.structures
        .filter((s) => s.alive && s.team !== c.team && dist(c, s) < 0.078)
        .sort((a, b) => dist(c, a) - dist(c, b))[0];
      if (towerThreat) {
        const cover = this.minions.filter((m) => m.alive && m.team === c.team && dist(m, towerThreat) < 0.085).length;
        const allies = this.champions.filter((x) => x.alive && x.team === c.team && dist(x, c) < 0.08).length;
        const enemies = this.champions.filter((x) => x.alive && x.team !== c.team && dist(x, c) < 0.08).length;
        if ((cover < 2 && c.hp / c.maxHp < 0.72) || enemies > allies + 1) {
          c.disengageUntil = Math.max(c.disengageUntil, this.time + 1.2);
          this.setPath(c, this.getLaneStageNode(c.team, c.homeLane, "safe"), "tower-reset");
        }
      }

      if (c.path.length > 1 && c.pathIndex < c.path.length) {
        const wp = c.path[c.pathIndex];
        const d = Math.hypot(wp.x - c.x, wp.y - c.y);
        const step = c.speed * dt;
        if (d <= step) {
          c.x = wp.x;
          c.y = wp.y;
          c.pathIndex += 1;
          if (c.pathIndex >= c.path.length) {
            c.path = [];
            c.state = "idle";
          }
        } else {
          c.x += ((wp.x - c.x) / d) * step;
          c.y += ((wp.y - c.y) / d) * step;
          c.state = "moving";
        }
      }

      if (c.hp < c.maxHp) c.hp = Math.min(c.maxHp, c.hp + dt * 0.85);
      if (this.time >= c.decisionAt && c.state !== "fighting") {
        this.chooseTarget(c);
        c.decisionAt = this.time + 2.8 + Math.random() * 1.8;
      }
    }
  }

  drawWalls() {
    ctx.save();
    ctx.globalAlpha = 0.15;
    ctx.fillStyle = "#f97316";
    for (const w of this.walls) {
      if (!w.points || w.points.length < 3) continue;
      ctx.beginPath();
      ctx.moveTo(w.points[0].x * MAP_SIZE, w.points[0].y * MAP_SIZE);
      for (let i = 1; i < w.points.length; i += 1) ctx.lineTo(w.points[i].x * MAP_SIZE, w.points[i].y * MAP_SIZE);
      ctx.closePath();
      ctx.fill();
    }
    ctx.restore();
  }

  render() {
    ctx.clearRect(0, 0, MAP_SIZE, MAP_SIZE);
    if (mapImage.complete && mapImage.naturalWidth > 0) {
      ctx.drawImage(mapImage, 0, 0, MAP_SIZE, MAP_SIZE);
    } else {
      ctx.fillStyle = "#0b1220";
      ctx.fillRect(0, 0, MAP_SIZE, MAP_SIZE);
    }

    this.drawWalls();

    for (const s of this.structures) {
      const x = s.x * MAP_SIZE;
      const y = s.y * MAP_SIZE;
      ctx.fillStyle = s.alive ? TEAM_STYLE[s.team] : "#64748b";
      ctx.globalAlpha = s.alive ? 0.9 : 0.45;
      const size = s.id.includes("nexus") ? 16 : 10;
      ctx.fillRect(x - size / 2, y - size / 2, size, size);
      ctx.globalAlpha = 1;
    }

    for (const m of this.minions) {
      ctx.fillStyle = TEAM_STYLE[m.team];
      ctx.globalAlpha = 0.8;
      ctx.beginPath();
      ctx.arc(m.x * MAP_SIZE, m.y * MAP_SIZE, 3, 0, Math.PI * 2);
      ctx.fill();
      ctx.globalAlpha = 1;
    }

    for (const c of this.champions) {
      const px = c.x * MAP_SIZE;
      const py = c.y * MAP_SIZE;
      if (!c.alive) {
        ctx.strokeStyle = "#f8fafc";
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(px - 6, py - 6);
        ctx.lineTo(px + 6, py + 6);
        ctx.moveTo(px + 6, py - 6);
        ctx.lineTo(px - 6, py + 6);
        ctx.stroke();
        continue;
      }

      ctx.fillStyle = TEAM_STYLE[c.team];
      ctx.beginPath();
      ctx.arc(px, py, 9, 0, Math.PI * 2);
      ctx.fill();

      if (c.state === "recalling") {
        const phase = ((c.recallUntil - this.time) % 1) * 6;
        ctx.strokeStyle = "#fde68a";
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(px, py, 11 + phase, 0, Math.PI * 2);
        ctx.stroke();
      }

      ctx.fillStyle = "#e2e8f0";
      ctx.font = "bold 9px Segoe UI";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(c.role, px, py);
    }
  }

  updateUI() {
    scoreboardEl.innerHTML = `
      <div class="time-value">${formatTime(this.time)}</div>
      <div class="score-row"><span class="score-title">Kills</span><span><span class="value-blue">${this.score.blue.kills}</span> - <span class="value-red">${this.score.red.kills}</span></span></div>
      <div class="score-row"><span class="score-title">Towers</span><span><span class="value-blue">${this.score.blue.towers}</span> - <span class="value-red">${this.score.red.towers}</span></span></div>
      <div class="score-row"><span class="score-title">Dragons</span><span><span class="value-blue">${this.score.blue.dragons}</span> - <span class="value-red">${this.score.red.dragons}</span></span></div>
      <div class="score-row"><span class="score-title">Barons</span><span><span class="value-blue">${this.score.blue.barons}</span> - <span class="value-red">${this.score.red.barons}</span></span></div>
      <div class="score-row"><span class="score-title">Estado</span><span>${this.winner ? `Gana ${this.winner.toUpperCase()}` : "En juego"}</span></div>
    `;
    eventsListEl.innerHTML = this.events.map((e) => `<li><span class="t">[${e.t}]</span>${e.text}</li>`).join("");
  }

  tick = () => {
    const now = performance.now();
    const frameDt = clamp((now - this.lastTick) / 1000, 0.001, 0.06);
    this.lastTick = now;
    const dt = frameDt * this.speed;

    if (!this.winner) {
      this.time += dt;
      if (this.time >= this.spawnWaveAt) {
        this.spawnMinionWave();
        this.spawnWaveAt += 30;
      }
      this.updateChampions(dt);
      this.updateMinions(dt);
      this.updateCombat();
      this.updateStructures(dt);
      this.handleObjectives();
    }

    this.render();
    this.updateUI();
    requestAnimationFrame(this.tick);
  };
}

async function loadWalls() {
  try {
    const res = await fetch("./lol_walls.json");
    if (!res.ok) throw new Error("no walls");
    const data = await res.json();
    return data.walls ?? FALLBACK_WALLS;
  } catch {
    return FALLBACK_WALLS;
  }
}

async function start() {
  const walls = await loadWalls();
  const nav = new NavGrid(walls, 120);
  const sim = new MatchSimulator(nav, walls);
  sim.tick();
}

start();
