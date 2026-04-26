import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { FixtureData, GameStateData } from "../../store/gameStore";
import { getFixtureDisplayLabel } from "../../lib/helpers";
import { MatchSnapshot } from "./types";
import PreMatchLineup, {
  getPositionOvr,
} from "./PreMatchLineup";
import MatchScreenLayout from "./MatchScreenLayout";
import { ChevronRight } from "lucide-react";
function normalizeKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]/g, "");
}

const TEAM_LOGO_BY_NAME: Record<string, string> = {
  g2esports: "/team-logos/g2-esports.png",
  fnatic: "/team-logos/fnatic.png",
  giantx: "/team-logos/giantx-lec.png",
  karminecorp: "/team-logos/karmine-corp.png",
  movistarkoi: "/team-logos/mad-lions.png",
  mkoi: "/team-logos/mad-lions.png",
  koi: "/team-logos/mad-lions.png",
  madlionskoi: "/team-logos/mad-lions.png",
  natusvincere: "/team-logos/natus-vincere.png",
  skgaming: "/team-logos/sk-gaming.png",
  teamheretics: "/team-logos/team-heretics-lec.png",
  teamvitality: "/team-logos/team-vitality.png",
  teambds: "/team-logos/team-bds.png",
  shifters: "/team-logos/team-bds.png",
};

function resolveTeamLogo(teamName: string): string | null {
  return TEAM_LOGO_BY_NAME[normalizeKey(teamName)] ?? null;
}

interface PreMatchSetupProps {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  currentFixture?: FixtureData | null;
  userSide: "Home" | "Away";
  onStart: () => void;
  onUpdateSnapshot: (snap: MatchSnapshot) => void;
}

