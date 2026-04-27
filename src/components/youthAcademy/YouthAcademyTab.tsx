import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { GraduationCap, Sparkles, Star, TrendingUp, Users } from "lucide-react";

import { calcAge } from "../../lib/helpers";
import { acquireAcademyTeam, getAcademyAcquisitionOptions, promoteAcademyPlayer } from "../../services/academyService";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import { findAcademyTeamForParent, getTeamAcademyRoster } from "../../store/academySelectors";
import type { AcademyAcquisitionOptionData } from "../../store/gameStore";
import { Badge, Button, Card, CardBody, CardHeader } from "../ui";
import { resolvePlayerLolRole } from "../../lib/lolIdentity";

interface YouthAcademyTabProps {
  gameState: GameStateData;
  onSelectPlayer?: (id: string) => void;
  onGameUpdate?: (state: GameStateData) => void;
}

type DraftRole = "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT";

const ROLE_ORDER: Record<DraftRole, number> = {
  TOP: 1,
  JUNGLE: 2,
  MID: 3,
  ADC: 4,
  SUPPORT: 5,
};

function roleVariant(role: DraftRole): "success" | "primary" | "accent" | "danger" | "neutral" {
  if (role === "TOP") return "success";
  if (role === "JUNGLE") return "primary";
  if (role === "MID") return "accent";
  if (role === "ADC") return "danger";
  return "neutral";
}

function getLolOvr(player: PlayerData): number {
  const attrs = player.attributes;
  const avg =
    (Number(attrs.dribbling ?? 0) +
      Number(attrs.shooting ?? 0) +
      Number(attrs.teamwork ?? 0) +
      Number(attrs.vision ?? 0) +
      Number(attrs.decisions ?? 0) +
      Number(attrs.leadership ?? 0) +
      Number(attrs.agility ?? 0) +
      Number(attrs.composure ?? 0) +
      Number(attrs.stamina ?? 0)) /
    9;
  return Math.max(1, Math.min(99, Math.round(avg)));
}

