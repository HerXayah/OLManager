import type { JSX } from "react";
import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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

const STRONG_SIDE_OPTIONS: Array<{ value: StrongSide; label: string; icon: string; description: string }> = [
  {
    value: "Top",
    label: "Top",
    icon: "🛡️",
    description: "Jugamos para top: prioridad de recursos y ganks arriba.",
  },
  {
    value: "Mid",
    label: "Mid",
    icon: "⚡",
    description: "El eje del mapa es mid: control de tempo y rotaciones.",
  },
  {
    value: "Bot",
    label: "Bot",
    icon: "🎯",
    description: "Invertimos en botlane para escalar peleas y objetivos.",
  },
];

const GAME_TIMING_OPTIONS: Array<{ value: GameTiming; label: string; icon: string; description: string }> = [
  {
    value: "Early",
    label: "Early game",
    icon: "🔥",
    description: "Buscamos ventaja antes de minuto 14 con ritmo agresivo.",
  },
  {
    value: "Mid",
    label: "Mid game",
    icon: "⚖️",
    description: "Pico de poder en mid game con setup de objetivos.",
  },
  {
    value: "Late",
    label: "Late game",
    icon: "📈",
    description: "Priorizamos escalado y ejecución en peleas largas.",
  },
];

const JUNGLE_STYLE_OPTIONS: Array<{ value: JungleStyle; label: string; icon: string; description: string }> = [
  {
    value: "Ganker",
    label: "Gankear",
    icon: "🎯",
    description: "JUNGLE de presión en líneas: castiga errores temprano.",
  },
  {
    value: "Invader",
    label: "Invadir",
    icon: "🗡️",
    description: "Entramos a JUNGLE rival para negar recursos y visión.",
  },
  {
    value: "Farmer",
    label: "Farmear",
    icon: "🌾",
    description: "Maximizamos farmeo para llegar fuertes a mid/late.",
  },
  {
    value: "Enabler",
    label: "Habilitar",
    icon: "🧠",
    description: "JUNGLE habilita carries con cobertura y tempo.",
  },
];

const JUNGLE_PATHING_OPTIONS: Array<{ value: JunglePathing; label: string; icon: string; description: string }> = [
  {
    value: "TopToBot",
    label: "Top → Bot",
    icon: "⬇️",
    description: "Abrimos arriba para terminar jugando por bot side.",
  },
  {
    value: "BotToTop",
    label: "Bot → Top",
    icon: "⬆️",
    description: "Abrimos abajo para impactar top en primeras ventanas.",
  },
];

const FIGHT_PLAN_OPTIONS: Array<{ value: FightPlan; label: string; icon: string; description: string }> = [
  {
    value: "FrontToBack",
    label: "Front to back",
    icon: "🧱",
    description: "Pelea ordenada: front line protege al carry.",
  },
  {
    value: "Pick",
    label: "Cazadas",
    icon: "🎣",
    description: "Jugamos visión y cazadas para pelear en ventaja.",
  },
  {
    value: "Dive",
    label: "Invade",
    icon: "🚀",
    description: "Entradas explosivas al backline para borrar carries.",
  },
  {
    value: "Siege",
    label: "Acecho",
    icon: "🏹",
    description: "Presión de rango y estructura, sin overextender.",
  },
];

