use std::collections::HashMap;

use super::{LiveMatchState, MatchSnapshot};

// ---------------------------------------------------------------------------
// Snapshot generation — read-only view of match state for the UI
// ---------------------------------------------------------------------------

impl LiveMatchState {
    /// Get a full snapshot of the current match state for the UI.
    pub fn snapshot(&self) -> MatchSnapshot {
        let total_poss = self.home_possession_ticks + self.away_possession_ticks;
        let home_pct = if total_poss > 0 {
            self.home_possession_ticks as f64 / total_poss as f64 * 100.0
        } else {
            50.0
        };

        let home_yellows = HashMap::new();
        let away_yellows = HashMap::new();
        let home_team = self.home.clone();
        let away_team = self.away.clone();

        MatchSnapshot {
            phase: self.phase,
            current_minute: self.current_minute,
            home_score: self.home_score,
            away_score: self.away_score,
            possession: self.possession,
            ball_zone: self.ball_zone,
            home_team,
            away_team,
            home_bench: self.home_bench.clone(),
            away_bench: self.away_bench.clone(),
            home_possession_pct: home_pct,
            away_possession_pct: 100.0 - home_pct,
            events: self.events.clone(),
            home_subs_made: self.home_subs_made,
            away_subs_made: self.away_subs_made,
            max_subs: self.max_subs,
            home_set_pieces: super::SetPieceTakers::default(),
            away_set_pieces: super::SetPieceTakers::default(),
            substitutions: self.substitutions.clone(),
            allows_extra_time: self.allows_extra_time,
            home_yellows,
            away_yellows,
            sent_off: std::collections::HashSet::new(),
            lol_map: self.lol_map.clone(),
        }
    }
}
