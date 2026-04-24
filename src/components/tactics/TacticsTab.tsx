import type { JSX } from "react";
import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowDown,
  ArrowUp,
  ArrowUpRight,
  Brain,
  Compass,
  Crosshair,
  Feather,
  Flame,
  Scale,
  Shield,
  Zap,
} from "lucide-react";
import type {
  GameStateData,
  LolTacticsData,
  PlayerSelectionOptions,
} from "../../store/gameStore";
import { useTranslation } from "react-i18next";
import {
  DEFAULT_LOL_TACTICS,
  ROLE_ORDER,
  computeCoherenceBreakdown,
  computeRoleModifiers,
  type DraftRole,
} from "../../lib/lolTactics";
import { Card, CardBody, CardHeader } from "../ui";

interface TacticsTabProps {
  gameState: GameStateData;
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onGameUpdate: (g: GameStateData) => void;
}

type StrongSide = LolTacticsData["strong_side"];
type GameTiming = LolTacticsData["game_timing"];
type JungleStyle = LolTacticsData["jungle_style"];
type JunglePathing = LolTacticsData["jungle_pathing"];
type FightPlan = LolTacticsData["fight_plan"];
type SupportRoaming = LolTacticsData["support_roaming"];

const OVR_KEYS = [
  "pace",
  "stamina",
  "strength",
  "agility",
  "passing",
  "shooting",
  "tackling",
  "dribbling",
  "defending",
  "positioning",
  "vision",
  "decisions",
  "composure",
  "aggression",
  "teamwork",
  "leadership",
] as const;

const ROLE_META: Record<DraftRole, { name: string; icon: string }> = {
  TOP: { name: "Carril superior", icon: "🛡️" },
  JUNGLE: { name: "JUNGLE", icon: "🌲" },
  MID: { name: "Carril central", icon: "⚡" },
  ADC: { name: "Carril inferior (tirador)", icon: "🎯" },
  SUPPORT: { name: "Soporte", icon: "🤝" },
};

const ROLE_ICON_URLS: Record<DraftRole, string> = {
  TOP: "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-top.png",
  JUNGLE:
    "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-jungle.png",
  MID: "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-middle.png",
  ADC: "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-bottom.png",
  SUPPORT:
    "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-utility.png",
};

const STRONG_SIDE_OPTIONS: Array<{ value: StrongSide; label: string; icon: JSX.Element; description: string }> = [
  {
    value: "Top",
    label: "Top",
    icon: <Shield className="h-4 w-4" />,
    description: "Jugamos para top: prioridad de recursos y ganks arriba.",
  },
  {
    value: "Mid",
    label: "Mid",
    icon: <Brain className="h-4 w-4" />,
    description: "El eje del mapa es mid: control de tempo y rotaciones.",
  },
  {
    value: "Bot",
    label: "Bot",
    icon: <Crosshair className="h-4 w-4" />,
    description: "Invertimos en botlane para escalar peleas y objetivos.",
  },
];

const GAME_TIMING_OPTIONS: Array<{ value: GameTiming; label: string; icon: JSX.Element; description: string }> = [
  {
    value: "Early",
    label: "Early game",
    icon: <Flame className="h-4 w-4 text-red-500" />,
    description: "Buscamos ventaja antes de minuto 14 con ritmo agresivo.",
  },
  {
    value: "Mid",
    label: "Mid game",
    icon: <Scale className="h-4 w-4 text-accent-500" />,
    description: "Pico de poder en mid game con setup de objetivos.",
  },
  {
    value: "Late",
    label: "Late game",
    icon: <Feather className="h-4 w-4 text-blue-500" />,
    description: "Priorizamos escalado y ejecución en peleas largas.",
  },
];

const JUNGLE_STYLE_OPTIONS: Array<{ value: JungleStyle; label: string; icon: JSX.Element; description: string }> = [
  {
    value: "Ganker",
    label: "Gankear",
    icon: <Crosshair className="h-4 w-4" />,
    description: "JUNGLE de presión en líneas: castiga errores temprano.",
  },
  {
    value: "Invader",
    label: "Invadir",
    icon: <Zap className="h-4 w-4" />,
    description: "Entramos a JUNGLE rival para negar recursos y visión.",
  },
  {
    value: "Farmer",
    label: "Farmear",
    icon: <Feather className="h-4 w-4" />,
    description: "Maximizamos farmeo para llegar fuertes a mid/late.",
  },
  {
    value: "Enabler",
    label: "Habilitar",
    icon: <Brain className="h-4 w-4" />,
    description: "JUNGLE habilita carries con cobertura y tempo.",
  },
];

