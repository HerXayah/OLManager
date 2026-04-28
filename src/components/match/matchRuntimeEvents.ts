import type { LolSimV1RuntimeState } from "./lol-prototype/backend/contract-v1";
import type { MatchEvent, MatchSnapshot } from "./types";

type RuntimeEvent = NonNullable<LolSimV1RuntimeState["events"]>[number];

function runtimeEventSide(event: RuntimeEvent): MatchEvent["side"] {
  const text = (event.text ?? "").toUpperCase();
  const firstSideToken = text.match(/\b(BLUE|HOME|RED|AWAY)\b/)?.[1];
  if (firstSideToken === "RED" || firstSideToken === "AWAY") return "Away";
  return "Home";
}

function runtimeEventType(event: RuntimeEvent): string {
  const text = (event.text ?? "").toLowerCase();
  const type = (event.type ?? "").toLowerCase();

  if (text.includes("first blood")) return "FirstBlood";
  if (text.includes("voidgrub")) return "VoidGrubs";
  if (text.includes("dragon soul") || text.includes(" soul")) return "DragonSoul";
  if (text.includes("elder")) return "ElderDragon";
  if (text.includes("baron")) return "Baron";
  if (text.includes("herald")) return "Herald";
  if (text.includes("inhib")) return "Inhibitor";
  if (text.includes("tower") || text.includes("turret")) return "Tower";
  if (text.includes("dragon")) return "Dragon";
  if (text.includes("nexus")) return "NexusDestroyed";

  switch (type) {
    case "kill":
      return "Kill";
    case "tower":
      return "Tower";
    case "dragon":
      return "Dragon";
    case "baron":
      return "Baron";
    case "nexus":
      return "NexusDestroyed";
    default:
      return event.type ?? "Info";
  }
}

function eventKey(event: MatchEvent): string {
  return [
    event.minute,
    event.event_type,
    event.side,
    event.zone,
    event.player_id ?? "",
    event.secondary_player_id ?? "",
  ].join("|");
}

/**
 * Adapts LoL runtime telemetry to the legacy MatchSnapshot event model used by
 * post-match systems. Runtime events only expose timestamp/type/text today, so
 * side and semantic event type are derived from the broadcast text locally here.
 */
export function mapRuntimeEventsToMatchEvents(events: LolSimV1RuntimeState["events"] | undefined): MatchEvent[] {
  return (events ?? []).map((event) => ({
    minute: Math.max(0, Math.floor((event.t ?? 0) / 60)),
    event_type: runtimeEventType(event),
    side: runtimeEventSide(event),
    zone: "mid",
    player_id: null,
    secondary_player_id: null,
  }));
}

export function mergeMatchEvents(existingEvents: MatchEvent[] | undefined, incomingEvents: MatchEvent[]): MatchEvent[] {
  const merged: MatchEvent[] = [];
  const seen = new Set<string>();

  for (const event of [...(existingEvents ?? []), ...incomingEvents]) {
    const key = eventKey(event);
    if (seen.has(key)) continue;
    seen.add(key);
    merged.push(event);
  }

  return merged;
}

export function mergeRuntimeEventsIntoSnapshot(
  snapshot: MatchSnapshot,
  runtimeEvents: LolSimV1RuntimeState["events"] | undefined,
): MatchSnapshot {
  return {
    ...snapshot,
    events: mergeMatchEvents(snapshot.events, mapRuntimeEventsToMatchEvents(runtimeEvents)),
  };
}