const SUPPORT_ROAMING_OPTIONS: Array<{ value: SupportRoaming; label: string; icon: string; description: string }> = [
  {
    value: "Lane",
    label: "Jugar línea",
    icon: "🧷",
    description: "Support prioriza 2v2 de bot, peel y control de oleada.",
  },
  {
    value: "RoamMid",
    label: "Rotar a mid",
    icon: "🧭",
    description: "Después del reset, rota a mid para picks y control de visión.",
  },
  {
    value: "RoamTop",
    label: "Rotar a top",
    icon: "🗺️",
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
  options: Array<{ value: T; label: string; icon: string; description: string }>;
  value: T;
  onChange: (value: T) => void;
}) {
  return (
    <div className="rounded-xl border border-gray-100 dark:border-navy-700 bg-white dark:bg-navy-800/40 p-4">
      <p className="text-sm font-heading font-bold uppercase tracking-wide text-gray-700 dark:text-gray-200 mb-3">
        {title}
      </p>
      <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
        {options.map((option) => {
          const active = option.value === value;
          return (
            <button
              key={option.value}
              className={`rounded-lg border px-3 py-3 text-sm font-heading font-bold transition-colors ${
                active
                  ? "border-accent-400 text-accent-400 bg-accent-500/10"
                  : "border-gray-200 dark:border-navy-600 text-gray-700 dark:text-gray-200 bg-gray-50 dark:bg-navy-900/40 hover:border-primary-400"
              }`}
              onClick={() => onChange(option.value)}
            >
              <span className="block text-base mb-1">{option.icon}</span>
              <span className="block">{option.label}</span>
              <span className="block text-[11px] leading-tight font-normal mt-1 opacity-85">
                {option.description}
              </span>
            </button>
          );
        })}
      </div>
    </div>
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
    <div className="max-w-7xl mx-auto flex flex-col gap-4">
      <div className="rounded-xl border border-primary-200 dark:border-primary-900/50 bg-primary-50/70 dark:bg-primary-900/20 p-4">
        <p className="text-sm font-heading font-bold text-primary-800 dark:text-primary-200 uppercase tracking-wide">
          Plan de juego LoL
        </p>
        <p className="mt-2 text-sm text-primary-900/90 dark:text-primary-100/90 leading-relaxed">
          Estas tácticas definen cómo juega tu equipo en simulación. Se aplican al impacto por rol y al comportamiento macro
          (objetivos/tempo) durante la partida. También quedan listas para scrims, para ajustar antes del partido y entre mapas
          en series Bo3/Bo5.
        </p>
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-[1.6fr_1fr] gap-4 items-start">
        <div className="flex flex-col gap-4">
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

        <aside className="rounded-xl border border-gray-100 dark:border-navy-700 bg-white dark:bg-navy-800/40 p-4 sticky top-2">
          <h3 className="text-sm font-heading font-bold uppercase tracking-wide text-gray-700 dark:text-gray-200">
            Impacto y coherencia
          </h3>

          <div className="mt-3 rounded-lg border border-gray-100 dark:border-navy-700 bg-gray-50 dark:bg-navy-900/50 p-3">
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

          <div className="mt-4 border-t border-gray-100 dark:border-navy-700 pt-3">
            <p className="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">Impacto por rol</p>
            <div className="mt-2 space-y-2.5">
              {roleImpactRows.map((row) => (
                <div
                  key={row.role}
                  className="rounded-xl border border-[#1a2a4a] bg-[#0b1730] px-3 py-2.5"
                >
                  <div className="flex items-center justify-between gap-3">
                    <div className="flex items-center gap-2.5 min-w-0">
                      <div className="w-8 h-8 rounded-md bg-[#12284b] border border-white/10 flex items-center justify-center shrink-0">
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
                          className="w-10 h-10 object-cover shrink-0"
                          loading="lazy"
                        />
                      ) : (
                        <div className="w-10 h-10 bg-[#12284b]/40 shrink-0" />
                      )}

                      <div className="min-w-0">
                        <p className="text-sm font-heading font-bold truncate text-white">
                          {row.playerName}
                        </p>
                        <p className="text-[11px] text-blue-200/75">
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
                      <p className="text-[10px] text-blue-200/55">
                        ±{row.variance.toFixed(1)} varianza
                      </p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>

          <p className="text-xs text-gray-500 dark:text-gray-400 mt-4">
            Tip: si el score de coherencia es bajo, intentá alinear lado fuerte + ruta de JUNGLE + timing.
          </p>
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
