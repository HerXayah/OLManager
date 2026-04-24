import { useEffect, useMemo, useRef, useState } from "react";
import type { MatchEvent, MatchSnapshot } from "./types";
import { getWalls } from "./lol-prototype/assets/map";
import { NavGrid } from "./lol-prototype/engine/navigation";
import { PrototypeSimulation } from "./lol-prototype/engine/simulation";
import type { ChampionCombatProfile } from "./lol-prototype/engine/simulation";
import type { MatchState } from "./lol-prototype/engine/types";
import type {
  LolChampionUltimateProfile,
  LolSimV1AiMode,
  LolSimV1PolicyConfig,
  LolSimV1RuntimeState,
} from "./lol-prototype/backend/contract-v1";
import { LolSimV2Client } from "./lol-prototype/backend/tauri-client";
import { renderSimulation } from "./lol-prototype/ui/render";
import { LecLowerThirdPanel } from "./lol-prototype/ui/panels";
import { useSettingsStore } from "../../store/settingsStore";

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
  onFullTime: (finalState: LolSimV1RuntimeState) => void;
}

const SPEEDS = [
  { id: "x1", value: 4 },
  { id: "x2", value: 8 },
  { id: "x4", value: 12 },
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

function randomSeed10Digits() {
  const firstDigit = Math.floor(Math.random() * 9) + 1;
  const rest = Math.floor(Math.random() * 1_000_000_000).toString().padStart(9, "0");
  return `${firstDigit}${rest}`;
}

function classifyUltimateArchetype(name: string, description: string) {
  const text = `${name} ${description}`.toLowerCase();
  if (text.includes("execute") || text.includes("missing health") || text.includes("below")) return "execute";
  if (text.includes("global") || text.includes("map") || text.includes("long range") || text.includes("anywhere")) return "global";
  if (text.includes("dash") || text.includes("leap") || text.includes("charge") || text.includes("knockup") || text.includes("pull")) return "engage";
  if (text.includes("heal") || text.includes("shield") || text.includes("invulner") || text.includes("stasis") || text.includes("untarget")) return "defensive";
  if (text.includes("zone") || text.includes("field") || text.includes("storm") || text.includes("area") || text.includes("aoe")) return "zone";
  if (text.includes("transform") || text.includes("form")) return "sustain";
  return "burst";
}

function toTitle(raw: string | null | undefined) {
  if (!raw) return "—";
  return raw.charAt(0).toUpperCase() + raw.slice(1).toLowerCase();
}

function objectiveStatus(alive: boolean | undefined, nextSpawnAt: number | null | undefined) {
  if (alive) return "Alive";
  if (typeof nextSpawnAt === "number") {
    return `${Math.max(0, Math.floor(nextSpawnAt / 60))}m`;
  }
  return "—";
}

export default function LolMatchLive({ snapshot, championSelections, onSnapshotUpdate, onImportantEvent, onFullTime }: Props) {
  const walls = useMemo(() => getWalls(), []);
  const nav = useMemo(() => new NavGrid(walls), [walls]);
  const [seed] = useState(randomSeed10Digits);
  const [running, setRunning] = useState(true);
  const [speed, setSpeed] = useState(4);
  const aiMode: LolSimV1AiMode = "hybrid";
  const { settings } = useSettingsStore();
  const simPolicy = useMemo<LolSimV1PolicyConfig>(() => ({
    hybridOpenTradeConfidenceHigh: settings.lol_hybrid_open_trade_confidence_high,
    hybridDisengageConfidenceLow: settings.lol_hybrid_disengage_confidence_low,
  }), [settings.lol_hybrid_disengage_confidence_low, settings.lol_hybrid_open_trade_confidence_high]);
  const [tick, setTick] = useState(0);

  const championByPlayerId = useMemo<Record<string, string>>(() => {
    if (!championSelections) return {};
    return {
      ...championSelections.home,
      ...championSelections.away,
    };
  }, [championSelections]);
  const [championProfilesById, setChampionProfilesById] = useState<Record<string, ChampionCombatProfile>>({});
  const [championUltimatesById, setChampionUltimatesById] = useState<Record<string, LolChampionUltimateProfile>>({});

  useEffect(() => {
    let cancelled = false;

    const loadChampionProfiles = async () => {
      const pickedChampionIds = Array.from(new Set(Object.values(championByPlayerId).filter(Boolean)));
      if (pickedChampionIds.length === 0) {
        if (!cancelled) setChampionProfilesById({});
        if (!cancelled) setChampionUltimatesById({});
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

        const uniqueChampionIds = Array.from(new Set(Object.values(championByPlayerId).filter(Boolean)));
        const ultimateEntries = await Promise.all(uniqueChampionIds.map(async (championId) => {
          try {
            const detailResponse = await fetch(`https://ddragon.leagueoflegends.com/cdn/${DDRAGON_VERSION}/data/en_US/champion/${championId}.json`);
            if (!detailResponse.ok) return [championId, null] as const;
            const detailPayload = await detailResponse.json() as {
              data?: Record<string, {
                id: string;
                spells?: Array<{ name?: string; description?: string; tooltip?: string; image?: { full?: string } }>;
              }>;
            };
            const detail = detailPayload.data?.[championId];
            const ultimate = detail?.spells?.[3];
            const image = ultimate?.image?.full;
            if (!ultimate || !image) return [championId, null] as const;
            const description = ultimate.tooltip ?? ultimate.description ?? "";
            const archetype = classifyUltimateArchetype(ultimate.name ?? "", description);
            return [championId, {
              archetype,
              icon: `https://ddragon.leagueoflegends.com/cdn/${DDRAGON_VERSION}/img/spell/${image}`,
            }] as const;
          } catch {
            return [championId, null] as const;
          }
        }));

        if (!cancelled) {
          const mapped = ultimateEntries.reduce<Record<string, LolChampionUltimateProfile>>((acc, [championId, value]) => {
            if (value) acc[championId] = value;
            return acc;
          }, {});
          setChampionUltimatesById(mapped);
        }
      } catch {
        if (!cancelled) {
          setChampionProfilesById({});
          setChampionUltimatesById({});
        }
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
        policy: simPolicy,
        snapshot,
          championByPlayerId,
          championProfilesById,
          championUltimatesById,
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
  }, [aiMode, nav, seed, simPolicy, snapshot, championByPlayerId, championProfilesById, championUltimatesById]);

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
        setTimeout(() => onFullTime(state), 400);
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
  const dragon = state?.objectives?.dragon;
  const baron = state?.objectives?.baron;
  const elder = state?.neutralTimers?.entities?.elder;
  const soulOwner = dragon?.soulClaimedBy;
  const soulLabel = soulOwner ? `${soulOwner} (${toTitle(dragon?.soulRiftKind)})` : "—";
  const dragonKind = toTitle(dragon?.currentKind);

  const handleReset = () => {
    const sim = simRef.current;
    if (!sim) return;

    const nextSeed = randomSeed10Digits();

    sim.reset(nextSeed);

    const backendClient = USE_RUST_SIM_V2 ? backendClientRef.current : null;
    if (backendClient && backendStateRef.current) {
      backendTickInFlightRef.current = false;
      void backendClient
        .reset({ seed: nextSeed, aiMode, policy: simPolicy, initialState: { ...sim.state, speed } })
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

          <div className="mx-auto mt-1 grid w-full max-w-[1400px] grid-cols-2 gap-1 px-[20px] text-[10px] text-white/75 md:grid-cols-4">
            <div className="rounded border border-white/10 bg-black/35 px-2 py-1">
              Dragon: {dragonKind} · {objectiveStatus(dragon?.alive, dragon?.nextSpawnAt)}
            </div>
            <div className="rounded border border-white/10 bg-black/35 px-2 py-1">
              Stacks H/A: {dragon?.homeStacks ?? 0}/{dragon?.awayStacks ?? 0} · Soul: {soulLabel}
            </div>
            <div className="rounded border border-white/10 bg-black/35 px-2 py-1">
              Baron: {objectiveStatus(baron?.alive, baron?.nextSpawnAt)}
            </div>
            <div className="rounded border border-white/10 bg-black/35 px-2 py-1">
              Elder: {objectiveStatus(elder?.alive, elder?.nextSpawnAt)}
            </div>
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
          </div>
        </div>
      </div>
    </div>
  );
}
