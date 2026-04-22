import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { calcOvr, getContractRiskLevel } from "../../lib/helpers";
import { PlayerData, GameStateData, PlayerMatchHistoryEntryData } from "../../store/gameStore";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { resolveBackendText } from "../../utils/backendI18n";
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

type LolRole = "TOP" | "JUNGLE" | "MID" | "ADC" | "SUPPORT";

interface PlayerSeed {
  ign: string;
  role: string;
  champions: Array<Array<string | number>>;
}

const PLAYER_SEEDS: PlayerSeed[] =
  ((playersSeed as { data?: { rostered_seeds?: PlayerSeed[] } }).data?.rostered_seeds ?? []) as PlayerSeed[];

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
  return value.toLowerCase().replace(/[^a-z]/g, "");
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
  matchName: string,
  championPerformance: Map<string, { wr: number; games: number }>,
) {
  const seed = PLAYER_SEEDS.find((entry) => normalizeKey(entry.ign) === normalizeKey(matchName));
  const champions = [...(seed?.champions ?? [])]
    .map((entry) => ({
      championName: String(entry[0] ?? ""),
      mastery: Number(entry[1] ?? 0),
    }))
    .filter((entry) => entry.championName.length > 0)
    .sort((a, b) => b.mastery - a.mastery);

  if (champions.length === 0) return [];

  const insignia = champions[0];
  const rest = champions.slice(1, 4);

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
  isOwnClub,
  startWithRenewalModal = false,
  onClose,
  onSelectTeam,
  onGameUpdate,
}: PlayerProfileProps) {
  const { t, i18n } = useTranslation();
  const weeklySuffix = t("finances.perWeekSuffix", "/wk");
  const primaryPosition = player.natural_position || player.position;

  if (!player) {
    return null;
  }

  const [scoutStatus, setScoutStatus] = useState<PlayerProfileScoutStatus>(
    "idle",
  );
  const [playerHistory, setPlayerHistory] = useState<PlayerMatchHistoryEntryData[]>([]);
  const [rerollingRole, setRerollingRole] = useState(false);
  const [scoutError, setScoutError] = useState<string | null>(null);
  const [showRenewalModal, setShowRenewalModal] = useState(false);
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
  const ovr = calcOvr(player, primaryPosition);
  const age = getPlayerAge(player.date_of_birth, gameState.clock.current_date);
  const teamName = getPlayerTeamName(
    gameState.teams,
    player.team_id,
    {
      freeAgent: t("common.freeAgent"),
      unknown: t("common.unknown"),
    },
  );
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
  const attrGroups = buildPlayerAttributeGroups(player, t);
  const championPerformance = buildChampionPerformanceMap(playerHistory);
  const topChampions = buildTopChampionMasteries(player.match_name, championPerformance);

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
    if (!isOwnClub || !onGameUpdate || rerollingRole) {
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
      !isOwnClub ||
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
    isOwnClub,
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
        primaryPosition={primaryPosition}
        age={age}
        teamName={teamName}
        weeklySuffix={weeklySuffix}
        language={i18n.language}
        isOwnClub={isOwnClub || !onGameUpdate}
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
          isOwnClub={isOwnClub}
          onOpenRenewal={openRenewalModal}
          t={t}
        />

        <div className="lg:col-span-2 flex flex-col gap-5">
          <PlayerProfileAttributesCard
            attrGroups={attrGroups}
            isOwnClub={isOwnClub}
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
    </div>
  );
}
