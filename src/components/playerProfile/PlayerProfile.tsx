import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getContractRiskLevel } from "../../lib/helpers";
import { calculateLolOvr } from "../../lib/lolPlayerStats";
import { PlayerData, GameStateData, PlayerMatchHistoryEntryData, ScoutReportData } from "../../store/gameStore";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { resolveBackendText } from "../../utils/backendI18n";
import { resolveSeasonContext } from "../../lib/seasonContext";
import DashboardModalFrame from "../dashboard/DashboardModalFrame";
import { Button } from "../ui";
import {
  getPlayerAge,
  getPlayerTeamName,
} from "./PlayerProfile.helpers";
import { buildPlayerAttributeGroups } from "./PlayerProfile.attributes";
import PlayerProfileAttributesCard from "./PlayerProfileAttributesCard";
import PlayerProfileContractCard from "./PlayerProfileContractCard";
import PlayerProfileHeroCard from "./PlayerProfileHeroCard";
import PlayerProfileInjuryBanner from "./PlayerProfileInjuryBanner";
import PlayerProfileRenewalModal from "./PlayerProfileRenewalModal";
import {
  type DelegatedRenewalCaseData,
  type DelegatedRenewalResponseData,
  type NegotiationFeedbackData,
  getRenewalStatusClassName,
  getRenewalStatusMessage,
  type RenewalProjectionData,
  type RenewalResponseData,
  type RenewalStatus,
  shouldDisableRenewalSubmit,
} from "./PlayerProfile.renewal";
import {
  getScoutAvailability,
  type PlayerProfileScoutStatus,
} from "./PlayerProfile.scouting";
import PlayerProfileChampionsCard from "./PlayerProfileChampionsCard";
import playersSeed from "../../../data/lec/draft/players.json";
import championsSeed from "../../../data/lec/draft/champions.json";
import { startPotentialResearch } from "../../services/playerService";
import { demoteMainPlayerToAcademy, promoteAcademyPlayer } from "../../services/academyService";
import { findAcademyTeamForParent } from "../../store/academySelectors";
import { fallbackChampionForRole, resolvePlayerLolRole } from "../../lib/lolIdentity";
import {
  makeTransferBid,
  releasePlayerContract,
  type TransferNegotiationFeedbackData,
} from "../../services/transfersService";
import { formatVal } from "../../lib/helpers";

type LolRole = "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT";

interface TransferOfferFeedbackState {
  decision: "accepted" | "rejected" | "counter_offer";
  feedback: TransferNegotiationFeedbackData;
}

function getLatestScoutReportForPlayer(
  gameState: GameStateData,
  playerId: string,
): ScoutReportData | null {
  return gameState.messages
    .filter((message) => message.context?.scout_report?.player_id === playerId)
    .sort((left, right) => right.date.localeCompare(left.date))[0]
    ?.context.scout_report ?? null;
}

interface PlayerSeed {
  ign: string;
  role: string;
  champions: Array<Array<string | number>>;
}

const PLAYER_SEEDS: PlayerSeed[] = [
  ...(((playersSeed as { data?: { rostered_seeds?: PlayerSeed[] } }).data?.rostered_seeds ?? []) as PlayerSeed[]),
  ...(((playersSeed as { data?: { free_agent_seeds?: PlayerSeed[] } }).data?.free_agent_seeds ?? []) as PlayerSeed[]),
];

const CHAMPION_ALIASES = (
  championsSeed as { data?: { display_aliases?: Record<string, string>; roles?: Record<string, unknown> } }
).data?.display_aliases ?? {};

const CHAMPION_ROLE_KEYS = Object.keys(
  (
    championsSeed as { data?: { roles?: Record<string, unknown> } }
  ).data?.roles ?? {},
);

const CHAMPION_ID_BY_NORMALIZED_NAME = new Map<string, string>([
  ...Object.entries(CHAMPION_ALIASES).map(([alias, id]) => [normalizeKey(alias), id] as const),
  ...CHAMPION_ROLE_KEYS.map((id) => [normalizeKey(id), id] as const),
]);

function normalizeKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]/g, "");
}