export default function YouthAcademyTab({ gameState, onSelectPlayer, onGameUpdate }: YouthAcademyTabProps) {
  const { t, i18n } = useTranslation();
  const numberLocale = i18n.language || "en";
  const myTeam = gameState.teams.find((team) => team.id === gameState.manager.team_id);
  const academyTeam = useMemo(
    () => findAcademyTeamForParent(gameState.teams, myTeam?.id),
    [gameState.teams, myTeam?.id],
  );

  const [promotingPlayerId, setPromotingPlayerId] = useState<string | null>(null);
  const [acquisitionOptions, setAcquisitionOptions] = useState<AcademyAcquisitionOptionData[]>([]);
  const [acquisitionBlockedReason, setAcquisitionBlockedReason] = useState<string | null>(null);
  const [acquisitionLoading, setAcquisitionLoading] = useState(false);
  const [acquiringSourceId, setAcquiringSourceId] = useState<string | null>(null);
  const [academyCustomName, setAcademyCustomName] = useState("");
  const [academyCustomShortName, setAcademyCustomShortName] = useState("");
  const [academyCustomLogoUrl, setAcademyCustomLogoUrl] = useState("");

  useEffect(() => {
    if (academyTeam || !myTeam?.id) {
      setAcquisitionOptions([]);
      setAcquisitionBlockedReason(null);
      return;
    }

    setAcquisitionLoading(true);
    void getAcademyAcquisitionOptions(myTeam.id)
      .then((response) => {
        setAcquisitionOptions(response.options ?? []);
        setAcquisitionBlockedReason(response.blocked_reason ?? null);
      })
      .catch(() => {
        setAcquisitionOptions([]);
        setAcquisitionBlockedReason(t("youthAcademy.loadOptionsError"));
      })
      .finally(() => {
        setAcquisitionLoading(false);
      });
  }, [academyTeam, myTeam?.id, t]);

  const youthPlayers = useMemo(
    () =>
      (myTeam ? getTeamAcademyRoster(gameState.teams, gameState.players, myTeam.id) : [])
        .map((player) => {
          const role = resolvePlayerLolRole(player);
          const ovr = getLolOvr(player);
          const age = calcAge(player.date_of_birth);
          const potential = player.potential_revealed ?? null;
          return { ...player, role, age, ovr, potential };
        })
        .sort((a, b) => {
          const byRole = ROLE_ORDER[a.role] - ROLE_ORDER[b.role];
          if (byRole !== 0) return byRole;
          const byOvr = b.ovr - a.ovr;
          if (byOvr !== 0) return byOvr;
          return (a.match_name || a.full_name).localeCompare(b.match_name || b.full_name);
        }),
    [gameState.players, gameState.teams, myTeam],
  );

  const avgOvr = youthPlayers.length > 0 ? Math.round(youthPlayers.reduce((sum, player) => sum + player.ovr, 0) / youthPlayers.length) : 0;
  const revealedPotentials = youthPlayers
    .map((player) => player.potential)
    .filter((value): value is number => typeof value === "number");
  const avgPotential =
    revealedPotentials.length > 0
      ? Math.round(revealedPotentials.reduce((sum, value) => sum + value, 0) / revealedPotentials.length)
      : null;
  const highPotential = revealedPotentials.filter((value) => value >= 75).length;
  const youthCoach = gameState.staff.filter((staff) => staff.team_id === myTeam?.id && staff.specialization === "Youth");

  if (!myTeam) {
    return (
      <div className="max-w-5xl mx-auto flex flex-col gap-5">
        <Card>
          <CardBody>
            <p className="text-sm text-gray-500 dark:text-gray-400">{t("youthAcademy.noYouthPlayers")}</p>
          </CardBody>
        </Card>
      </div>
    );
  }

  return (
    <div className="max-w-5xl mx-auto flex flex-col gap-5">
      <div className="flex items-center gap-3 flex-wrap">
        <GraduationCap className="w-5 h-5 text-primary-500" />
        <h2 className="text-lg font-heading font-bold text-gray-800 dark:text-gray-100 uppercase tracking-wider">
          {t("youthAcademy.title")}
        </h2>
        <Badge variant="neutral" size="sm">
          {`${youthPlayers.length} jugadores academia`}
        </Badge>
        {academyTeam && (
          <Badge variant="primary" size="sm">
            {academyTeam.name}
          </Badge>
        )}
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <Card>
          <CardBody>
            <div className="text-center">
              <Users className="w-5 h-5 text-gray-400 dark:text-gray-500 mx-auto mb-1" />
              <p className="font-heading font-bold text-2xl text-gray-800 dark:text-gray-100">{youthPlayers.length}</p>
              <p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
                Jugadores academia
              </p>
            </div>
          </CardBody>
        </Card>
        <Card>
          <CardBody>
            <div className="text-center">
              <Star className="w-5 h-5 text-accent-400 mx-auto mb-1" />
              <p className="font-heading font-bold text-2xl text-gray-800 dark:text-gray-100">{avgOvr}</p>
              <p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
                {t("youthAcademy.avgOvr")}
              </p>
            </div>
          </CardBody>
        </Card>
        <Card>
          <CardBody>
            <div className="text-center">
              <TrendingUp className="w-5 h-5 text-green-500 mx-auto mb-1" />
              <p className="font-heading font-bold text-2xl text-gray-800 dark:text-gray-100">{avgPotential ?? "??"}</p>
              <p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
                {t("youthAcademy.avgPotential")}
              </p>
            </div>
          </CardBody>
        </Card>
        <Card>
          <CardBody>
            <div className="text-center">
              <Sparkles className="w-5 h-5 text-accent-400 mx-auto mb-1" />
              <p className="font-heading font-bold text-2xl text-accent-500">{highPotential}</p>
              <p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
                {t("youthAcademy.highPotential")}
              </p>
            </div>
          </CardBody>
        </Card>
      </div>

      {youthCoach.length > 0 && (
        <Card>
          <CardBody>
            <div className="flex items-center gap-2 text-xs">
              <GraduationCap className="w-3.5 h-3.5 text-primary-500" />
              <span className="text-gray-500 dark:text-gray-400">{t("youthAcademy.youthCoach")}</span>
              {youthCoach.map((staff) => (
                <Badge key={staff.id} variant="primary" size="sm">
                  {staff.first_name} {staff.last_name} ({staff.attributes.coaching})
                </Badge>
              ))}
            </div>
          </CardBody>
        </Card>
      )}

      {!academyTeam && (
        <Card accent="accent">
          <CardHeader>{t("youthAcademy.academyCardTitle")}</CardHeader>
          <CardBody>
            <div className="flex flex-col gap-4">
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {acquisitionLoading
                  ? t("youthAcademy.acquisitionLoading")
                  : acquisitionOptions.length > 0
                    ? t("youthAcademy.acquisitionIntro")
                    : acquisitionBlockedReason ?? t("youthAcademy.acquisitionNoOptions")}
              </p>
              {acquisitionOptions.length > 0 && (
                <div className="grid gap-2 md:grid-cols-3">
                  <input
                    value={academyCustomName}
                    onChange={(event) => setAcademyCustomName(event.target.value)}
                    placeholder={t("youthAcademy.placeholderCustomName")}
                    className="w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white/80 dark:bg-navy-900 px-3 py-2 text-sm text-gray-700 dark:text-gray-200"
                  />
                  <input
                    value={academyCustomShortName}
                    onChange={(event) => setAcademyCustomShortName(event.target.value)}
                    placeholder={t("youthAcademy.placeholderCustomShortName")}
                    className="w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white/80 dark:bg-navy-900 px-3 py-2 text-sm text-gray-700 dark:text-gray-200"
                  />
                  <input
                    value={academyCustomLogoUrl}
                    onChange={(event) => setAcademyCustomLogoUrl(event.target.value)}
                    placeholder={t("youthAcademy.placeholderCustomLogoUrl")}
                    className="w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white/80 dark:bg-navy-900 px-3 py-2 text-sm text-gray-700 dark:text-gray-200"
                  />
                </div>
              )}
              {acquisitionOptions.length === 0 && (
                <Button size="sm" variant="outline" disabled>
                  {t("youthAcademy.fundAcademy")}
                </Button>
              )}
              {acquisitionOptions.length > 0 && (
                <div className="grid gap-3 md:grid-cols-2">
                  {acquisitionOptions.map((option) => (
                    <div key={option.source_team_id} className="rounded-lg border border-gray-100 dark:border-navy-600 p-4 flex items-center justify-between gap-3">
                      <div className="flex items-center gap-3 min-w-0">
                        <div className="w-10 h-10 rounded-lg bg-navy-700/40 border border-navy-600 flex items-center justify-center overflow-hidden shrink-0">
                          {option.source_team_logo_url ? (
                            <img
                              src={option.source_team_logo_url}
                              alt={t("youthAcademy.sourceTeamLogoAlt", { team: option.source_team_name })}
                              className="w-8 h-8 object-contain"
                              loading="lazy"
                            />
                          ) : (
                            <span className="text-[10px] font-heading text-gray-300">{option.source_team_short_name}</span>
                          )}
                        </div>
                        <div className="min-w-0">
                          <p className="text-sm font-bold text-gray-800 dark:text-gray-100 truncate">{option.source_team_name}</p>
                          <p className="text-xs text-gray-500 dark:text-gray-400 truncate">
                            {option.league_name} · {option.country} · €{option.acquisition_cost.toLocaleString(numberLocale)}
                          </p>
                        </div>
                      </div>
                      <Button
                        size="sm"
                        disabled={acquiringSourceId === option.source_team_id}
                        onClick={async () => {
                          if (!myTeam?.id) return;
                          try {
                            setAcquiringSourceId(option.source_team_id);
                            const updated = await acquireAcademyTeam({
                              parent_team_id: myTeam.id,
                              source_team_id: option.source_team_id,
                              custom_name: academyCustomName.trim() || undefined,
                              custom_short_name: academyCustomShortName.trim() || undefined,
                              custom_logo_url: academyCustomLogoUrl.trim() || undefined,
                            });
                            onGameUpdate?.(updated);
                          } finally {
                            setAcquiringSourceId(null);
                          }
                        }}
                      >
                        {acquiringSourceId === option.source_team_id
                          ? t("youthAcademy.fundingAcademy")
                          : t("youthAcademy.fundAcademy")}
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </CardBody>
        </Card>
      )}

      <Card>
        <CardHeader>
          {academyTeam ? t("youthAcademy.academyRosterLinked") : t("youthAcademy.academyNotLinked")}
        </CardHeader>
        <CardBody className="p-0">
          {youthPlayers.length === 0 ? (
            <div className="flex flex-col items-center gap-3 py-12">
              <GraduationCap className="w-10 h-10 text-gray-300 dark:text-navy-600" />
              <p className="text-sm text-gray-500 dark:text-gray-400">{t("youthAcademy.noYouthPlayers")}</p>
            </div>
          ) : (
            <table className="w-full text-left border-collapse">
              <thead>
                <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t("youthAcademy.player")}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">{t("youthAcademy.pos")}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t("youthAcademy.age")}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t("youthAcademy.ovr")}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t("youthAcademy.potential")}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">{t("youthAcademy.condition")}</th>
                  <th className="py-3 px-4 font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 text-center">
                    {t("common.actions")}
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
                {youthPlayers.map((player) => {
                  return (
                    <tr
                      key={player.id}
                      onClick={() => onSelectPlayer?.(player.id)}
                      className="hover:bg-gray-50 dark:hover:bg-navy-700/50 cursor-pointer transition-colors"
                    >
                      <td className="py-2.5 px-4">
                        <div>
                          <p className="text-sm font-medium text-gray-800 dark:text-gray-200">{player.match_name || player.id}</p>
                          <p className="text-xs text-gray-500 dark:text-gray-400">{player.full_name}</p>
                        </div>
                      </td>
                      <td className="py-2.5 px-4">
                        <Badge variant={roleVariant(player.role)} size="sm">
                          {player.role}
                        </Badge>
                      </td>
                      <td className="py-2.5 px-4 text-center text-sm text-gray-700 dark:text-gray-300">{player.age}</td>
                      <td className="py-2.5 px-4 text-center">
                        <span className="font-heading font-bold text-gray-800 dark:text-gray-100">{player.ovr}</span>
                      </td>
                      <td className="py-2.5 px-4 text-center">
                        <span className="font-heading font-bold text-accent-500">{player.potential ?? "??"}</span>
                      </td>
                      <td className="py-2.5 px-4 text-center text-sm font-medium text-gray-700 dark:text-gray-300">
                        {player.condition}%
                      </td>
                      <td className="py-2.5 px-4 text-center">
                        <Button
                          size="sm"
                          disabled={promotingPlayerId === player.id}
                          onClick={async (event) => {
                            event.stopPropagation();
                            try {
                              setPromotingPlayerId(player.id);
                              const updated = await promoteAcademyPlayer(player.id);
                              onGameUpdate?.(updated);
                            } finally {
                              setPromotingPlayerId(null);
                            }
                          }}
                        >
                          {promotingPlayerId === player.id ? t("youthAcademy.promoting") : t("youthAcademy.promote")}
                        </Button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </CardBody>
      </Card>
    </div>
  );
}
