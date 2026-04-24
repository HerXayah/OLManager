import { useTranslation } from "react-i18next";
import type { FixtureData, GameStateData } from "../../store/gameStore";
import type { MatchEvent, MatchSnapshot } from "./types";
import type { LolSimV1RuntimeState } from "./lol-prototype/backend/contract-v1";

interface LolResultScreenProps {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  currentFixture?: FixtureData | null;
  userSide: "Home" | "Away" | null;
  importantEvents: MatchEvent[];
  finalRuntimeState?: LolSimV1RuntimeState | null;
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
  finalRuntimeState,
  onPressConference,
  onFinish,
}: LolResultScreenProps) {
  const { t } = useTranslation();

  const runtime = finalRuntimeState ?? null;

  const homeKills = runtime ? runtime.stats.blue.kills : count(importantEvents, "Kill", "Home");
  const awayKills = runtime ? runtime.stats.red.kills : count(importantEvents, "Kill", "Away");
  const homeKillsFromUnits = snapshot.lol_map?.units
    .filter((unit) => unit.side === "Home")
    .reduce((acc, unit) => acc + unit.kills, 0) ?? 0;
  const awayKillsFromUnits = snapshot.lol_map?.units
    .filter((unit) => unit.side === "Away")
    .reduce((acc, unit) => acc + unit.kills, 0) ?? 0;
  const displayHomeKills = Math.max(homeKills, homeKillsFromUnits);
  const displayAwayKills = Math.max(awayKills, awayKillsFromUnits);

  const homeObjectives = runtime
    ? runtime.stats.blue.dragons + runtime.stats.blue.barons
    : count(importantEvents, "ObjectiveTaken", "Home");
  const awayObjectives = runtime
    ? runtime.stats.red.dragons + runtime.stats.red.barons
    : count(importantEvents, "ObjectiveTaken", "Away");

  const homeStructures = runtime ? runtime.stats.blue.towers :
    count(importantEvents, "TowerDestroyed", "Home") +
      count(importantEvents, "InhibitorDestroyed", "Home") +
      count(importantEvents, "NexusTowerDestroyed", "Home") +
      count(importantEvents, "NexusDestroyed", "Home");
  const awayStructures = runtime ? runtime.stats.red.towers :
    count(importantEvents, "TowerDestroyed", "Away") +
      count(importantEvents, "InhibitorDestroyed", "Away") +
      count(importantEvents, "NexusTowerDestroyed", "Away") +
      count(importantEvents, "NexusDestroyed", "Away");

  const winnerSide = runtime?.winner
    ? runtime.winner === "blue" ? "Home" : "Away"
    : snapshot.lol_map?.destroyed_nexus_by ?? (displayHomeKills >= displayAwayKills ? "Home" : "Away");
  const userWon = userSide ? winnerSide === userSide : false;

  const durationMin = runtime ? Math.floor((runtime.timeSec ?? 0) / 60) : snapshot.current_minute;
  const homeChampions = runtime?.champions?.filter((champion) => champion.team === "blue") ?? [];
  const awayChampions = runtime?.champions?.filter((champion) => champion.team === "red") ?? [];
  const dragonObjective = runtime?.objectives?.dragon;
  const dragonSummary = runtime
    ? `Dragon ${dragonObjective?.currentKind ?? "elemental"} · H/A stacks ${dragonObjective?.homeStacks ?? 0}/${dragonObjective?.awayStacks ?? 0} · Soul ${dragonObjective?.soulClaimedBy ?? "-"}`
    : null;
  const timelineItems = runtime
    ? [...(runtime.events ?? [])].slice(-20).reverse().map((event, idx) => ({
      key: `${event.t}-${event.type}-${idx}`,
      minute: Math.max(0, Math.floor((event.t ?? 0) / 60)),
      label: event.text,
      side: event.text?.toUpperCase().includes("RED") ? "Away" : event.text?.toUpperCase().includes("BLUE") ? "Home" : "-",
    }))
    : importantEvents.slice(-20).reverse().map((evt, idx) => ({
      key: `${evt.minute}-${evt.event_type}-${idx}`,
      minute: evt.minute,
      label: evt.event_type.replace(/([A-Z])/g, " $1").trim(),
      side: evt.side,
    }));

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
            <p className="mt-2 text-lg font-heading">{durationMin}m</p>
          </div>
        </section>

        {runtime && (
          <section className="rounded-xl border border-white/10 bg-[#0a1433] p-4">
            <p className="text-[11px] uppercase tracking-[0.2em] text-gray-400 mb-2">K / D / A · CS · Gold</p>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
              <div>
                <p className="text-cyan-300 font-heading mb-1">{snapshot.home_team.name}</p>
                {homeChampions.map((champion) => (
                  <div key={champion.id} className="flex items-center justify-between py-1 border-b border-white/5">
                    <span>{champion.name}</span>
                    <span>{champion.kills}/{champion.deaths}/{champion.assists} · {champion.cs} · {champion.gold}</span>
                  </div>
                ))}
              </div>
              <div>
                <p className="text-orange-300 font-heading mb-1">{snapshot.away_team.name}</p>
                {awayChampions.map((champion) => (
                  <div key={champion.id} className="flex items-center justify-between py-1 border-b border-white/5">
                    <span>{champion.name}</span>
                    <span>{champion.kills}/{champion.deaths}/{champion.assists} · {champion.cs} · {champion.gold}</span>
                  </div>
                ))}
              </div>
            </div>
          </section>
        )}

        {dragonSummary && (
          <section className="rounded-xl border border-white/10 bg-[#0a1433] p-4">
            <p className="text-[11px] uppercase tracking-[0.2em] text-gray-400">Objective State</p>
            <p className="mt-2 text-sm text-gray-200">{dragonSummary}</p>
          </section>
        )}

        <section className="rounded-xl border border-white/10 bg-[#0a1433] p-4">
          <p className="text-[11px] uppercase tracking-[0.2em] text-gray-400 mb-2">Key Timeline</p>
          <div className="space-y-1 max-h-64 overflow-auto pr-1">
            {timelineItems.map((evt) => (
              <div key={evt.key} className="text-sm text-gray-200 flex items-center justify-between gap-3">
                <span>{evt.minute}'</span>
                <span className="flex-1">{evt.label}</span>
                <span className={evt.side === "Home" ? "text-cyan-300" : evt.side === "Away" ? "text-orange-300" : "text-gray-400"}>{evt.side}</span>
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
