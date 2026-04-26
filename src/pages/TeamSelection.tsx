import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useGameStore, GameStateData, PlayerData } from "../store/gameStore";
import { Card, CardBody, Badge, TeamLocation, ThemeToggle } from "../components/ui";
import { ArrowLeft, Users, Trophy, Landmark, ChevronRight, Star, Loader2 } from "lucide-react";

type TeamSelectionData = {
  manager: GameStateData["manager"];
  teams: GameStateData["teams"];
  players: GameStateData["players"];
};

function clampOvr(value: number): number {
  return Math.max(1, Math.min(99, Math.round(value)));
}

function avg(...values: number[]): number {
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function lolRoleFromPlayer(player: PlayerData): "top" | "jungle" | "mid" | "bottom" | "support" | "unknown" {
  const position = (player.natural_position || player.position || "").toLowerCase();

  if (position.includes("defender") && !position.includes("midfielder")) return "top";
  if (position === "midfielder") return "jungle";
  if (position.includes("attackingmidfielder")) return "mid";
  if (position.includes("forward")) return "bottom";
  if (position.includes("defensivemidfielder") || position.includes("goalkeeper")) return "support";
  return "unknown";
}

function roleBonus(player: PlayerData): number {
  const a = player.attributes;
  switch (lolRoleFromPlayer(player)) {
    case "top":
      return avg(a.strength, a.defending, a.positioning) * 0.08;
    case "jungle":
      return avg(a.vision, a.decisions, a.passing) * 0.08;
    case "mid":
      return avg(a.dribbling, a.shooting, a.decisions) * 0.08;
    case "bottom":
      return avg(a.shooting, a.pace, a.positioning) * 0.08;
    case "support":
      return avg(a.passing, a.vision, a.teamwork) * 0.08;
    default:
      return 0;
  }
}

function lolPlayerOvr(player: PlayerData): number {
  const a = player.attributes;

  const mechanics = avg(a.dribbling, a.agility, a.pace, a.composure);
  const macro = avg(a.vision, a.decisions, a.positioning, a.passing);
  const teamfight = avg(a.teamwork, a.stamina, a.composure, a.strength);
  const consistency = avg(a.decisions, a.vision, a.positioning, a.composure);

  const weighted =
    mechanics * 0.34 +
    macro * 0.28 +
    teamfight * 0.22 +
    consistency * 0.16;

  return clampOvr(weighted + roleBonus(player));
}

function getTeamLogoPath(teamId: string): string {
  const slug = teamId.replace(/^lec-/, "");
  if (slug === "shifters") {
    return "https://static.lolesports.com/teams/1765897071435_600px-Shifters_allmode.png";
  }
  return `/team-logos/${slug}.png`;
}

function isAcademyPlayer(playerId: string): boolean {
  return playerId.includes("-academy-");
}

export default function TeamSelection() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { gameState, setGameState, setGameActive } = useGameStore();
  const [selectedTeamId, setSelectedTeamId] = useState<string | null>(null);
  const [isConfirming, setIsConfirming] = useState(false);
  const [isRecoveringGame, setIsRecoveringGame] = useState(false);
  const [teamSelectionData, setTeamSelectionData] = useState<TeamSelectionData | null>(null);

  useEffect(() => {
    if (gameState || teamSelectionData) return;

    let cancelled = false;
    setIsRecoveringGame(true);
    console.debug("[TeamSelection] recovering data via get_team_selection_data");

    void invoke<TeamSelectionData>("get_team_selection_data")
      .then((data) => {
        if (cancelled) return;
        console.debug("[TeamSelection] data recovered", {
          teams: data.teams.length,
          players: data.players.length,
        });
        setTeamSelectionData(data);
      })
      .catch((error) => {
        console.error("Failed to recover active game for team selection:", error);
        if (!cancelled) {
          alert(`No se pudo cargar la selección de equipo: ${String(error)}`);
          navigate("/");
        }
      })
      .finally(() => {
        if (!cancelled) setIsRecoveringGame(false);
      });

    return () => {
      cancelled = true;
    };
  }, [gameState, navigate, setGameState, teamSelectionData]);

  const viewState = gameState ?? teamSelectionData;

  if (!viewState) {
    return (
      <div className="min-h-screen bg-gray-100 dark:bg-navy-900 flex items-center justify-center">
        <div className="text-center text-gray-600 dark:text-gray-300">
          <Loader2 className="w-8 h-8 animate-spin mx-auto mb-3" />
          <p className="text-sm">
            {isRecoveringGame ? t("worldSelect.creatingWorld") : "Cargando partida..."}
          </p>
        </div>
      </div>
    );
  }

  const teams = viewState.teams;

  const getTeamPlayers = (teamId: string): PlayerData[] =>
    viewState.players.filter((p) => p.team_id === teamId);

  const getCompetitiveRoster = (teamId: string): PlayerData[] => {
    const players = getTeamPlayers(teamId);
    const mainRoster = players.filter((player) => !isAcademyPlayer(player.id));
    const source = mainRoster.length > 0 ? mainRoster : players;
    return source.slice(0, 5);
  };

  const getTeamAvgOvr = (teamId: string): number => {
    const players = getCompetitiveRoster(teamId);
    if (players.length === 0) return 0;
    const total = players.reduce((sum, player) => sum + lolPlayerOvr(player), 0);
    return clampOvr(total / players.length);
  };

  const getReputationLabel = (rep: number): { label: string; variant: "primary" | "accent" | "success" | "danger" | "neutral" } => {
    if (rep >= 750) return { label: t('teamSelect.repWorldClass'), variant: "accent" };
    if (rep >= 600) return { label: t('teamSelect.repStrong'), variant: "success" };
    if (rep >= 400) return { label: t('teamSelect.repAverage'), variant: "neutral" };
    return { label: t('teamSelect.repDeveloping'), variant: "danger" };
  };

  const formatFinance = (val: number): string => {
    if (val >= 1_000_000) return `€${(val / 1_000_000).toFixed(1)}M`;
    if (val >= 1_000) return `€${(val / 1_000).toFixed(0)}K`;
    return `€${val}`;
  };

  const handleConfirm = async () => {
    if (!selectedTeamId || isConfirming) return;
    setIsConfirming(true);
    try {
      const updatedGame = await invoke<GameStateData>("select_team", { teamId: selectedTeamId });
      setGameState(updatedGame);
      const mgr = updatedGame.manager;
      const displayName = mgr.nickname?.trim() || `${mgr.first_name} ${mgr.last_name}`;
      setGameActive(true, displayName);
      navigate("/dashboard");
    } catch (error) {
      console.error("Failed to select team:", error);
      alert("Failed to select team: " + String(error));
    } finally {
      setIsConfirming(false);
    }
  };

  const selectedTeam = teams.find((t) => t.id === selectedTeamId);

  return (
    <div className="min-h-screen bg-gray-100 dark:bg-navy-900 transition-colors duration-300">
      {/* Header */}
      <header className="bg-white dark:bg-navy-800 border-b border-gray-200 dark:border-navy-700 px-6 py-4 flex justify-between items-center shadow-sm">
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate("/")}
            className="p-2 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-navy-700 transition-colors"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
          <div>
            <h1 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-800 dark:text-gray-100">
              {t('teamSelect.title')}
            </h1>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
              {t('teamSelect.subtitle')}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <ThemeToggle />
          {selectedTeam && (
            <button
              onClick={handleConfirm}
              disabled={isConfirming}
              className={`bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white px-6 py-2.5 rounded-lg font-heading font-bold uppercase tracking-wider text-sm shadow-md hover:shadow-lg hover:shadow-primary-500/20 transition-all flex items-center gap-2 ${isConfirming ? "opacity-70 cursor-wait" : ""}`}
            >
              <span>{isConfirming ? t('teamSelect.confirming') : t('teamSelect.manage', { name: selectedTeam.short_name })}</span>
              {isConfirming ? <Loader2 className="w-4 h-4 animate-spin" /> : <ChevronRight className="w-4 h-4" />}
            </button>
          )}
        </div>
      </header>

      <div className="max-w-7xl mx-auto p-6">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {teams.map((team) => {
            const isSelected = selectedTeamId === team.id;
            const avgOvr = getTeamAvgOvr(team.id);
            const repInfo = getReputationLabel(team.reputation);
            const playerCount = getCompetitiveRoster(team.id).length;

            return (
              <button
                key={team.id}
                onClick={() => setSelectedTeamId(team.id)}
                className={`text-left transition-all duration-200 rounded-xl ${
                  isSelected
                    ? "ring-2 ring-primary-500 ring-offset-2 dark:ring-offset-navy-900 scale-[1.02]"
                    : "hover:scale-[1.01]"
                }`}
              >
                <Card
                  accent={isSelected ? "primary" : "none"}
                  className="h-full"
                >
                  {/* Team header with gradient */}
                  <div className={`p-4 rounded-t-xl ${
                    isSelected
                      ? "bg-gradient-to-r from-primary-600 to-primary-700"
                      : "bg-gradient-to-r from-navy-700 to-navy-800"
                  }`}>
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <div
                          className={`w-12 h-12 rounded-lg flex items-center justify-center overflow-hidden ${
                            isSelected ? "bg-white/20" : "bg-white/10"
                          }`}
                        >
                          <img
                            src={getTeamLogoPath(team.id)}
                            alt={`${team.name} logo`}
                            className="w-10 h-10 object-contain"
                            loading="lazy"
                          />
                        </div>
                        <div>
                          <h3 className="font-heading font-bold text-white uppercase tracking-wide text-sm">
                            {team.name}
                          </h3>
                          <TeamLocation
                            city={team.city}
                            countryCode={team.country}
                            locale={i18n.language}
                            className="mt-0.5 text-xs text-gray-300"
                            iconClassName="w-3 h-3"
                            flagClassName="text-xs leading-none"
                          />
                        </div>
                      </div>
                      {isSelected && (
                        <Star className="w-5 h-5 text-accent-400 fill-current" />
                      )}
                    </div>
                  </div>

                  <CardBody className="p-4">
                    <div className="grid grid-cols-2 gap-3">
                      <StatItem
                        icon={<Trophy className="w-3.5 h-3.5" />}
                        label={t('teamSelect.reputation')}
                        value={<Badge variant={repInfo.variant} size="sm">{repInfo.label}</Badge>}
                      />
                      <StatItem
                        icon={<Users className="w-3.5 h-3.5" />}
                        label={t('teamSelect.squad')}
                        value={<span className="font-heading font-bold text-gray-800 dark:text-gray-200">{playerCount}</span>}
                      />
                      <StatItem
                        icon={<Landmark className="w-3.5 h-3.5" />}
                        label={t('teamSelect.finances')}
                        value={<span className="font-heading font-bold text-gray-800 dark:text-gray-200">{formatFinance(team.finance)}</span>}
                      />
                      <StatItem
                        icon={<Star className="w-3.5 h-3.5" />}
                        label={t('teamSelect.avgOvr')}
                        value={
                          <span className={`font-heading font-bold text-lg ${
                            avgOvr >= 70 ? "text-primary-500" :
                            avgOvr >= 55 ? "text-accent-600 dark:text-accent-400" :
                            "text-gray-500"
                          }`}>{avgOvr}</span>
                        }
                      />
                    </div>

                  </CardBody>
                </Card>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function StatItem({ icon, label, value }: { icon: React.ReactNode; label: string; value: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs text-gray-400 dark:text-gray-500 flex items-center gap-1">
        {icon} {label}
      </span>
      {value}
    </div>
  );
}
