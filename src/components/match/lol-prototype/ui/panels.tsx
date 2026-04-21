import type { SimEvent } from "../engine/types";

function ts(sec: number) {
  const m = Math.floor(sec / 60).toString().padStart(2, "0");
  const s = Math.floor(sec % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

interface ScoreboardProps {
  timeSec: number;
  status: string;
  blue: { kills: number; towers: number; dragons: number; barons: number; gold: number; avgLevel: number };
  red: { kills: number; towers: number; dragons: number; barons: number; gold: number; avgLevel: number };
}

function compactGold(gold: number) {
  if (gold < 1000) return `${Math.round(gold)}g`;
  return `${(gold / 1000).toFixed(1)}k`;
}

export function ScoreboardPanel({ timeSec, status, blue, red }: ScoreboardProps) {
  return (
    <div className="rounded-xl border border-cyan-500/25 bg-[#0a142b] p-3 text-slate-100">
      <div className="mb-2 flex items-center justify-between text-xs uppercase tracking-widest text-cyan-300">
        <span>{status}</span>
        <span>{ts(timeSec)}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs">
        <div className="rounded border border-cyan-500/20 p-2">
          <p className="font-semibold text-cyan-300">Blue</p>
          <p>K {blue.kills} · T {blue.towers}</p>
          <p>D {blue.dragons} · B {blue.barons}</p>
          <p>G {compactGold(blue.gold)} · L {blue.avgLevel.toFixed(1)}</p>
        </div>
        <div className="rounded border border-rose-500/20 p-2">
          <p className="font-semibold text-rose-300">Red</p>
          <p>K {red.kills} · T {red.towers}</p>
          <p>D {red.dragons} · B {red.barons}</p>
          <p>G {compactGold(red.gold)} · L {red.avgLevel.toFixed(1)}</p>
        </div>
      </div>
    </div>
  );
}

interface EventsProps {
  events: SimEvent[];
}

export function EventFeedPanel({ events }: EventsProps) {
  return (
    <div className="rounded-xl border border-cyan-500/25 bg-[#0a142b] p-3 text-slate-100">
      <p className="mb-2 text-xs uppercase tracking-widest text-cyan-300">Events</p>
      <div className="max-h-64 overflow-auto space-y-1 text-xs">
        {events.map((e, idx) => (
          <div key={`${e.t}-${idx}-${e.type}`} className="flex gap-2">
            <span className="w-11 text-cyan-300">{ts(e.t)}</span>
            <span className="text-slate-200">{e.text}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
