import type { GameStateData } from "../../store/gameStore";
import { normalizeTrainingFocus } from "../../lib/trainingFocus";
import { Card, CardHeader, CardBody } from "../ui";
import { formatDateShort } from "../../lib/helpers";
import { resolveSeasonContext } from "../../lib/seasonContext";
import NextMatchDisplay from "../NextMatchDisplay";
import {
  resolveMessage,
} from "../../utils/backendI18n";
import {
  getHomeRosterOverview,
  getOnboardingCompletionState,
  getRecentResultsForTeam,
} from "./HomeTab.helpers";
import HomeLeaguePositionCard from "./HomeLeaguePositionCard";
import HomeLatestNewsCard from "./HomeLatestNewsCard";
import HomeRosterLineupCard from "./HomeRosterLineupCard";
import HomePlayerMomentumCard from "./HomePlayerMomentumCard";
import HomeRecentResultsCard from "./HomeRecentResultsCard";
import HomeRecentMessagesCard from "./HomeRecentMessagesCard";
import HomeSquadOverviewCard from "./HomeSquadOverviewCard";
import HomeSeasonStatusCard from "./HomeSeasonStatusCard";
import HomeUnavailablePlayersCard from "./HomeUnavailablePlayersCard";
import {
  Dumbbell,
  Mail,
  Flame,
  Scale,
  Feather,
  Users,
  Crosshair,
  UserCog,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import HomeOnboardingChecklistCard from "./HomeOnboardingChecklistCard";
import JobOpportunitiesCard from "./JobOpportunitiesCard";
import HomeThisWeekCard from "./HomeThisWeekCard";
import HomeFinancesCard from "./HomeFinancesCard";

interface HomeTabProps {
  gameState: GameStateData;
  onNavigate?: (tab: string, context?: { messageId?: string }) => void;
  onGameUpdate?: (state: GameStateData) => void;
  visitedOnboardingTabs: ReadonlySet<string>;
}

const SCHEDULE_ICONS: Record<string, { icon: React.ReactNode; color: string }> =
{
  Intense: { icon: <Flame className="w-3.5 h-3.5" />, color: "text-red-500" },
  Balanced: {
    icon: <Scale className="w-3.5 h-3.5" />,
    color: "text-primary-500",
  },
  Light: {
    icon: <Feather className="w-3.5 h-3.5" />,
    color: "text-blue-500",
  },
};

export default function HomeTab({
  gameState,
  onNavigate,
  onGameUpdate,
  visitedOnboardingTabs,
}: HomeTabProps) {
  const { t, i18n } = useTranslation();
  const myTeam = gameState.teams.find(
    (tm) => tm.id === gameState.manager.team_id,
  );
  const league = gameState.league;
  const roster = myTeam
    ? gameState.players.filter((p) => p.team_id === myTeam.id)
    : [];
  const {
    avgCondition,
    avgOvr,
    coldPlayers,
    exhaustedCount,
    hotPlayers,
    unavailablePlayers,
  } = getHomeRosterOverview(roster);
  const resolveInjuryName = (injuryName: string): string => {
    if (injuryName.includes(".")) {
      return t(injuryName, { defaultValue: injuryName });
    }

    return t(`common.injuries.${injuryName}`, { defaultValue: injuryName });
  };

  // Current date / season context
  const lang = i18n.language;
  const seasonContext = resolveSeasonContext(gameState);
  const isPreseason = seasonContext.phase === "Preseason";
  const seasonStartLabel = seasonContext.season_start
    ? formatDateShort(seasonContext.season_start, lang)
    : null;
  const transferWindow = seasonContext.transfer_window;
  const transferWindowVariant =
    transferWindow.status === "DeadlineDay"
      ? "danger"
      : transferWindow.status === "Open"
        ? "success"
        : "neutral";
  const transferWindowSummary =
    transferWindow.status === "DeadlineDay"
      ? t("season.windowClosesToday")
      : transferWindow.status === "Open" &&
        transferWindow.days_remaining !== null
        ? t("season.windowClosesInDays", {
          count: transferWindow.days_remaining,
        })
        : transferWindow.status === "Closed" &&
          transferWindow.days_until_opens !== null
          ? t("season.windowOpensInDays", {
            count: transferWindow.days_until_opens,
          })
          : t("season.windowClosed");

  const sortedStandings = league
    ? [...league.standings].sort(
      (a, b) =>
        b.points - a.points ||
        b.goals_for - b.goals_against - (a.goals_for - a.goals_against),
    )
    : [];

  const recentResults = getRecentResultsForTeam(gameState, myTeam?.id ?? null);

  // Training schedule
  const schedule = myTeam?.training_schedule || "Balanced";
  const schedIcons = SCHEDULE_ICONS[schedule] || SCHEDULE_ICONS.Balanced;
  const schedLabel = t(`common.trainingSchedules.${schedule}`, schedule);
  const focus = normalizeTrainingFocus(myTeam?.training_focus);

  // Latest news
  const latestNews = (gameState.news || [])
    .sort((a, b) => b.date.localeCompare(a.date))
    .slice(0, 2);
  const recentMessages = (gameState.messages || [])
    .slice(0, 4)
    .map(resolveMessage);
  const onboardingState = getOnboardingCompletionState(
    gameState,
    visitedOnboardingTabs,
  );

  const onboardingSteps = [
    {
      id: "squad",
      done: onboardingState.hasVisitedSquadPage,
      label: t("onboarding.reviewSquad"),
      description: t("onboarding.reviewSquadDesc"),
      tab: "Squad",
      icon: <Users className="w-4 h-4" />,
    },
    {
      id: "staff",
      done: onboardingState.hasVisitedStaffPage,
      label: t("onboarding.hireStaff"),
      description: t("onboarding.hireStaffDesc"),
      tab: "Staff",
      icon: <UserCog className="w-4 h-4" />,
    },
    {
      id: "tactics",
      done: onboardingState.hasVisitedTacticsPage,
      label: t("onboarding.setTactics"),
      description: t("onboarding.setTacticsDesc"),
      tab: "Tactics",
      icon: <Crosshair className="w-4 h-4" />,
    },
    {
      id: "training",
      done: onboardingState.hasVisitedTrainingPage,
      label: t("onboarding.configTraining"),
      description: t("onboarding.configTrainingDesc"),
      tab: "Training",
      icon: <Dumbbell className="w-4 h-4" />,
    },
    {
      id: "inbox",
      done: onboardingState.hasReadInbox,
      label: t("onboarding.readMessages"),
      description: t("onboarding.readMessagesDesc"),
      tab: "Inbox",
      icon: <Mail className="w-4 h-4" />,
    },
  ];
  const completedSteps = onboardingState.completedSteps;

  return (
    <div className="max-w-6xl mx-auto flex flex-col gap-5">
      {myTeam && isPreseason && (
        <HomeSeasonStatusCard
          phase={seasonContext.phase}
          seasonStartLabel={seasonStartLabel}
          daysUntilSeasonStart={seasonContext.days_until_season_start}
          transferWindowStatus={transferWindow.status}
          transferWindowVariant={transferWindowVariant}
          transferWindowSummary={transferWindowSummary}
          transferWindowOpensOn={transferWindow.opens_on}
          transferWindowClosesOn={transferWindow.closes_on}
          lang={lang}
        />
      )}

      {/* Onboarding — Getting Started Checklist */}
      {myTeam && onboardingState.showOnboarding &&
        completedSteps < onboardingSteps.length && (
          <HomeOnboardingChecklistCard
            completedSteps={completedSteps}
            totalSteps={onboardingSteps.length}
            steps={onboardingSteps}
            onNavigate={onNavigate}
          />
        )}

      {myTeam ? (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
          {/* Next Match Card */}
          <Card accent="primary" className="md:col-span-2">
            <CardHeader>{t("home.nextMatch")}</CardHeader>
            <CardBody>
              <NextMatchDisplay gameState={gameState} />
            </CardBody>
          </Card>

          {/* League Position */}
          <HomeLeaguePositionCard
            isPreseason={isPreseason}
            phase={seasonContext.phase}
            seasonStartLabel={seasonStartLabel}
            sortedStandings={sortedStandings}
            teams={gameState.teams}
            myTeamId={myTeam.id}
            onNavigate={onNavigate}
          />
        </div>
      ) : (
        <>
          <HomeLeaguePositionCard
            isPreseason={isPreseason}
            phase={seasonContext.phase}
            seasonStartLabel={seasonStartLabel}
            sortedStandings={sortedStandings}
            teams={gameState.teams}
            myTeamId={null}
            onNavigate={onNavigate}
          />
          {onGameUpdate && (
            <JobOpportunitiesCard
              gameState={gameState}
              onGameUpdate={onGameUpdate}
            />
          )}
        </>
      )}

      {myTeam && (
        <>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
            <HomeThisWeekCard gameState={gameState} />
            <HomeFinancesCard team={myTeam} onNavigate={onNavigate} />
          </div>

          <HomeUnavailablePlayersCard
            players={unavailablePlayers}
            resolveInjuryName={resolveInjuryName}
            onNavigate={onNavigate}
          />

          <HomeRosterLineupCard roster={roster} onNavigate={onNavigate} />

          <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
            {/* Squad Fitness */}
            <HomeSquadOverviewCard
              avgCondition={avgCondition}
              avgOvr={avgOvr}
              exhaustedCount={exhaustedCount}
              scheduleIcon={schedIcons.icon}
              scheduleColorClass={schedIcons.color}
              scheduleLabel={schedLabel}
              focus={focus}
              onNavigate={onNavigate}
            />

            <HomeRecentResultsCard
              recentResults={recentResults}
              teams={gameState.teams}
              onNavigate={onNavigate}
            />

            <HomeLatestNewsCard
              articles={latestNews}
              teams={gameState.teams}
              lang={lang}
              onNavigate={onNavigate}
            />
          </div>

          {roster.length > 0 && (hotPlayers.length > 0 || coldPlayers.length > 0) && (
            <HomePlayerMomentumCard
              hotPlayers={hotPlayers}
              coldPlayers={coldPlayers}
              onNavigate={onNavigate}
            />
          )}
        </>
      )}

      <HomeRecentMessagesCard
        messages={recentMessages}
        lang={lang}
        onNavigate={onNavigate}
      />
    </div>
  );
}