const JUNGLE_PATHING_OPTIONS: Array<{ value: JunglePathing; label: string; icon: JSX.Element; description: string }> = [
  {
    value: "TopToBot",
    label: "Top → Bot",
    icon: <ArrowDown className="h-4 w-4" />,
    description: "Abrimos arriba para terminar jugando por bot side.",
  },
  {
    value: "BotToTop",
    label: "Bot → Top",
    icon: <ArrowUp className="h-4 w-4" />,
    description: "Abrimos abajo para impactar top en primeras ventanas.",
  },
];

const FIGHT_PLAN_OPTIONS: Array<{ value: FightPlan; label: string; icon: JSX.Element; description: string }> = [
  {
    value: "FrontToBack",
    label: "Front to back",
    icon: <Shield className="h-4 w-4" />,
    description: "Pelea ordenada: front line protege al carry.",
  },
  {
    value: "Pick",
    label: "Cazadas",
    icon: <Crosshair className="h-4 w-4" />,
    description: "Jugamos visión y cazadas para pelear en ventaja.",
  },
  {
    value: "Dive",
    label: "Invade",
    icon: <Zap className="h-4 w-4" />,
    description: "Entradas explosivas al backline para borrar carries.",
  },
  {
    value: "Siege",
    label: "Acecho",
    icon: <Brain className="h-4 w-4" />,
    description: "Presión de rango y estructura, sin overextender.",
  },
];

const SUPPORT_ROAMING_OPTIONS: Array<{ value: SupportRoaming; label: string; icon: JSX.Element; description: string }> = [
  {
    value: "Lane",
    label: "Jugar línea",
    icon: <Shield className="h-4 w-4" />,
    description: "Support prioriza 2v2 de bot, peel y control de oleada.",
  },
  {
    value: "RoamMid",
    label: "Rotar a mid",
    icon: <Compass className="h-4 w-4" />,
    description: "Después del reset, rota a mid para picks y control de visión.",
  },
  {
    value: "RoamTop",
    label: "Rotar a top",
    icon: <ArrowUpRight className="h-4 w-4" />,
    description: "Rotaciones tempranas a top para dives, grubs y tempo de mapa.",
  },
];

function normalizePosition(position: string): string {
  return position.toLowerCase().replace(/[^a-z]/g, "");
}

function positionToRole(position: string): DraftRole | null {
  const normalized = normalizePosition(position);
  if (normalized === "defender") return "TOP";
  if (normalized === "midfielder") return "JUNGLE";
  if (normalized === "attackingmidfielder") return "MID";
  if (normalized === "forward") return "ADC";
  if (normalized === "defensivemidfielder" || normalized === "goalkeeper") return "SUPPORT";
  return null;
}

function playerBaseOvr(player: GameStateData["players"][number]): number {
  const sum = OVR_KEYS.reduce((acc, key) => acc + Number(player.attributes[key] ?? 0), 0);
  return sum / OVR_KEYS.length;
}

function playerPhotoUrl(playerId: string): string | null {
  const match = playerId.match(/^lec-player-(.+)$/);
  if (!match) return null;
  return `/player-photos/${match[1]}.png`;
}

function Section<T extends string>({
  title,
  options,
  value,
  onChange,
}: {
  title: string;
  options: Array<{ value: T; label: string; icon: JSX.Element; description: string }>;
  value: T;
  onChange: (value: T) => void;
}) {
  return (
    <Card accent="primary">
      <CardHeader className="text-base">{title}</CardHeader>
      <CardBody className="p-4">
        <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
          {options.map((option) => {
            const active = option.value === value;
            return (
              <button
                key={option.value}
                className={`rounded-xl border-2 px-3 py-3 text-left transition-all ${
                  active
                    ? "border-primary-500 bg-primary-50 dark:bg-primary-500/10 shadow-md shadow-primary-500/10"
                    : "border-gray-200 dark:border-navy-600 hover:border-gray-300 dark:hover:border-navy-500"
                }`}
                onClick={() => onChange(option.value)}
              >
                <span className="mb-1 block text-base text-gray-700 dark:text-gray-200">{option.icon}</span>
                <span className="block font-heading text-sm font-bold uppercase tracking-wider text-gray-800 dark:text-gray-100">
                  {option.label}
                </span>
                <span className="mt-1 block text-[11px] leading-tight text-gray-500 dark:text-gray-400">
                  {option.description}
                </span>
              </button>
            );
          })}
        </div>
      </CardBody>
    </Card>
  );
}

