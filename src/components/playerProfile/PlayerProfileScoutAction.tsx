import { ScanSearch } from "lucide-react";
import type {
    PlayerProfileScoutStatus,
    ScoutAvailability,
} from "./PlayerProfile.scouting";

interface PlayerProfileScoutActionProps {
    availability: ScoutAvailability;
    scoutStatus: PlayerProfileScoutStatus;
    scoutError: string | null;
    onScout: () => void;
}

export default function PlayerProfileScoutAction({
    availability,
    scoutStatus,
    scoutError,
    onScout,
}: PlayerProfileScoutActionProps) {
    if (availability.scouts.length === 0) {
        return (
            <p className="text-xs text-gray-500">Hire a scout to evaluate players</p>
        );
    }

    if (availability.alreadyScouting || scoutStatus === "sent") {
        return (
            <span className="text-xs text-primary-400 font-heading font-bold uppercase tracking-wider flex items-center gap-1.5">
                <ScanSearch className="w-3.5 h-3.5" /> Scouting in progress
            </span>
        );
    }

    return (
        <>
            <button
                disabled={!availability.canScout || scoutStatus === "sending"}
                onClick={onScout}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary-500/20 text-primary-400 hover:bg-primary-500/30 transition-colors text-xs font-heading font-bold uppercase tracking-wider disabled:opacity-50"
            >
                <ScanSearch className="w-3.5 h-3.5" />
                {scoutStatus === "sending" ? "Sending..." : "Scout Player"}
            </button>
            {scoutError ? (
                <p className="text-xs text-red-400 mt-1">{scoutError}</p>
            ) : null}
        </>
    );
}