function championIdFromName(name: string): string | null {
  const normalized = normalizeKey(name);
  if (!normalized) return null;

  const fromCatalog = CHAMPION_ID_BY_NORMALIZED_NAME.get(normalized);
  if (fromCatalog) return fromCatalog;

  const overrides: Record<string, string> = {
    aurelionsol: "AurelionSol",
    belveth: "Belveth",
    chogath: "Chogath",
    drmundo: "DrMundo",
    jarvaniv: "JarvanIV",
    ksante: "KSante",
    kaisa: "Kaisa",
    khazix: "Khazix",
    kogmaw: "KogMaw",
    leblanc: "Leblanc",
    leesin: "LeeSin",
    monkeyking: "MonkeyKing",
    nunuandwillump: "Nunu",
    reksai: "RekSai",
    tahmkench: "TahmKench",
    twistedfate: "TwistedFate",
    velkoz: "Velkoz",
  };

  if (overrides[normalized]) return overrides[normalized];
  return normalized.charAt(0).toUpperCase() + normalized.slice(1);
}

function buildChampionPerformanceMap(
  history: PlayerMatchHistoryEntryData[],
): Map<string, { wr: number; games: number }> {
  const bucket = new Map<string, { wins: number; games: number }>();

  history.forEach((entry) => {
    const championId = entry.championId;
    if (!championId) return;

    const current = bucket.get(championId) ?? { wins: 0, games: 0 };
    current.games += 1;
    const won = entry.result === "Win";
    if (won) {
      current.wins += 1;
    }
    bucket.set(championId, current);
  });

  return new Map(
    [...bucket.entries()].map(([championId, value]) => [
      championId,
      {
        games: value.games,
        wr:
          value.games > 0
            ? Number(((value.wins / value.games) * 100).toFixed(1))
            : 0,
      },
    ]),
  );
}

function buildTopChampionMasteries(
  playerId: string,
  matchName: string,
  role: "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT",
  championPerformance: Map<string, { wr: number; games: number }>,
  visibleChampionCount = 4,
) {
  const seed = PLAYER_SEEDS.find((entry) => normalizeKey(entry.ign) === normalizeKey(matchName));
  const champions = [...(seed?.champions ?? [])]
    .map((entry) => ({
      championName: String(entry[0] ?? ""),
      mastery: Number(entry[1] ?? 0),
    }))
    .filter((entry) => entry.championName.length > 0)
    .sort((a, b) => b.mastery - a.mastery);

  if (champions.length === 0) {
    const fallbackChampion = fallbackChampionForRole(playerId, role);
    if (!fallbackChampion) return [];
    return [
      {
        championId: fallbackChampion,
        championName: fallbackChampion,
        mastery: 100,
        rank: "insignia" as const,
        wr: championPerformance.get(fallbackChampion)?.wr ?? 0,
        games: championPerformance.get(fallbackChampion)?.games ?? 0,
      },
    ];
  }

  const insignia = champions[0];
  const rest = champions.slice(1, Math.max(1, visibleChampionCount));

  const firstId = championIdFromName(insignia.championName);
  if (!firstId) return [];

  return [
    {
      championId: firstId,
      championName: insignia.championName,
      mastery: Math.max(100, insignia.mastery),
      rank: "insignia" as const,
      wr: championPerformance.get(firstId)?.wr ?? 0,
      games: championPerformance.get(firstId)?.games ?? 0,
    },
    ...rest
      .map((entry, idx) => {
        const championId = championIdFromName(entry.championName);
        if (!championId) return null;
        return {
          championId,
          championName: entry.championName,
          mastery: entry.mastery,
          rank: (idx + 1) as 1 | 2 | 3,
          wr: championPerformance.get(championId)?.wr ?? 0,
          games: championPerformance.get(championId)?.games ?? 0,
        };
      })
      .filter(
        (entry): entry is {
          championId: string;
          championName: string;
          mastery: number;
          rank: 1 | 2 | 3;
          wr: number;
          games: number;
        } => entry !== null,
      ),
  ];
}

interface PlayerProfileProps {
  player: PlayerData;
  gameState: GameStateData;
  isOwnClub: boolean;
  startWithRenewalModal?: boolean;
  onClose: () => void;
  onSelectTeam?: (id: string) => void;
  onGameUpdate?: (g: GameStateData) => void;
}

