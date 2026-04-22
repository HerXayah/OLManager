import { useEffect, useMemo, useRef, useState } from "react";
import type { MatchEvent, MatchSnapshot } from "./types";
import { getWalls } from "./lol-prototype/assets/map";
import { NavGrid } from "./lol-prototype/engine/navigation";
import { PrototypeSimulation } from "./lol-prototype/engine/simulation";
import type { ChampionCombatProfile } from "./lol-prototype/engine/simulation";
import { renderSimulation } from "./lol-prototype/ui/render";
import { EventFeedPanel, ScoreboardPanel } from "./lol-prototype/ui/panels";

export interface ChampionSelectionByPlayer {
  home: Record<string, string>;
  away: Record<string, string>;
  homeRoles: Record<string, "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT">;
  awayRoles: Record<string, "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT">;
}

interface Props {
  snapshot: MatchSnapshot;
  championSelections?: ChampionSelectionByPlayer | null;
  onSnapshotUpdate: (snap: MatchSnapshot) => void;
  onImportantEvent: (evt: MatchEvent) => void;
  onFullTime: () => void;
}

const SPEEDS = [
  { id: "x4", value: 4 },
  { id: "x8", value: 8 },
  { id: "x12", value: 12 },
];

const DDRAGON_VERSION = "14.24.1";

function attackTypeFromStats(attackRange: number, tags: string[]) {
  if (attackRange >= 300) return "ranged" as const;
  if (tags.includes("Marksman")) return "ranged" as const;
  return "melee" as const;
}

function normalizeAttackRange(attackRange: number) {
  // Compact ranged vs melee spacing for this prototype:
  // ranged should have some advantage, but not excessive standoff distance.
  if (attackRange >= 300) return 0.056;
  return 0.049;
}

function compactGold(gold: number) {
  if (gold < 1000) return `${Math.round(gold)}g`;
  return `${(gold / 1000).toFixed(1)}k`;
}

