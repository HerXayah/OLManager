import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { GraduationCap, Handshake, Sparkles, Star, TrendingUp, Users } from "lucide-react";

import { calcAge, calcOvr, positionBadgeVariant } from "../../lib/helpers";
import { countryName } from "../../lib/countries";
import { acquireAcademyTeam, getAcademyAcquisitionOptions } from "../../services/academyService";
import type { AcademyAcquisitionOptionData, GameStateData, PlayerData } from "../../store/gameStore";
import { Badge, Button, Card, CardBody, CardHeader, CountryFlag, ProgressBar } from "../ui";
import { TraitList } from "../TraitBadge";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";

interface YouthAcademyTabProps {
  gameState: GameStateData;
  onSelectPlayer?: (id: string) => void;
}

function estimatePotential(player: PlayerData): number {
  const ovr = calcOvr(player, player.natural_position || player.position);
  const age = calcAge(player.date_of_birth);
  return Math.min(99, Math.round(ovr + Math.max(0, (23 - age) * 2.5)));
}

function AcquisitionOptionCard({
  option,
  onAcquire,
  acquiring,
}: {
  option: AcademyAcquisitionOptionData;
  onAcquire: () => void;
  acquiring: boolean;
}) {
  return (
    <div className="rounded-lg border border-gray-100 dark:border-navy-600 p-4 flex flex-col gap-3">
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2 flex-wrap">
            <p className="text-sm font-bold text-gray-800 dark:text-gray-100">{option.source_team_name}</p>
            <Badge variant="neutral" size="sm">{option.source_team_short_name}</Badge>
          </div>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
            {option.league_name} · {countryName(option.country)} · {option.region}
          </p>
          {option.fallback_reason && <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">{option.fallback_reason}</p>}
        </div>
        <div className="text-right">
          <p className="font-heading font-bold text-accent-500 tabular-nums">{option.acquisition_cost}</p>
          <p className="text-[10px] uppercase tracking-wider text-gray-400 dark:text-gray-500">Coste</p>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-2 text-[10px] uppercase tracking-wider text-gray-400 dark:text-gray-500">
        <span>Reputación {option.reputation}</span>
        <span>Desarrollo {option.development_level}</span>
      </div>

      <div className="flex items-center justify-between gap-3">
        <p className="text-xs text-gray-500 dark:text-gray-400">{option.rebrand_allowed ? "Rebranding disponible" : "Rebranding bloqueado"}</p>
        <Button size="sm" onClick={onAcquire} disabled={acquiring}>
          {acquiring ? "Adquiriendo" : "Adquirir"}
        </Button>
      </div>
    </div>
  );
}

export default function YouthAcademyTab({ gameState, onSelectPlayer }: YouthAcademyTabProps) {
  const { t, i18n } = useTranslation();
  const myTeam = gameState.teams.find((tm) => tm.id === gameState.manager.team_id);

  const [acquisitionOptions, setAcquisitionOptions] = useState<AcademyAcquisitionOptionData[]>([]);
  const [acquisitionBlockedReason, setAcquisitionBlockedReason] = useState<string | null>(null);
  const [isLoadingAcquisitionOptions, setIsLoadingAcquisitionOptions] = useState(false);
  const [acquiringSourceId, setAcquiringSourceId] = useState<string | null>(null);
  const [acquisitionMessage, setAcquisitionMessage] = useState<string | null>(null);

  useEffect(() => {
    if (!myTeam?.id) {
      setAcquisitionOptions([]);
      setAcquisitionBlockedReason(null);
      return;
    }

    let cancelled = false;
    setIsLoadingAcquisitionOptions(true);

    getAcademyAcquisitionOptions(myTeam.id)
      .then((response) => {
        if (cancelled) return;
        setAcquisitionOptions(response.options);
        setAcquisitionBlockedReason(response.acquisition_allowed ? null : response.blocked_reason);
      })
      .catch(() => {
        if (!cancelled) {
          setAcquisitionOptions([]);
          setAcquisitionBlockedReason("Acquisition options unavailable");
        }
      })
      .finally(() => {
        if (!cancelled) setIsLoadingAcquisitionOptions(false);
      });

    return () => {
      cancelled = true;
    };
  }, [myTeam?.id]);

  const youthPlayers = useMemo(
    () =>
      (myTeam ? gameState.players.filter((p) => p.team_id === myTeam.id) : [])
        .map((p) => ({ ...p, age: calcAge(p.date_of_birth), ovr: calcOvr(p, p.natural_position || p.position), potential: estimatePotential(p) }))
        .filter((p) => p.age <= 21)
        .sort((a, b) => b.potential - a.potential),
    [gameState.players, myTeam],
  );

  const avgOvr = youthPlayers.length ? Math.round(youthPlayers.reduce((s, p) => s + p.ovr, 0) / youthPlayers.length) : 0;
  const avgPotential = youthPlayers.length ? Math.round(youthPlayers.reduce((s, p) => s + p.potential, 0) / youthPlayers.length) : 0;
  const highPotential = youthPlayers.filter((p) => p.potential >= 75).length;
  const youthCoach = gameState.staff.filter((s) => s.team_id === myTeam?.id && s.specialization === "Youth");

  const acquisitionSummary = acquisitionBlockedReason
    ? acquisitionBlockedReason
    : isLoadingAcquisitionOptions
      ? "Cargando opciones de adquisición..."
      : acquisitionOptions.length > 0
        ? `${acquisitionOptions.length} opciones de adquisición disponibles`
        : "No hay opciones de adquisición disponibles";

  if (!myTeam) {
    return (
      <div className="max-w-5xl mx-auto flex flex-col gap-5">
        <Card>
          <CardBody>
            <p className="text-sm text-gray-500 dark:text-gray-400">
              {t("youthAcademy.noYouthPlayers")}
            </p>
          </CardBody>
        </Card>
      </div>
    );
  }

  return (
    <div className="max-w-5xl mx-auto flex flex-col gap-5">
      <div className="flex items-center gap-3">
        <GraduationCap className="w-5 h-5 text-primary-500" />
        <h2 className="text-lg font-heading font-bold text-gray-800 dark:text-gray-100 uppercase tracking-wider">{t("youthAcademy.title")}</h2>
        <Badge variant="neutral" size="sm">{t("youthAcademy.playersUnder21", { count: youthPlayers.length })}</Badge>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <Card><CardBody><div className="text-center"><Users className="w-5 h-5 text-gray-400 dark:text-gray-500 mx-auto mb-1" /><p className="font-heading font-bold text-2xl text-gray-800 dark:text-gray-100">{youthPlayers.length}</p><p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">{t("youthAcademy.youthPlayers")}</p></div></CardBody></Card>
        <Card><CardBody><div className="text-center"><Star className="w-5 h-5 text-accent-400 mx-auto mb-1" /><p className="font-heading font-bold text-2xl text-gray-800 dark:text-gray-100">{avgOvr}</p><p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">{t("youthAcademy.avgOvr")}</p></div></CardBody></Card>
        <Card><CardBody><div className="text-center"><TrendingUp className="w-5 h-5 text-green-500 mx-auto mb-1" /><p className="font-heading font-bold text-2xl text-gray-800 dark:text-gray-100">{avgPotential}</p><p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">{t("youthAcademy.avgPotential")}</p></div></CardBody></Card>
        <Card><CardBody><div className="text-center"><Sparkles className="w-5 h-5 text-accent-400 mx-auto mb-1" /><p className="font-heading font-bold text-2xl text-accent-500">{highPotential}</p><p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">{t("youthAcademy.highPotential")}</p></div></CardBody></Card>
      </div>

      {youthCoach.length > 0 && <Card><CardBody><div className="flex items-center gap-2 text-xs"><GraduationCap className="w-3.5 h-3.5 text-primary-500" /><span className="text-gray-500 dark:text-gray-400">{t("youthAcademy.youthCoach")}</span>{youthCoach.map((s) => <Badge key={s.id} variant="primary" size="sm">{s.first_name} {s.last_name} ({s.attributes.coaching})</Badge>)}</div></CardBody></Card>}

      <Card accent="accent">
        <CardHeader action={<Badge variant={acquisitionBlockedReason ? "neutral" : "primary"} size="sm">{acquisitionBlockedReason ? "Bloqueado" : "Compra"}</Badge>}>
          Equipo ERL para adquirir
        </CardHeader>
        <CardBody>
          <div className="flex flex-col gap-5">
            <div className="flex items-start gap-3">
              <div className="w-12 h-12 rounded-xl bg-accent-100 dark:bg-accent-900/40 flex items-center justify-center">
                <Handshake className="w-6 h-6 text-accent-600 dark:text-accent-300" />
              </div>
              <div>
                <h3 className="font-heading font-bold text-xl text-gray-800 dark:text-gray-100">Comprar equipo ERL existente</h3>
                <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">{acquisitionSummary}</p>
                <p className="text-xs text-gray-400 dark:text-gray-500 mt-2">Backend autoritativo · sin recalcular elegibilidad en React.</p>
              </div>
            </div>

            {acquisitionMessage && <div className="rounded-lg border border-green-200 bg-green-50 px-4 py-3 text-sm text-green-700 dark:border-green-900 dark:bg-green-950/40 dark:text-green-300">{acquisitionMessage}</div>}

            {acquisitionOptions.length > 0 && (
              <div className="grid gap-3 md:grid-cols-2">
                {acquisitionOptions.map((option) => (
                  <AcquisitionOptionCard
                    key={option.source_team_id}
                    option={option}
                    acquiring={acquiringSourceId === option.source_team_id}
                    onAcquire={async () => {
                      setAcquiringSourceId(option.source_team_id);
                      setAcquisitionMessage(null);
                      try {
                        await acquireAcademyTeam({
                          parent_team_id: myTeam?.id ?? "",
                          source_team_id: option.source_team_id,
                        });
                        setAcquisitionMessage(`${option.source_team_name} adquirido`);
                      } finally {
                        setAcquiringSourceId(null);
                      }
                    }}
                  />
                ))}
              </div>
            )}
          </div>
        </CardBody>
      </Card>

      <Card>
        <CardHeader>Promesas del primer equipo</CardHeader>
        <CardBody className="p-0">
          {youthPlayers.length === 0 ? <div className="flex flex-col items-center gap-3 py-12"><GraduationCap className="w-10 h-10 text-gray-300 dark:text-navy-600" /><p className="text-sm text-gray-500 dark:text-gray-400">{t("youthAcademy.noYouthPlayers")}</p></div> : <table className="w-full text-left border-collapse"><thead><tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs"><th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t("youthAcademy.player")}</th><th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t("youthAcademy.pos")}</th><th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t("youthAcademy.age")}</th><th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t("youthAcademy.ovr")}</th><th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t("youthAcademy.potential")}</th><th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t("youthAcademy.growth")}</th><th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t("youthAcademy.traits")}</th><th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t("youthAcademy.condition")}</th></tr></thead><tbody className="divide-y divide-gray-100 dark:divide-navy-600">{youthPlayers.map((player) => { const growthRoom = player.potential - player.ovr; return (<tr key={player.id} onClick={() => onSelectPlayer?.(player.id)} className="hover:bg-gray-50 dark:hover:bg-navy-700/50 cursor-pointer transition-colors"><td className="py-2.5 px-4"><div><p className="text-sm font-medium text-gray-800 dark:text-gray-200">{player.full_name}</p><p className="text-xs text-gray-500 dark:text-gray-400">{player.match_name}</p></div></td><td className="py-2.5 px-4"><div className="flex flex-wrap gap-1">{translatePositionAbbreviation(player.position)}<Badge variant={positionBadgeVariant(player.position)} size="sm">{player.position}</Badge></div></td><td className="py-2.5 px-4 text-center text-sm text-gray-700 dark:text-gray-300">{player.age}</td><td className="py-2.5 px-4 text-center text-sm text-gray-700 dark:text-gray-300">{player.ovr}</td><td className="py-2.5 px-4 text-center text-sm text-accent-500 font-bold">{player.potential}</td><td className="py-2.5 px-4"><div className="flex items-center gap-2"><ProgressBar value={Math.min(100, Math.max(0, growthRoom * 10))} className="flex-1" /><span className="text-xs text-gray-500 dark:text-gray-400 w-12 text-right">+{growthRoom}</span></div></td><td className="py-2.5 px-4 text-xs text-gray-500 dark:text-gray-400"><TraitList traits={player.traits || []} locale={i18n.language} /></td><td className="py-2.5 px-4 text-center"><span className="inline-flex items-center justify-center rounded-full bg-gray-100 dark:bg-navy-700 px-2.5 py-1 text-xs font-bold text-gray-600 dark:text-gray-300">{player.condition}%</span></td></tr>); })}</tbody></table>}
        </CardBody>
      </Card>
    </div>
  );
}
