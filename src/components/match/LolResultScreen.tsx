import { useTranslation } from "react-i18next";
import type { FixtureData, GameStateData } from "../../store/gameStore";
import type { MatchEvent, MatchSnapshot } from "./types";

interface LolResultScreenProps {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  currentFixture?: FixtureData | null;
  userSide: "Home" | "Away" | null;
  importantEvents: MatchEvent[];
  onPressConference: () => void;
  onFinish: () => void;
}

function count(events: MatchEvent[], type: string, side: "Home" | "Away") {
  return events.filter((event) => event.event_type === type && event.side === side).length;
}

export default function LolResultScreen({
  snapshot,
  userSide,
  importantEvents,
  onPressConference,
  onFinish,
}: LolResultScreenProps) {
  const { t } = useTranslation();

  const homeKills = count(importantEvents, "Kill", "Home");
  const awayKills = count(importantEvents, "Kill", "Away");
  const homeKillsFromUnits = snapshot.lol_map?.units
    .filter((unit) => unit.side === "Home")
    .reduce((acc, unit) => acc + unit.kills, 0) ?? 0;
  const awayKillsFromUnits = snapshot.lol_map?.units
    .filter((unit) => unit.side === "Away")
    .reduce((acc, unit) => acc + unit.kills, 0) ?? 0;
  const displayHomeKills = Math.max(homeKills, homeKillsFromUnits);
  const displayAwayKills = Math.max(awayKills, awayKillsFromUnits);

  const homeObjectives = count(importantEvents, "ObjectiveTaken", "Home");
  const awayObjectives = count(importantEvents, "ObjectiveTaken", "Away");

  const homeStructures =
    count(importantEvents, "TowerDestroyed", "Home") +
    count(importantEvents, "InhibitorDestroyed", "Home") +
    count(importantEvents, "NexusTowerDestroyed", "Home") +
    count(importantEvents, "NexusDestroyed", "Home");
  const awayStructures =
    count(importantEvents, "TowerDestroyed", "Away") +
    count(importantEvents, "InhibitorDestroyed", "Away") +
    count(importantEvents, "NexusTowerDestroyed", "Away") +
    count(importantEvents, "NexusDestroyed", "Away");

  const winnerSide = snapshot.lol_map?.destroyed_nexus_by ?? (displayHomeKills >= displayAwayKills ? "Home" : "Away");
  const userWon = userSide ? winnerSide === userSide : false;

  return (
    <div className="min-h-screen bg-[#050608] text-white p-4 md:p-6">
      <div className="max-w-5xl mx-auto space-y-4">
        <header className="rounded-xl border border-cyan-400/25 bg-[#0a1433] p-5 text-center">
          <p className="text-xs uppercase tracking-[0.3em] text-gray-400">{t("match.matchOver", "Match Over")}</p>
          <h1 className={`mt-1 text-4xl font-heading uppercase ${userWon ? "text-emerald-400" : "text-rose-400"}`}>
            {userWon ? t("match.victory", "Victory") : t("match.defeat", "Defeat")}
          </h1>
            <p className="mt-3 text-3xl font-black">
              {snapshot.home_team.name} <span className="text-cyan-300">{displayHomeKills}</span> - <span className="text-orange-300">{displayAwayKills}</span> {snapshot.away_team.name}
            </p>
        </header>

        <section className="grid grid-cols-1 md:grid-cols-3 gap-3">
          <div className="rounded-xl border border-white/10 bg-[#0a1433] p-4">
            <p className="text-[11px] uppercase tracking-[0.2em] text-gray-400">Neutral Objectives</p>
            <p className="mt-2 text-lg font-heading">{homeObjectives} - {awayObjectives}</p>
          </div>
          <div className="rounded-xl border border-white/10 bg-[#0a1433] p-4">
            <p className="text-[11px] uppercase tracking-[0.2em] text-gray-400">Structures</p>
            <p className="mt-2 text-lg font-heading">{homeStructures} - {awayStructures}</p>
          </div>
          <div className="rounded-xl border border-white/10 bg-[#0a1433] p-4">
            <p className="text-[11px] uppercase tracking-[0.2em] text-gray-400">Duration</p>
            <p className="mt-2 text-lg font-heading">{snapshot.current_minute}m</p>
          </div>
        </section>

        <section className="rounded-xl border border-white/10 bg-[#0a1433] p-4">
          <p className="text-[11px] uppercase tracking-[0.2em] text-gray-400 mb-2">Key Timeline</p>
          <div className="space-y-1 max-h-64 overflow-auto pr-1">
            {importantEvents.slice(-20).reverse().map((evt, idx) => (
              <div key={`${evt.minute}-${evt.event_type}-${idx}`} className="text-sm text-gray-200 flex items-center justify-between gap-3">
                <span>{evt.minute}'</span>
                <span className="flex-1">{evt.event_type.replace(/([A-Z])/g, " $1").trim()}</span>
                <span className={evt.side === "Home" ? "text-cyan-300" : "text-orange-300"}>{evt.side}</span>
              </div>
            ))}
          </div>
        </section>

        <footer className="flex items-center justify-end gap-2">
          <button
            onClick={onPressConference}
            className="px-4 py-2 rounded-lg border border-white/20 bg-white/5 hover:bg-white/10 text-xs uppercase tracking-wider"
          >
            {t("match.pressConference", { defaultValue: "Press Conference" })}
          </button>
          <button
            onClick={onFinish}
            className="px-4 py-2 rounded-lg bg-cyan-500 hover:bg-cyan-400 text-navy-900 font-heading text-xs uppercase tracking-wider"
          >
            {t("match.continue", { defaultValue: "Continue" })}
          </button>
        </footer>
      </div>
    </div>
  );
}