export default function LolMatchLive({ snapshot, championSelections, onSnapshotUpdate, onImportantEvent, onFullTime }: Props) {
  const walls = useMemo(() => getWalls(), []);
  const nav = useMemo(() => new NavGrid(walls), [walls]);
  const [seed, setSeed] = useState("lol-prototype-1");
  const [running, setRunning] = useState(true);
  const [speed, setSpeed] = useState(4);
  const [tick, setTick] = useState(0);

  const championByPlayerId = useMemo<Record<string, string>>(() => {
    if (!championSelections) return {};
    return {
      ...championSelections.home,
      ...championSelections.away,
    };
  }, [championSelections]);
  const [championProfilesById, setChampionProfilesById] = useState<Record<string, ChampionCombatProfile>>({});

  useEffect(() => {
    let cancelled = false;

    const loadChampionProfiles = async () => {
      const pickedChampionIds = Array.from(new Set(Object.values(championByPlayerId).filter(Boolean)));
      if (pickedChampionIds.length === 0) {
        if (!cancelled) setChampionProfilesById({});
        return;
      }

      try {
        const response = await fetch(`https://ddragon.leagueoflegends.com/cdn/${DDRAGON_VERSION}/data/en_US/champion.json`);
        if (!response.ok) throw new Error(`champion.json status ${response.status}`);
        const payload = await response.json() as {
          data?: Record<string, { id: string; tags: string[]; stats: { hp: number; attackrange: number } }>;
        };

        const nextProfiles: Record<string, ChampionCombatProfile> = {};
        const champions = payload.data ?? {};
        pickedChampionIds.forEach((championId) => {
          const data = champions[championId];
          if (!data) return;
          const attackType = attackTypeFromStats(data.stats.attackrange, data.tags ?? []);
          nextProfiles[championId] = {
            baseHp: Math.round(data.stats.hp),
            attackType,
            attackRange: normalizeAttackRange(data.stats.attackrange),
          };
        });

        if (!cancelled) setChampionProfilesById(nextProfiles);
      } catch {
        if (!cancelled) setChampionProfilesById({});
      }
    };

    void loadChampionProfiles();
    return () => {
      cancelled = true;
    };
  }, [championByPlayerId]);

  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const simRef = useRef<PrototypeSimulation | null>(null);
  const rafRef = useRef<number | null>(null);
  const lastRef = useRef<number>(0);
  const finishedRef = useRef(false);

  useEffect(() => {
    simRef.current = new PrototypeSimulation(nav, snapshot, seed, championByPlayerId, championProfilesById);
    finishedRef.current = false;
  }, [nav, seed, snapshot, championByPlayerId, championProfilesById]);

  useEffect(() => {
    const loop = (ts: number) => {
      const sim = simRef.current;
      const canvas = canvasRef.current;
      if (!sim || !canvas) return;

      if (!lastRef.current) lastRef.current = ts;
      const dt = Math.min(0.05, (ts - lastRef.current) / 1000);
      lastRef.current = ts;

      sim.setRunning(running);
      sim.tick(dt, speed);

      const rect = canvas.getBoundingClientRect();
      const size = Math.max(320, Math.floor(Math.min(rect.width, rect.height)));
      if (canvas.width !== size || canvas.height !== size) {
        canvas.width = size;
        canvas.height = size;
      }
      renderSimulation(canvas, sim.state, walls, championByPlayerId);

      if (sim.state.winner && !finishedRef.current) {
        finishedRef.current = true;
        const evt: MatchEvent = {
          minute: Math.floor(sim.state.timeSec / 60),
          event_type: "NexusDestroyed",
          side: sim.state.winner === "blue" ? "Home" : "Away",
          zone: "Midfield",
          player_id: null,
          secondary_player_id: null,
        };
        onImportantEvent(evt);
        onSnapshotUpdate({
          ...snapshot,
          phase: "Finished",
          current_minute: Math.floor(sim.state.timeSec / 60),
          home_score: sim.state.winner === "blue" ? 1 : 0,
          away_score: sim.state.winner === "red" ? 1 : 0,
        });
        setTimeout(onFullTime, 400);
      }

      setTick((v) => v + 1);
      rafRef.current = requestAnimationFrame(loop);
    };

    rafRef.current = requestAnimationFrame(loop);
    return () => {
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
      lastRef.current = 0;
    };
  }, [championByPlayerId, onFullTime, onImportantEvent, onSnapshotUpdate, running, snapshot, speed, walls]);

  const sim = simRef.current;
  const teamHud = (() => {
    const blueTeam = sim?.state.champions.filter((c) => c.team === "blue") ?? [];
    const redTeam = sim?.state.champions.filter((c) => c.team === "red") ?? [];
    const blueAvg = blueTeam.length ? blueTeam.reduce((sum, c) => sum + c.level, 0) / blueTeam.length : 1;
    const redAvg = redTeam.length ? redTeam.reduce((sum, c) => sum + c.level, 0) / redTeam.length : 1;
    return {
      blueGold: sim?.state.stats.blue.gold ?? 0,
      redGold: sim?.state.stats.red.gold ?? 0,
      blueAvg,
      redAvg,
    };
  })();
  const status = sim?.state.winner
    ? `Winner: ${sim.state.winner.toUpperCase()}`
    : running
      ? "Running"
      : "Paused";

  const handleReset = () => {
    simRef.current?.reset(seed);
    finishedRef.current = false;
    setRunning(true);
  };

  const toggleWalls = () => {
    simRef.current?.toggleWalls();
  };

  void tick;

  return (
    <div className="min-h-screen bg-[#020617] text-slate-100">
      <div className="mx-auto max-w-[1400px] p-4 grid grid-cols-1 xl:grid-cols-[1fr_360px] gap-4">
        <div className="rounded-2xl border border-cyan-500/20 bg-[#070f21] p-3">
          <div className="mb-3 flex items-center justify-between">
            <div>
              <p className="text-xs uppercase tracking-widest text-cyan-300">Prototype · simulator.js baseline</p>
              <p className="text-sm font-semibold">{snapshot.home_team.name} vs {snapshot.away_team.name}</p>
            </div>
            <div className="text-right text-xs text-cyan-300">
              <p>{Math.floor((sim?.state.timeSec ?? 0) / 60)}:{Math.floor((sim?.state.timeSec ?? 0) % 60).toString().padStart(2, "0")}</p>
              <p>{status}</p>
            </div>
          </div>

          <div className="w-full flex justify-center">
            <div className="relative w-full max-w-[980px]">
              <canvas ref={canvasRef} className="w-full aspect-square rounded-xl border border-cyan-500/25 bg-black" />
              <div className="pointer-events-none absolute bottom-3 left-3 rounded border border-cyan-400/40 bg-[#00121f]/85 px-2 py-1 text-[11px] text-cyan-100">
                <p className="font-semibold text-cyan-300">Blue</p>
                <p>Gold {compactGold(teamHud.blueGold)} · Avg Lv {teamHud.blueAvg.toFixed(1)}</p>
              </div>
              <div className="pointer-events-none absolute bottom-3 right-3 rounded border border-rose-400/40 bg-[#240611]/85 px-2 py-1 text-[11px] text-rose-100 text-right">
                <p className="font-semibold text-rose-300">Red</p>
                <p>Gold {compactGold(teamHud.redGold)} · Avg Lv {teamHud.redAvg.toFixed(1)}</p>
              </div>
            </div>
          </div>
        </div>

        <div className="space-y-3">
          <ScoreboardPanel
            timeSec={sim?.state.timeSec ?? 0}
            status={status}
            blue={{
              ...(sim?.state.stats.blue ?? { kills: 0, towers: 0, dragons: 0, barons: 0, gold: 0 }),
              avgLevel: teamHud.blueAvg,
            }}
            red={{
              ...(sim?.state.stats.red ?? { kills: 0, towers: 0, dragons: 0, barons: 0, gold: 0 }),
              avgLevel: teamHud.redAvg,
            }}
          />

          <div className="rounded-xl border border-cyan-500/25 bg-[#0a142b] p-3 text-xs">
            <p className="mb-2 uppercase tracking-widest text-cyan-300">Controls</p>
            <div className="grid grid-cols-2 gap-2">
              <button className="rounded border border-cyan-500/30 bg-slate-900/50 px-2 py-1" onClick={() => setRunning((v) => !v)}>
                {running ? "Pause" : "Play"}
              </button>
              <button className="rounded border border-cyan-500/30 bg-slate-900/50 px-2 py-1" onClick={handleReset}>
                Reset Match
              </button>
              <button className="rounded border border-cyan-500/30 bg-slate-900/50 px-2 py-1 col-span-2" onClick={toggleWalls}>
                Toggle Show Walls
              </button>
            </div>
            <div className="mt-2 grid grid-cols-3 gap-1">
              {SPEEDS.map((s) => (
                <button
                  key={s.id}
                  className={`rounded border px-2 py-1 ${speed === s.value ? "border-cyan-300 bg-cyan-500/20" : "border-cyan-500/30 bg-slate-900/50"}`}
                  onClick={() => setSpeed(s.value)}
                >
                  {s.id}
                </button>
              ))}
            </div>
            <div className="mt-2">
              <label className="text-[11px] text-cyan-200">Seed</label>
              <input
                value={seed}
                onChange={(e) => setSeed(e.target.value)}
                className="mt-1 w-full rounded border border-cyan-500/30 bg-slate-950 px-2 py-1 text-xs"
              />
            </div>
          </div>

          <EventFeedPanel events={sim?.state.events ?? []} />
        </div>
      </div>
    </div>
  );
}