export default function PreMatchSetup({
  snapshot,
  gameState,
  currentFixture,
  userSide,
  onStart,
  onUpdateSnapshot,
}: PreMatchSetupProps) {
  const { t } = useTranslation();
  const [selectedStarterId, setSelectedStarterId] = useState<string | null>(
    null,
  );
  const [isAutoSelecting, setIsAutoSelecting] = useState(false);

  const userTeam =
    userSide === "Home" ? snapshot.home_team : snapshot.away_team;
  const oppTeam = userSide === "Home" ? snapshot.away_team : snapshot.home_team;

  const homeTeamColor =
    gameState.teams.find((t) => t.id === snapshot.home_team.id)?.colors
      ?.primary || "#10b981";
  const awayTeamColor =
    gameState.teams.find((t) => t.id === snapshot.away_team.id)?.colors
      ?.primary || "#6366f1";
  const userColor = userSide === "Home" ? homeTeamColor : awayTeamColor;
  const fixtureLabel = currentFixture
    ? getFixtureDisplayLabel(t, currentFixture)
    : t("match.matchDay");
  const homeLogo = resolveTeamLogo(snapshot.home_team.name);
  const awayLogo = resolveTeamLogo(snapshot.away_team.name);

  // Use snapshot bench data (updated after swaps)
  const userBench =
    userSide === "Home" ? snapshot.home_bench || [] : snapshot.away_bench || [];

  console.info("[PreMatchSetup] render", {
    awayTeam: snapshot.away_team.name,
    benchCount: userBench.length,
    homeTeam: snapshot.home_team.name,
    phase: snapshot.phase,
    playStyle: userTeam.play_style,
    selectedStarterId,
    startingPlayerCount: userTeam.players.length,
    userSide,
    userTeam: userTeam.name,
  });

  const handleSwap = async (benchPlayerId: string) => {
    if (!selectedStarterId) return;
    try {
      const snap = await invoke<MatchSnapshot>("apply_match_command", {
        command: {
          PreMatchSwap: {
            side: userSide,
            player_off_id: selectedStarterId,
            player_on_id: benchPlayerId,
          },
        },
      });
      onUpdateSnapshot(snap);
    } catch (err) {
      console.error("Pre-match swap failed:", err);
    }
    setSelectedStarterId(null);
  };

  const handleAutoSelect = async () => {
    setIsAutoSelecting(true);
    try {
      const pool = [...userTeam.players, ...userBench];
      const ranked = [...pool].sort(
        (a, b) =>
          getPositionOvr(b) * (b.condition / 100) -
          getPositionOvr(a) * (a.condition / 100),
      );
      const idealIds = new Set(ranked.slice(0, 5).map((p) => p.id));

      const currentIds = new Set(userTeam.players.map((p) => p.id));
      const toAdd = [...idealIds].filter((id) => !currentIds.has(id));
      const toRemove = [...currentIds].filter((id) => !idealIds.has(id));

      let snap: MatchSnapshot | null = null;
      for (let i = 0; i < Math.min(toAdd.length, toRemove.length); i++) {
        snap = await invoke<MatchSnapshot>("apply_match_command", {
          command: {
            PreMatchSwap: {
              side: userSide,
              player_off_id: toRemove[i],
              player_on_id: toAdd[i],
            },
          },
        });
      }
      if (snap) onUpdateSnapshot(snap);
    } catch (err) {
      console.error("Auto-select failed:", err);
    } finally {
      setIsAutoSelecting(false);
      setSelectedStarterId(null);
    }
  };

  return (
    <MatchScreenLayout
      headerClassName="bg-linear-to-r from-gray-200 via-white to-gray-200 dark:from-navy-800 dark:via-navy-900 dark:to-navy-800"
      headerContentClassName="max-w-5xl py-6"
      contentClassName="overflow-auto"
      header={
        <>
          <div className="flex items-center justify-between mb-6">
            <div className="flex items-center gap-4">
              <div
                className="w-14 h-14 rounded-xl flex items-center justify-center font-heading font-bold text-lg"
                style={{
                  backgroundColor: homeTeamColor + "30",
                  borderColor: homeTeamColor,
                  borderWidth: 2,
                }}
              >
                {homeLogo ? (
                  <img
                    src={homeLogo}
                    alt={snapshot.home_team.name}
                    className="w-10 h-10 object-contain"
                    loading="lazy"
                  />
                ) : (
                  snapshot.home_team.name.substring(0, 3).toUpperCase()
                )}
              </div>
              <div>
                <p className="font-heading font-bold text-lg text-gray-900 dark:text-white">
                  {snapshot.home_team.name}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {t("match.home")} · {t("match.lineup")} {snapshot.home_team.players.length}/5
                </p>
              </div>
            </div>

            <div className="text-center">
              <p className="text-xs font-heading uppercase tracking-widest text-accent-700 dark:text-accent-400 mb-1">
                {fixtureLabel}
              </p>
              <p className="text-3xl font-heading font-bold text-gray-500 dark:text-gray-400">
                VS
              </p>
            </div>

            <div className="flex items-center gap-4">
              <div className="text-right">
                <p className="font-heading font-bold text-lg text-gray-900 dark:text-white">
                  {snapshot.away_team.name}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {t("match.away")} · {t("match.lineup")} {snapshot.away_team.players.length}/5
                </p>
              </div>
              <div
                className="w-14 h-14 rounded-xl flex items-center justify-center font-heading font-bold text-lg"
                style={{
                  backgroundColor: awayTeamColor + "30",
                  borderColor: awayTeamColor,
                  borderWidth: 2,
                }}
              >
                {awayLogo ? (
                  <img
                    src={awayLogo}
                    alt={snapshot.away_team.name}
                    className="w-10 h-10 object-contain"
                    loading="lazy"
                  />
                ) : (
                  snapshot.away_team.name.substring(0, 3).toUpperCase()
                )}
              </div>
            </div>
          </div>

          <div className="flex justify-center mt-2">
            <button
              onClick={onStart}
              className="flex items-center gap-3 px-10 py-3.5 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 rounded-xl font-heading font-bold uppercase tracking-wider text-sm text-white shadow-lg shadow-primary-500/20 transition-all hover:scale-[1.02] active:scale-[0.98]"
            >
              {t("match.startMatch")}
              <ChevronRight className="w-5 h-5" />
            </button>
          </div>
        </>
      }
    >
      <div className="max-w-5xl mx-auto px-6 py-6 flex flex-col gap-6">
        <PreMatchLineup
          userTeam={userTeam}
          userBench={userBench}
          oppTeam={oppTeam}
          userColor={userColor}
          homeTeamColor={homeTeamColor}
          awayTeamColor={awayTeamColor}
          userSide={userSide}
          selectedStarterId={selectedStarterId}
          isAutoSelecting={isAutoSelecting}
          onSelectStarter={setSelectedStarterId}
          onSwap={handleSwap}
          onAutoSelect={handleAutoSelect}
        />
      </div>
    </MatchScreenLayout>
  );
}
