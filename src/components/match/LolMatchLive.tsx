import { useEffect, useMemo, useRef, useState } from "react";
import type { MatchEvent, MatchSnapshot } from "./types";
import { getWalls } from "./lol-prototype/assets/map";
import { NavGrid } from "./lol-prototype/engine/navigation";
import { PrototypeSimulation } from "./lol-prototype/engine/simulation";
import type { ChampionCombatProfile } from "./lol-prototype/engine/simulation";
import type { MatchState } from "./lol-prototype/engine/types";
import type { LolSimV1AiMode, LolSimV1RuntimeState } from "./lol-prototype/backend/contract-v1";
import { LolSimV2Client } from "./lol-prototype/backend/tauri-client";
import { renderSimulation } from "./lol-prototype/ui/render";
import { LecLowerThirdPanel } from "./lol-prototype/ui/panels";

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

const AI_MODES: Array<{ id: LolSimV1AiMode; label: string }> = [
  { id: "rules", label: "Rules" },
  { id: "hybrid", label: "Hybrid" },
];

const DDRAGON_VERSION = "14.24.1";
const USE_RUST_SIM_V2 = true;

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

export default function LolMatchLive({ snapshot, championSelections, onSnapshotUpdate, onImportantEvent, onFullTime }: Props) {
  const walls = useMemo(() => getWalls(), []);
  const nav = useMemo(() => new NavGrid(walls), [walls]);
  const [seed, setSeed] = useState("lol-prototype-1");
  const [running, setRunning] = useState(true);
  const [speed, setSpeed] = useState(4);
  const [aiMode, setAiMode] = useState<LolSimV1AiMode>("hybrid");
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
  const backendClientRef = useRef<LolSimV2Client | null>(null);
  const backendStateRef = useRef<LolSimV1RuntimeState | null>(null);
  const backendTickInFlightRef = useRef(false);
  const backendPendingDtRef = useRef(0);
  const rafRef = useRef<number | null>(null);
  const lastRef = useRef<number>(0);
  const finishedRef = useRef(false);

  const currentState = (): MatchState | null => {
    if (USE_RUST_SIM_V2 && backendStateRef.current) return backendStateRef.current;
    return simRef.current?.state ?? null;
  };

  useEffect(() => {
    const tsSim = new PrototypeSimulation(nav, snapshot, seed, championByPlayerId, championProfilesById);
    simRef.current = tsSim;
    backendClientRef.current = null;
    backendStateRef.current = null;
    backendTickInFlightRef.current = false;
    backendPendingDtRef.current = 0;
    finishedRef.current = false;

    if (!USE_RUST_SIM_V2) return;

    const client = new LolSimV2Client();
    backendClientRef.current = client;
    let disposed = false;

    void client
      .init({
        seed,
        aiMode,
        snapshot,
        championByPlayerId,
        championProfilesById,
        initialState: { ...tsSim.state, speed },
      })
      .then((response) => {
        if (disposed || backendClientRef.current !== client) return;
        backendStateRef.current = response.state;
      })
      .catch(() => {
        if (disposed || backendClientRef.current !== client) return;
        backendClientRef.current = null;
        backendStateRef.current = null;
        backendTickInFlightRef.current = false;
      });

    return () => {
      disposed = true;
      if (backendClientRef.current === client) {
        backendClientRef.current = null;
        backendStateRef.current = null;
        backendTickInFlightRef.current = false;
        backendPendingDtRef.current = 0;
      }
      void client.dispose().catch(() => undefined);
    };
  }, [aiMode, nav, seed, snapshot, championByPlayerId, championProfilesById]);

  useEffect(() => {
    const loop = (ts: number) => {
      const sim = simRef.current;
      const canvas = canvasRef.current;
      if (!sim || !canvas) return;

      if (!lastRef.current) lastRef.current = ts;
      const dt = Math.min(0.05, (ts - lastRef.current) / 1000);
      lastRef.current = ts;

      sim.setRunning(running);

      const backendClient = USE_RUST_SIM_V2 ? backendClientRef.current : null;
      if (backendClient && backendStateRef.current) {
        // Acumulador anti-tirones: si backend está ocupado, no perdemos tiempo simulado.
        backendPendingDtRef.current = Math.min(0.5, backendPendingDtRef.current + dt);
        if (!backendTickInFlightRef.current) {
          backendTickInFlightRef.current = true;
          const dtForBackend = Math.min(0.05, backendPendingDtRef.current);
          backendPendingDtRef.current = Math.max(0, backendPendingDtRef.current - dtForBackend);
          void backendClient
            .tick({ dtSec: dtForBackend, running, speed })
            .then((response) => {
              if (backendClientRef.current !== backendClient) return;
              backendStateRef.current = response.state;
            })
            .catch(() => {
              if (backendClientRef.current !== backendClient) return;
              backendClientRef.current = null;
              backendStateRef.current = null;
              backendPendingDtRef.current = 0;
            })
            .finally(() => {
              if (backendClientRef.current === backendClient) {
                backendTickInFlightRef.current = false;
              }
            });
        }
      } else {
        sim.tick(dt, speed);
      }

      const state = currentState();
      if (!state) return;

      const rect = canvas.getBoundingClientRect();
      const size = Math.max(320, Math.floor(Math.min(rect.width, rect.height)));
      if (canvas.width !== size || canvas.height !== size) {
        canvas.width = size;
        canvas.height = size;
      }
      renderSimulation(canvas, state, walls, championByPlayerId);

      if (state.winner && !finishedRef.current) {
        finishedRef.current = true;
        const evt: MatchEvent = {
          minute: Math.floor(state.timeSec / 60),
          event_type: "NexusDestroyed",
          side: state.winner === "blue" ? "Home" : "Away",
          zone: "Midfield",
          player_id: null,
          secondary_player_id: null,
        };
        onImportantEvent(evt);
        onSnapshotUpdate({
          ...snapshot,
          phase: "Finished",
          current_minute: Math.floor(state.timeSec / 60),
          home_score: state.winner === "blue" ? 1 : 0,
          away_score: state.winner === "red" ? 1 : 0,
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

  useEffect(() => {
    return () => {
      const client = backendClientRef.current;
      backendClientRef.current = null;
      backendStateRef.current = null;
      backendTickInFlightRef.current = false;
      backendPendingDtRef.current = 0;
      if (client) {
        void client.dispose().catch(() => undefined);
      }
    };
  }, []);

  const state = currentState();
  const status = state?.winner
    ? `Winner: ${state.winner.toUpperCase()}`
    : running
      ? "Running"
      : "Paused";

  const handleReset = () => {
    const sim = simRef.current;
    if (!sim) return;

    sim.reset(seed);

    const backendClient = USE_RUST_SIM_V2 ? backendClientRef.current : null;
    if (backendClient && backendStateRef.current) {
      backendTickInFlightRef.current = false;
      void backendClient
        .reset({ seed, aiMode, initialState: { ...sim.state, speed } })
        .then((response) => {
          if (backendClientRef.current !== backendClient) return;
          backendStateRef.current = response.state;
          setTick((v) => v + 1);
        })
        .catch(() => {
          if (backendClientRef.current !== backendClient) return;
          backendClientRef.current = null;
          backendStateRef.current = null;
        });
    }

    finishedRef.current = false;
    setRunning(true);
  };

  const toggleWalls = () => {
    const backendState = USE_RUST_SIM_V2 ? backendStateRef.current : null;
    if (backendState) {
      backendState.showWalls = !backendState.showWalls;
      setTick((v) => v + 1);
      return;
    }
    simRef.current?.toggleWalls();
  };

  void tick;

  return (
    <div className="h-screen w-screen overflow-hidden bg-[#050505] text-white">
      <div className="flex h-full w-full flex-col items-center justify-center">
        <div className="map-container flex w-full flex-[0_0_auto] justify-center">
          <canvas
            ref={canvasRef}
            className="h-[60vh] w-auto max-w-[92vw] object-contain"
          />
        </div>

        <div className="hud-board w-full">
          <LecLowerThirdPanel champions={state?.champions ?? []} championByPlayerId={championByPlayerId} timeSec={state?.timeSec ?? 0} />

          <div className="mx-auto mt-1 flex w-full max-w-[1400px] items-center justify-between px-[20px] text-[10px] text-white/55">
            <span>{snapshot.home_team.name} vs {snapshot.away_team.name}</span>
            <span>{Math.floor((state?.timeSec ?? 0) / 60)}:{Math.floor((state?.timeSec ?? 0) % 60).toString().padStart(2, "0")} · {status}</span>
          </div>

          <div className="mx-auto mt-1 flex w-full max-w-[1400px] flex-wrap items-center justify-center gap-1 px-[20px] pb-2 text-[10px] text-white/75">
            <button className="rounded border border-cyan-500/30 bg-black/60 px-2 py-1" onClick={() => setRunning((v) => !v)}>
              {running ? "Pause" : "Play"}
            </button>
            <button className="rounded border border-cyan-500/30 bg-black/60 px-2 py-1" onClick={handleReset}>Reset</button>
            <button className="rounded border border-cyan-500/30 bg-black/60 px-2 py-1" onClick={toggleWalls}>Walls</button>
            {SPEEDS.map((s) => (
              <button
                key={s.id}
                className={`rounded border px-2 py-1 ${speed === s.value ? "border-cyan-300 bg-cyan-500/20" : "border-cyan-500/30 bg-black/60"}`}
                onClick={() => setSpeed(s.value)}
              >
                {s.id}
              </button>
            ))}
            {AI_MODES.map((mode) => (
              <button
                key={mode.id}
                className={`rounded border px-2 py-1 ${aiMode === mode.id ? "border-cyan-300 bg-cyan-500/20" : "border-cyan-500/30 bg-black/60"}`}
                onClick={() => setAiMode(mode.id)}
              >
                {mode.label}
              </button>
            ))}
            <input
              value={seed}
              onChange={(e) => setSeed(e.target.value)}
              className="rounded border border-cyan-500/30 bg-black/70 px-2 py-1 text-[10px]"
              aria-label="Simulation seed"
            />
          </div>
        </div>
      </div>
    </div>
  );
}