export default function TacticsTab({
  gameState,
  onSelectPlayer: _onSelectPlayer,
  onGameUpdate,
}: TacticsTabProps): JSX.Element {
  const { t } = useTranslation();
  const myTeam = gameState.teams.find(
    (team) => team.id === gameState.manager.team_id,
  );

  const initial = useMemo<LolTacticsData>(() => {
    if (!myTeam?.lol_tactics) return DEFAULT_LOL_TACTICS;
    return {
      strong_side: myTeam.lol_tactics.strong_side,
      game_timing: myTeam.lol_tactics.game_timing,
      jungle_style: myTeam.lol_tactics.jungle_style,
      jungle_pathing: myTeam.lol_tactics.jungle_pathing,
      fight_plan: myTeam.lol_tactics.fight_plan,
      support_roaming: myTeam.lol_tactics.support_roaming ?? "Lane",
    };
  }, [myTeam?.lol_tactics]);

  const [tactics, setTactics] = useState<LolTacticsData>(initial);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setTactics(initial);
  }, [initial]);

  const roleModifiers = useMemo(() => computeRoleModifiers(tactics), [tactics]);
  const coherence = useMemo(() => computeCoherenceBreakdown(tactics), [tactics]);
  const coherenceScore = useMemo(
    () => coherence.reduce((sum, item) => sum + item.delta, 0),
    [coherence],
  );

  const roleImpactRows = useMemo(() => {
    if (!myTeam) return [];

    const teamPlayers = gameState.players.filter((player) => player.team_id === myTeam.id);
    const starterIds = new Set(myTeam.starting_xi_ids ?? []);

    const startersFirst = [
      ...teamPlayers.filter((player) => starterIds.has(player.id)),
      ...teamPlayers.filter((player) => !starterIds.has(player.id)),
    ];

    const pickedByRole = new Map<DraftRole, GameStateData["players"][number]>();
    startersFirst.forEach((player) => {
      const role = positionToRole(player.position);
      if (!role || pickedByRole.has(role)) return;
      pickedByRole.set(role, player);
    });

    return ROLE_ORDER.map((role) => {
      const player = pickedByRole.get(role) ?? null;
      const base = player ? playerBaseOvr(player) : 70;
      const modifier = roleModifiers[role] * 1.8;
      const variance = Math.max(0.5, Math.abs(roleModifiers[role]) * 0.6 + 0.6);
      const effective = base + modifier;
      return {
        role,
        playerId: player?.id ?? null,
        playerName: player?.match_name ?? "Sin titular definido",
        base,
        modifier,
        variance,
        effective,
      };
    });
  }, [gameState.players, myTeam, roleModifiers]);

  if (!myTeam) {
    return (
      <p className="text-gray-500 dark:text-gray-400">{t("common.noTeam")}</p>
    );
  }

  async function persist(next: LolTacticsData): Promise<void> {
    setTactics(next);
    setSaving(true);
    try {
      const updated = await invoke<GameStateData>("set_lol_tactics", {
        lolTactics: {
          strong_side: next.strong_side,
          game_timing: next.game_timing,
          jungle_style: next.jungle_style,
          jungle_pathing: next.jungle_pathing,
          fight_plan: next.fight_plan,
          support_roaming: next.support_roaming,
        },
      });
      onGameUpdate(updated);
    } catch (error) {
      console.error("Failed to set lol tactics:", error);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="mx-auto flex w-full max-w-7xl flex-col gap-5">
      <Card accent="accent">
        <CardHeader>Plan de juego</CardHeader>
        <CardBody>
          <p className="text-sm leading-relaxed text-gray-700 dark:text-gray-200">
          Estas tácticas definen cómo juega tu equipo en simulación. Se aplican al impacto por rol y al comportamiento macro
          (objetivos/tempo) durante la partida. También quedan listas para scrims, para ajustar antes del partido y entre mapas
          en series Bo3/Bo5.
          </p>
        </CardBody>
      </Card>

      <div className="grid grid-cols-1 items-start gap-5 xl:grid-cols-[1.6fr_1fr]">
        <div className="flex flex-col gap-4">
          <Section<GameTiming>
            title={t("tactics.gameTiming", { defaultValue: "Timing de partida" })}
            value={tactics.game_timing}
            onChange={(value) =>
              void persist({
                ...tactics,
                game_timing: value,
              })
            }
            options={GAME_TIMING_OPTIONS}
          />

          <Section<StrongSide>
            title={t("tactics.strongSide", { defaultValue: "Lado fuerte" })}
            value={tactics.strong_side}
            onChange={(value) =>
              void persist({
                ...tactics,
                strong_side: value,
              })
            }
            options={STRONG_SIDE_OPTIONS}
          />

          <Section<JungleStyle>
            title={t("tactics.jungleStyle", { defaultValue: "Estilo de JUNGLE" })}
            value={tactics.jungle_style}
            onChange={(value) =>
              void persist({
                ...tactics,
                jungle_style: value,
              })
            }
            options={JUNGLE_STYLE_OPTIONS}
          />

          <Section<JunglePathing>
            title={t("tactics.junglePathing", { defaultValue: "Ruta de JUNGLE" })}
            value={tactics.jungle_pathing}
            onChange={(value) =>
              void persist({
                ...tactics,
                jungle_pathing: value,
              })
            }
            options={JUNGLE_PATHING_OPTIONS}
          />

          <Section<FightPlan>
            title={t("tactics.fightPlan", { defaultValue: "Plan de teamfight" })}
            value={tactics.fight_plan}
            onChange={(value) =>
              void persist({
                ...tactics,
                fight_plan: value,
              })
            }
            options={FIGHT_PLAN_OPTIONS}
          />

          <Section<SupportRoaming>
            title={t("tactics.supportRoaming", { defaultValue: "Roaming de support" })}
            value={tactics.support_roaming}
            onChange={(value) =>
              void persist({
                ...tactics,
                support_roaming: value,
              })
            }
            options={SUPPORT_ROAMING_OPTIONS}
          />
        </div>

        <aside className="sticky top-2">
          <Card>
            <CardHeader>
            Impacto y coherencia
            </CardHeader>
            <CardBody className="p-4">

          <div className="rounded-lg border border-gray-200 bg-gray-50 p-3 dark:border-navy-600 dark:bg-navy-900/50">
            <p className="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">Coherencia táctica</p>
            <p className="text-lg font-heading font-bold text-gray-900 dark:text-gray-100">
              {coherenceScore >= 1 ? "Alta" : coherenceScore >= 0 ? "Media" : "Baja"}
            </p>
            <p className="text-xs text-gray-600 dark:text-gray-300 mt-1">
              Score: {coherenceScore > 0 ? "+" : ""}
              {coherenceScore.toFixed(2)}
            </p>
          </div>

          <div className="mt-3 space-y-2">
            {coherence.map((item) => (
              <div key={item.label} className="flex items-start justify-between gap-2 text-xs">
                <span className="text-gray-600 dark:text-gray-300">{item.label}</span>
                <span
                  className={`font-heading font-bold ${
                    item.delta >= 0 ? "text-emerald-600 dark:text-emerald-400" : "text-rose-600 dark:text-rose-400"
                  }`}
                >
                  {item.delta > 0 ? "+" : ""}
                  {item.delta.toFixed(2)}
                </span>
              </div>
            ))}
          </div>

          <div className="mt-4 border-t border-gray-100 pt-3 dark:border-navy-700">
            <p className="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">Impacto por rol</p>
            <div className="mt-2 space-y-2.5">
              {roleImpactRows.map((row) => (
                <div
                  key={row.role}
                  className="rounded-xl border border-gray-200 bg-white px-3 py-2.5 dark:border-navy-600 dark:bg-navy-800/40"
                >
                  <div className="flex items-center justify-between gap-3">
                    <div className="flex items-center gap-2.5 min-w-0">
                      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-gray-200 bg-gray-100 dark:border-white/10 dark:bg-navy-700">
                        <img
                          src={ROLE_ICON_URLS[row.role]}
                          alt={ROLE_META[row.role].name}
                          className="w-4 h-4 object-contain opacity-90"
                          loading="lazy"
                        />
                      </div>

                      {row.playerId ? (
                        <img
                          src={playerPhotoUrl(row.playerId) ?? ""}
                          alt={row.playerName}
                          className="h-10 w-10 shrink-0 rounded object-cover"
                          loading="lazy"
                        />
                      ) : (
                        <div className="h-10 w-10 shrink-0 rounded bg-gray-100 dark:bg-navy-700/40" />
                      )}

                      <div className="min-w-0">
                        <p className="truncate text-sm font-heading font-bold text-gray-900 dark:text-gray-100">
                          {row.playerName}
                        </p>
                        <p className="text-[11px] text-gray-500 dark:text-gray-300">
                          {Math.round(row.base)} OVR · {ROLE_META[row.role].name}
                        </p>
                      </div>
                    </div>

                    <div className="text-right shrink-0">
                      <p
                        className={`text-xl leading-none font-heading font-black ${
                          row.modifier >= 0 ? "text-emerald-400" : "text-rose-400"
                        }`}
                      >
                        {row.modifier >= 0 ? "+" : ""}
                        {row.modifier.toFixed(1)}
                      </p>
                      <p className="text-[10px] text-gray-500 dark:text-gray-400">
                        ±{row.variance.toFixed(1)} varianza
                      </p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>

          <p className="mt-4 text-xs text-gray-500 dark:text-gray-400">
            Tip: si el score de coherencia es bajo, intentá alinear lado fuerte + ruta de JUNGLE + timing.
          </p>
            </CardBody>
          </Card>
        </aside>
      </div>

      <p className="text-xs text-gray-500 dark:text-gray-400 px-1">
        {saving
          ? t("common.saving", { defaultValue: "Guardando..." })
          : "Los cambios se guardan automáticamente"}
      </p>
    </div>
  );
}