export default function PlayerProfile({
  player,
  gameState,
  startWithRenewalModal = false,
  onClose,
  onSelectTeam,
  onGameUpdate,
}: PlayerProfileProps) {
  const { t, i18n } = useTranslation();
  const weeklySuffix = t("finances.perWeekSuffix", "/wk");
  const primaryRole = resolvePlayerLolRole(player);

  if (!player) {
    return null;
  }

  const [scoutStatus, setScoutStatus] = useState<PlayerProfileScoutStatus>(
    "idle",
  );
  const [academyActionSubmitting, setAcademyActionSubmitting] = useState(false);
  const [playerHistory, setPlayerHistory] = useState<PlayerMatchHistoryEntryData[]>([]);
  const [rerollingRole, setRerollingRole] = useState(false);
  const [potentialResearchSubmitting, setPotentialResearchSubmitting] = useState(false);
  const [scoutError, setScoutError] = useState<string | null>(null);
  const [showRenewalModal, setShowRenewalModal] = useState(false);
  const [showReleaseContractModal, setShowReleaseContractModal] = useState(false);
  const [showTransferOfferModal, setShowTransferOfferModal] = useState(false);
  const [transferActionSubmitting, setTransferActionSubmitting] = useState(false);
  const [transferOfferAmount, setTransferOfferAmount] = useState("");
  const [transferOfferError, setTransferOfferError] = useState<string | null>(null);
  const [transferOfferFeedback, setTransferOfferFeedback] =
    useState<TransferOfferFeedbackState | null>(null);
  const [renewalWage, setRenewalWage] = useState("");
  const [renewalLength, setRenewalLength] = useState("2");
  const [renewalSubmitting, setRenewalSubmitting] = useState(false);
  const [renewalStatus, setRenewalStatus] = useState<RenewalStatus>("idle");
  const [renewalError, setRenewalError] = useState<string | null>(null);
  const [renewalSuggestedWage, setRenewalSuggestedWage] = useState<
    number | null
  >(null);
  const [renewalSuggestedYears, setRenewalSuggestedYears] = useState<
    number | null
  >(null);
  const [renewalSessionStatus, setRenewalSessionStatus] =
    useState<RenewalResponseData["session_status"]>("idle");
  const [renewalIsTerminal, setRenewalIsTerminal] = useState(false);
  const [renewalCooledOff, setRenewalCooledOff] = useState(false);
  const [renewalFeedback, setRenewalFeedback] =
    useState<NegotiationFeedbackData | null>(null);
  const [renewalProjection, setRenewalProjection] =
    useState<RenewalProjectionData["projection"] | null>(null);
  const [hasConsumedInitialRenewalIntent, setHasConsumedInitialRenewalIntent] =
    useState(false);
  const ovr = calculateLolOvr(player);
  const age = getPlayerAge(player.date_of_birth, gameState.clock.current_date);
  const teamName = getPlayerTeamName(
    gameState.teams,
    player.team_id,
    {
      freeAgent: t("common.freeAgent"),
      unknown: t("common.unknown"),
    },
  );
  const managerTeamId = gameState.manager.team_id;
  const managerAcademyTeam = findAcademyTeamForParent(gameState.teams, managerTeamId);
  const isOwnMainPlayer = managerTeamId !== null && player.team_id === managerTeamId;
  const isOwnAcademyPlayer = managerAcademyTeam !== null && player.team_id === managerAcademyTeam.id;
  const actualIsOwnClub = isOwnMainPlayer || isOwnAcademyPlayer;
  const seasonContext = resolveSeasonContext(gameState);
  const isTransferWindowOpen =
    seasonContext.transfer_window.status === "Open" ||
    seasonContext.transfer_window.status === "DeadlineDay";
  const releasePenaltyPreview = (() => {
    if (!player.contract_end) {
      return 0;
    }

    const currentDate = new Date(gameState.clock.current_date);
    const contractEndDate = new Date(`${player.contract_end}T00:00:00Z`);
    if (Number.isNaN(currentDate.getTime()) || Number.isNaN(contractEndDate.getTime())) {
      return 0;
    }

    const msPerDay = 24 * 60 * 60 * 1000;
    const daysRemaining = Math.max(
      0,
      Math.ceil((contractEndDate.getTime() - currentDate.getTime()) / msPerDay),
    );
    const remainingSalary = Math.round((player.wage * daysRemaining) / 365);
    return Math.round(remainingSalary * 0.4);
  })();
  const contractRiskLevel = getContractRiskLevel(
    player.contract_end,
    gameState.clock.current_date,
  );
  const contractRiskLabel =
    contractRiskLevel === "critical"
      ? t("finances.contractRiskCritical")
      : contractRiskLevel === "warning"
        ? t("finances.contractRiskWarning")
        : t("finances.contractRiskStable");
  const renewalOfferedWage = Number(renewalWage);
  const renewalOfferedYears = Number(renewalLength);
  const isRenewalWageValid =
    Number.isFinite(renewalOfferedWage) && renewalOfferedWage > 0;
  const isRenewalLengthValid =
    Number.isInteger(renewalOfferedYears) && renewalOfferedYears > 0;
  const renewalViolatesSoftCap =
    isRenewalWageValid &&
    renewalProjection !== null &&
    !renewalProjection.policy_allows;
  const renewalSubmitDisabled = shouldDisableRenewalSubmit({
    renewalSubmitting,
    renewalIsTerminal,
    isRenewalWageValid,
    isRenewalLengthValid,
    renewalViolatesSoftCap,
  });
  const renewalStatusMessage = getRenewalStatusMessage(
    {
      renewalSessionStatus,
      renewalStatus,
      renewalSuggestedWage,
      renewalSuggestedYears,
      renewalError,
    },
    t,
  );
  const renewalStatusClassName = getRenewalStatusClassName(renewalStatus);
  const scoutAvailability = getScoutAvailability({
    staff: gameState.staff,
    scoutingAssignments: gameState.scouting_assignments || [],
    managerTeamId: gameState.manager.team_id,
    playerId: player.id,
    scoutStatus,
  });
  const latestScoutReport = getLatestScoutReportForPlayer(gameState, player.id);
  const attrGroups = buildPlayerAttributeGroups(
    player,
    t,
    actualIsOwnClub ? undefined : latestScoutReport,
  );
  const canViewAttributes = actualIsOwnClub || attrGroups.some((group) =>
    group.attrs.some((attribute) => attribute.value !== null),
  );
  const championPerformance = buildChampionPerformanceMap(playerHistory);
  const visibleChampionMasteryCount = actualIsOwnClub ? 4 : latestScoutReport ? 2 : 1;
  const topChampions = buildTopChampionMasteries(
    player.id,
    player.match_name,
    primaryRole,
    championPerformance,
    visibleChampionMasteryCount,
  );
  const activePotentialResearchPlayer = gameState.players.find(
    (candidate) => (candidate.potential_research_eta_days ?? 0) > 0,
  );
  const isPotentialResearchActiveForPlayer = activePotentialResearchPlayer?.id === player.id;
  const isPotentialResearchBlockedByOther =
    Boolean(activePotentialResearchPlayer) && !isPotentialResearchActiveForPlayer;

  useEffect(() => {
    let cancelled = false;

    const loadHistory = async (): Promise<void> => {
      try {
        const history = await invoke<PlayerMatchHistoryEntryData[]>("get_player_match_history", {
          playerId: player.id,
          limit: 500,
        });
        if (!cancelled) {
          setPlayerHistory(history);
        }
      } catch {
        if (!cancelled) {
          setPlayerHistory([]);
        }
      }
    };

    void loadHistory();

    return () => {
      cancelled = true;
    };
  }, [player.id]);

  async function handleRerollRole(role: LolRole): Promise<void> {
    if (!actualIsOwnClub || !onGameUpdate || rerollingRole) {
      return;
    }

    setRerollingRole(true);
    try {
      const updated = await invoke<GameStateData>("reroll_player_lol_role", {
        playerId: player.id,
        role,
      });
      onGameUpdate(updated);
    } catch {
      return;
    } finally {
      setRerollingRole(false);
    }
  }

  function handleRequestReleaseContract(): void {
    if (!isTransferWindowOpen || !actualIsOwnClub || transferActionSubmitting) {
      return;
    }

    setShowReleaseContractModal(true);
  }

  async function handleConfirmReleaseContract(): Promise<void> {
    if (!onGameUpdate || !actualIsOwnClub || transferActionSubmitting) {
      return;
    }

    setTransferActionSubmitting(true);
    try {
      const updated = await releasePlayerContract(player.id);
      onGameUpdate(updated);
      setShowReleaseContractModal(false);
      onClose();
    } catch (error) {
      console.error("Failed to release player contract:", error);
    } finally {
      setTransferActionSubmitting(false);
    }
  }

  function handleOpenTransferOfferModal(): void {
    if (!onGameUpdate || actualIsOwnClub || transferActionSubmitting || !isTransferWindowOpen) {
      return;
    }

    const initialFee = Math.max(1, Math.round(player.market_value));
    setTransferOfferAmount(String(initialFee));
    setTransferOfferError(null);
    setTransferOfferFeedback(null);
    setShowTransferOfferModal(true);
  }

  async function handleSubmitTransferOffer(): Promise<void> {
    if (!onGameUpdate || actualIsOwnClub || transferActionSubmitting) {
      return;
    }

    const fee = Math.round(Number.parseFloat(transferOfferAmount));
    if (!Number.isFinite(fee) || fee <= 0) {
      setTransferOfferError(
        t("playerProfile.transferOfferInvalid", {
          defaultValue: "Ingresá un monto de oferta valido.",
        }),
      );
      return;
    }

    setTransferOfferError(null);
    setTransferOfferFeedback(null);
    setTransferActionSubmitting(true);
    try {
      const result = await makeTransferBid(player.id, fee);
      onGameUpdate(result.game);
      setTransferOfferFeedback({
        decision: result.decision,
        feedback: result.feedback,
      });

      if (result.suggested_fee !== null) {
        setTransferOfferAmount(String(Math.round(result.suggested_fee)));
      }

      if (result.decision === "accepted") {
        setShowTransferOfferModal(false);
      }
    } catch (error) {
      console.error("Failed to make transfer offer:", error);
      const rawError =
        typeof error === "string"
          ? error
          : error instanceof Error
            ? error.message
            : String(error);

      if (rawError.includes("Transfer budget too low")) {
        setTransferOfferError(
          t("playerProfile.transferOfferBudgetError", {
            defaultValue: "Tu presupuesto de fichajes no alcanza para este monto.",
          }),
        );
      } else if (rawError.includes("Insufficient funds")) {
        setTransferOfferError(
          t("playerProfile.transferOfferFundsError", {
            defaultValue: "No tenes fondos suficientes para esta oferta.",
          }),
        );
      } else if (rawError.includes("Transfer window is closed")) {
        setTransferOfferError(
          t("playerProfile.transferOfferWindowClosed", {
            defaultValue: "El mercado esta cerrado.",
          }),
        );
      } else {
        setTransferOfferError(
          t("playerProfile.transferOfferFailed", {
            defaultValue: "No se pudo enviar la oferta. Probá de nuevo.",
          }),
        );
      }
    } finally {
      setTransferActionSubmitting(false);
    }
  }

  function openRenewalModal(): void {
    setRenewalWage(String(player.wage));
    setRenewalLength("2");
    setRenewalSubmitting(false);
    setRenewalStatus("idle");
    setRenewalError(null);
    setRenewalSuggestedWage(null);
    setRenewalSuggestedYears(null);
    setRenewalSessionStatus("idle");
    setRenewalIsTerminal(false);
    setRenewalCooledOff(false);
    setRenewalFeedback(null);
    setRenewalProjection(null);
    setShowRenewalModal(true);
  }

  function closeRenewalModal(): void {
    if (renewalSubmitting) {
      return;
    }

    setShowRenewalModal(false);
  }

  useEffect(() => {
    setHasConsumedInitialRenewalIntent(false);
  }, [player.id, startWithRenewalModal]);

  useEffect(() => {
    if (
      !actualIsOwnClub ||
      !startWithRenewalModal ||
      showRenewalModal ||
      hasConsumedInitialRenewalIntent
    ) {
      return;
    }

    setHasConsumedInitialRenewalIntent(true);
    openRenewalModal();
  }, [
    hasConsumedInitialRenewalIntent,
    actualIsOwnClub,
    showRenewalModal,
    startWithRenewalModal,
  ]);

  useEffect(() => {
    if (!showRenewalModal || !isRenewalWageValid) {
      setRenewalProjection(null);
      return;
    }

    let cancelled = false;

    const loadProjection = async (): Promise<void> => {
      try {
        const result = await invoke<RenewalProjectionData>(
          "preview_renewal_financial_impact",
          {
            playerId: player.id,
            weeklyWage: renewalOfferedWage,
          },
        );

        if (!cancelled) {
          setRenewalProjection(result.projection ?? null);
        }
      } catch {
        if (!cancelled) {
          setRenewalProjection(null);
        }
      }
    };

    loadProjection();

    return () => {
      cancelled = true;
    };
  }, [isRenewalWageValid, player.id, renewalOfferedWage, showRenewalModal]);

  async function handleRenewalSubmit(): Promise<void> {
    if (renewalSubmitDisabled) {
      return;
    }

    setRenewalSubmitting(true);
    setRenewalStatus("idle");
    setRenewalError(null);
    setRenewalCooledOff(false);

    try {
      const result = await invoke<RenewalResponseData>("propose_renewal", {
        playerId: player.id,
        weeklyWage: renewalOfferedWage,
        contractYears: renewalOfferedYears,
      });

      onGameUpdate?.(result.game);
      setRenewalStatus(result.outcome);
      setRenewalSuggestedWage(result.suggested_wage);
      setRenewalSuggestedYears(result.suggested_years);
      setRenewalSessionStatus(result.session_status);
      setRenewalIsTerminal(result.is_terminal);
      setRenewalCooledOff(result.cooled_off ?? false);
      setRenewalFeedback(result.feedback ?? null);

      if (result.session_status === "blocked") {
        setRenewalStatus("blocked");
      }

      if (result.outcome === "counter_offer") {
        if (result.suggested_wage !== null) {
          setRenewalWage(String(result.suggested_wage));
        }

        if (result.suggested_years !== null) {
          setRenewalLength(String(result.suggested_years));
        }
      }
    } catch (error) {
      setRenewalStatus("error");
      setRenewalError(String(error));
      setRenewalCooledOff(false);
    } finally {
      setRenewalSubmitting(false);
    }
  }

  async function handleDelegateRenewal(): Promise<void> {
    if (renewalSubmitting) {
      return;
    }

    setRenewalSubmitting(true);
    setRenewalError(null);
    setRenewalCooledOff(false);

    try {
      const result = await invoke<DelegatedRenewalResponseData>(
        "delegate_renewals",
        {
          playerIds: [player.id],
          maxWageIncreasePct: 35,
          maxContractYears: 3,
        },
      );

      onGameUpdate?.(result.game);
      const delegatedCase: DelegatedRenewalCaseData | undefined =
        result.report.cases.find(
          (renewalCase) => renewalCase.player_id === player.id,
        );

      if (!delegatedCase) {
        setRenewalStatus("error");
        setRenewalError(t("playerProfile.renewalDelegateMissingReport"));
        return;
      }

      if (delegatedCase.status === "successful") {
        setRenewalStatus("accepted");
        setRenewalSessionStatus("agreed");
        setRenewalIsTerminal(true);
        setRenewalSuggestedWage(null);
        setRenewalSuggestedYears(null);
        setRenewalCooledOff(false);
        setRenewalFeedback(null);
        return;
      }

      if (delegatedCase.status === "stalled") {
        setRenewalStatus("rejected");
        setRenewalSessionStatus("stalled");
        setRenewalIsTerminal(false);
        setRenewalCooledOff(false);
        setRenewalFeedback(null);
        setRenewalError(
          resolveBackendText(
            delegatedCase.note_key,
            delegatedCase.note,
            delegatedCase.note_params,
          ),
        );
        return;
      }

      setRenewalStatus("blocked");
      setRenewalSessionStatus("blocked");
      setRenewalIsTerminal(true);
      setRenewalCooledOff(false);
      setRenewalFeedback(null);
      setRenewalError(
        resolveBackendText(
          delegatedCase.note_key,
          delegatedCase.note,
          delegatedCase.note_params,
        ),
      );
    } catch (error) {
      setRenewalStatus("error");
      setRenewalError(String(error));
      setRenewalCooledOff(false);
    } finally {
      setRenewalSubmitting(false);
    }
  }

  return (
    <div className="max-w-6xl mx-auto">
      <button
        onClick={onClose}
        className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 transition-colors mb-4"
      >
        <ArrowLeft className="w-4 h-4" />
        <span className="font-heading font-bold uppercase tracking-wider">
          {t("common.back")}
        </span>
      </button>

      <PlayerProfileHeroCard
        player={player}
        ovr={ovr}
        primaryRole={primaryRole}
        age={age}
        teamName={teamName}
        weeklySuffix={weeklySuffix}
        language={i18n.language}
        isOwnClub={actualIsOwnClub || !onGameUpdate}
        scoutAvailability={scoutAvailability}
        scoutStatus={scoutStatus}
        scoutError={scoutError}
        onScout={() => {
          const availableScout = scoutAvailability.availableScout;
          if (!availableScout || !onGameUpdate) {
            return;
          }

          void (async () => {
            setScoutStatus("sending");
            setScoutError(null);

            try {
              const updated = await invoke<GameStateData>("send_scout", {
                scoutId: availableScout.id,
                playerId: player.id,
              });
              onGameUpdate(updated);
              setScoutStatus("sent");
            } catch (err) {
              setScoutError(String(err));
              setScoutStatus("error");
            }
          })();
        }}
        onRerollRole={(role) => {
          void handleRerollRole(role);
        }}
        rerollingRole={rerollingRole}
        insigniaChampionId={topChampions[0]?.championId ?? null}
        onSelectTeam={onSelectTeam}
        academyActionLabel={
          isOwnAcademyPlayer
            ? "Subir al primer equipo"
            : isOwnMainPlayer && managerAcademyTeam
              ? "Bajar a academia"
              : null
        }
        academyActionLoading={academyActionSubmitting}
        onAcademyAction={
          isOwnAcademyPlayer || (isOwnMainPlayer && managerAcademyTeam)
            ? () => {
                if (!onGameUpdate || academyActionSubmitting) {
                  return;
                }

                void (async () => {
                  setAcademyActionSubmitting(true);
                  try {
                    const updated = isOwnAcademyPlayer
                      ? await promoteAcademyPlayer(player.id)
                      : await demoteMainPlayerToAcademy(player.id);
                    onGameUpdate(updated);
                  } catch {
                    return;
                  } finally {
                    setAcademyActionSubmitting(false);
                  }
                })();
              }
            : null
        }
        onStartPotentialResearch={
          onGameUpdate
            ? () => {
                if (potentialResearchSubmitting) {
                  return;
                }

                void (async () => {
                  setPotentialResearchSubmitting(true);
                  try {
                    const updated = await startPotentialResearch(player.id);
                    onGameUpdate(updated);
                  } catch {
                    return;
                  } finally {
                    setPotentialResearchSubmitting(false);
                  }
                })();
              }
            : undefined
        }
        potentialResearchSubmitting={potentialResearchSubmitting}
        isPotentialResearchBlockedByOther={isPotentialResearchBlockedByOther}
        t={t}
      />

      {/* Injury banner */}
      {player.injury ? (
        <PlayerProfileInjuryBanner injury={player.injury} t={t} />
      ) : null}

      {/* Main content grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
        <PlayerProfileContractCard
          dateOfBirth={player.date_of_birth}
          contractEnd={player.contract_end}
          currentDate={gameState.clock.current_date}
          condition={player.condition}
          morale={player.morale}
          marketValue={player.market_value}
          wage={player.wage}
          weeklySuffix={weeklySuffix}
          language={i18n.language}
          contractRiskLevel={contractRiskLevel}
          contractRiskLabel={contractRiskLabel}
          isOwnClub={actualIsOwnClub}
          isTransferWindowOpen={isTransferWindowOpen}
          transferActionSubmitting={transferActionSubmitting}
          onOpenRenewal={openRenewalModal}
          onReleaseContract={handleRequestReleaseContract}
          onOpenTransferBid={handleOpenTransferOfferModal}
          t={t}
        />

        <div className="lg:col-span-2 flex flex-col gap-5">
          <PlayerProfileAttributesCard
            attrGroups={attrGroups}
            canViewAttributes={canViewAttributes}
            title={t("playerProfile.attributes")}
            averageLabel={t("common.average")}
            hiddenTitle={t("playerProfile.attributesHidden")}
            hiddenBody={t("playerProfile.scoutToView")}
          />

          {topChampions.length > 0 ? (
            <PlayerProfileChampionsCard champions={topChampions} />
          ) : null}
        </div>

      </div>

      <PlayerProfileRenewalModal
        show={showRenewalModal}
        playerName={player.full_name}
        t={t}
        weeklySuffix={weeklySuffix}
        renewalWage={renewalWage}
        renewalLength={renewalLength}
        renewalIsTerminal={renewalIsTerminal}
        isRenewalWageValid={isRenewalWageValid}
        renewalViolatesSoftCap={renewalViolatesSoftCap}
        renewalProjection={renewalProjection}
        renewalStatusMessage={renewalStatusMessage}
        renewalStatusClassName={renewalStatusClassName}
        renewalCooledOff={renewalCooledOff}
        renewalFeedback={renewalFeedback}
        renewalSubmitting={renewalSubmitting}
        renewalSubmitDisabled={renewalSubmitDisabled}
        onWageChange={setRenewalWage}
        onLengthChange={setRenewalLength}
        onClose={closeRenewalModal}
        onDelegate={() => void handleDelegateRenewal()}
        onSubmit={() => void handleRenewalSubmit()}
      />

      {showReleaseContractModal ? (
        <DashboardModalFrame maxWidthClassName="max-w-md">
          <div className="space-y-4">
            <h3 className="text-lg font-heading font-bold uppercase tracking-wider text-gray-900 dark:text-gray-100">
              {t("playerProfile.releaseContract")}
            </h3>
            <p className="text-sm text-gray-600 dark:text-gray-300">
              {t("playerProfile.releaseContractConfirm")}
            </p>
            <div className="rounded-lg border border-gray-200 dark:border-navy-600 bg-gray-50 dark:bg-navy-700/40 p-3">
              <p className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400">
                {t("playerProfile.releasePenalty", { defaultValue: "Termination cost" })}
              </p>
              <p className="text-sm font-semibold text-red-500 mt-1">
                {formatVal(releasePenaltyPreview)}
              </p>
            </div>
            <div className="flex gap-2 justify-end">
              <Button
                variant="ghost"
                onClick={() => setShowReleaseContractModal(false)}
                disabled={transferActionSubmitting}
              >
                {t("common.cancel")}
              </Button>
              <Button
                variant="primary"
                className="bg-red-600 hover:bg-red-700 active:bg-red-800"
                onClick={() => void handleConfirmReleaseContract()}
                disabled={transferActionSubmitting}
              >
                {t("playerProfile.releaseContract")}
              </Button>
            </div>
          </div>
        </DashboardModalFrame>
      ) : null}

      {showTransferOfferModal ? (
        <DashboardModalFrame maxWidthClassName="max-w-md">
          <div className="space-y-4">
            <h3 className="text-lg font-heading font-bold uppercase tracking-wider text-gray-900 dark:text-gray-100">
              {t("playerProfile.makeTransferOffer")}
            </h3>
            <p className="text-sm text-gray-500 dark:text-gray-400">{player.full_name}</p>
            <div>
              <label
                htmlFor="transfer-offer-amount"
                className="text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 block mb-1"
              >
                {t("playerProfile.transferOfferAmount", {
                  defaultValue: "Offer amount",
                })}
              </label>
              <input
                id="transfer-offer-amount"
                type="number"
                min="1"
                step="1000"
                value={transferOfferAmount}
                onChange={(event) => setTransferOfferAmount(event.target.value)}
                className="w-full px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600 text-sm text-gray-800 dark:text-gray-200 focus:outline-none focus:ring-2 focus:ring-primary-500/50"
              />
            </div>
            {transferOfferError ? (
              <p className="rounded-md border border-red-400/30 bg-red-500/10 px-3 py-2 text-xs text-red-200">
                {transferOfferError}
              </p>
            ) : null}
            {transferOfferFeedback ? (
              <div className="rounded-md border border-primary-400/30 bg-primary-500/10 px-3 py-2 text-xs text-primary-100">
                <p className="font-heading font-bold uppercase tracking-wider text-[10px] text-primary-200/90">
                  {resolveBackendText(
                    transferOfferFeedback.feedback.headline_key,
                    transferOfferFeedback.feedback.headline_key,
                    transferOfferFeedback.feedback.params,
                  )}
                </p>
                {transferOfferFeedback.feedback.detail_key ? (
                  <p className="mt-1 text-primary-100/90">
                    {resolveBackendText(
                      transferOfferFeedback.feedback.detail_key,
                      transferOfferFeedback.feedback.detail_key,
                      transferOfferFeedback.feedback.params,
                    )}
                  </p>
                ) : null}
              </div>
            ) : null}
            <div className="flex gap-2 justify-end">
              <Button
                variant="ghost"
                onClick={() => setShowTransferOfferModal(false)}
                disabled={transferActionSubmitting}
              >
                {t("common.cancel")}
              </Button>
              <Button
                onClick={() => void handleSubmitTransferOffer()}
                disabled={transferActionSubmitting}
              >
                {t("playerProfile.transferOfferSubmit", {
                  defaultValue: "Send offer",
                })}
              </Button>
            </div>
          </div>
        </DashboardModalFrame>
      ) : null}
    </div>
  );
}
