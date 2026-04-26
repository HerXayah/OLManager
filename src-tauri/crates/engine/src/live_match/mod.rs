mod lol_map;
mod simulation;
mod snapshot;

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::event::MatchEvent;
use crate::report::MatchReport;
use crate::types::{MatchConfig, PlayStyle, PlayerData, Side, TeamData, Zone};
pub use lol_map::{
    LolDragonKind, LolDragonState, LolGrubsState, LolLaneState, LolMapState, LolObjectiveState,
    LolObjectivesState, LolRole, LolTask, LolTeamStructuresState, LolUnitState,
};

// ---------------------------------------------------------------------------
// MatchPhase — tracks where we are in the match lifecycle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchPhase {
    PreGame,
    Live,
    Finished,
}

// ---------------------------------------------------------------------------
// MatchCommand — actions injected by user or AI between minutes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchCommand {
    PreMatchSwap {
        side: Side,
        player_off_id: String,
        player_on_id: String,
    },
    Substitute {
        side: Side,
        player_off_id: String,
        player_on_id: String,
    },
    ChangeFormation {
        side: Side,
        formation: String,
    },
    ChangePlayStyle {
        side: Side,
        play_style: PlayStyle,
    },
    SetFreeKickTaker {
        side: Side,
        player_id: String,
    },
    SetCornerTaker {
        side: Side,
        player_id: String,
    },
    SetPenaltyTaker {
        side: Side,
        player_id: String,
    },
    SetCaptain {
        side: Side,
        player_id: String,
    },
}

// ---------------------------------------------------------------------------
// SubstitutionRecord — tracks a substitution that was made
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstitutionRecord {
    pub minute: u8,
    pub side: Side,
    pub player_off_id: String,
    pub player_on_id: String,
}

// ---------------------------------------------------------------------------
// SetPieceTakers — designated set piece takers for a side
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SetPieceTakers {
    pub free_kick_taker: Option<String>,
    pub corner_taker: Option<String>,
    pub penalty_taker: Option<String>,
    pub captain: Option<String>,
}

// ---------------------------------------------------------------------------
// MinuteResult — what happened during one simulated minute
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinuteResult {
    pub minute: u8,
    pub phase: MatchPhase,
    pub events: Vec<MatchEvent>,
    pub home_score: u8,
    pub away_score: u8,
    pub possession: Side,
    pub ball_zone: Zone,
    pub is_finished: bool,
}

// ---------------------------------------------------------------------------
// MatchSnapshot — full read-only view of the match for the UI
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchSnapshot {
    pub phase: MatchPhase,
    pub current_minute: u8,
    pub home_score: u8,
    pub away_score: u8,
    pub possession: Side,
    pub ball_zone: Zone,
    pub home_team: TeamData,
    pub away_team: TeamData,
    pub home_bench: Vec<PlayerData>,
    pub away_bench: Vec<PlayerData>,
    pub home_possession_pct: f64,
    pub away_possession_pct: f64,
    pub events: Vec<MatchEvent>,
    pub home_subs_made: u8,
    pub away_subs_made: u8,
    pub max_subs: u8,
    pub home_set_pieces: SetPieceTakers,
    pub away_set_pieces: SetPieceTakers,
    pub substitutions: Vec<SubstitutionRecord>,
    pub allows_extra_time: bool,
    pub home_yellows: HashMap<String, u8>,
    pub away_yellows: HashMap<String, u8>,
    pub sent_off: HashSet<String>,
    pub lol_map: LolMapState,
}

// ---------------------------------------------------------------------------
// LiveMatchState — the core step-by-step simulation engine
// ---------------------------------------------------------------------------

pub struct LiveMatchState {
    // Teams (owned — subs mutate the player list)
    home: TeamData,
    away: TeamData,

    // Match progress
    phase: MatchPhase,
    current_minute: u8,

    // Score
    home_score: u8,
    away_score: u8,

    // Field state
    ball_zone: Zone,
    possession: Side,

    // Events log
    events: Vec<MatchEvent>,

    // Possession tracking
    home_possession_ticks: u32,
    away_possession_ticks: u32,

    // Substitutions
    home_subs_made: u8,
    away_subs_made: u8,
    max_subs: u8,
    substitutions: Vec<SubstitutionRecord>,

    // Bench players (available for substitution)
    home_bench: Vec<PlayerData>,
    away_bench: Vec<PlayerData>,

    // Extra time / knockout
    allows_extra_time: bool,

    // LoL objective/map state (incremental overlay layer)
    lol_map: LolMapState,
}

impl LiveMatchState {
    /// Create a new live match. `starting_xi` are already in `home.players` / `away.players`.
    /// Bench players are separate and available for substitution.
    pub fn new(
        home: TeamData,
        away: TeamData,
        _config: MatchConfig,
        home_bench: Vec<PlayerData>,
        away_bench: Vec<PlayerData>,
        allows_extra_time: bool,
    ) -> Self {
        let mut lol_map = LolMapState::new();
        lol_map.seed_units(&home, &away);

        Self {
            home,
            away,
            phase: MatchPhase::PreGame,
            current_minute: 0,
            home_score: 0,
            away_score: 0,
            ball_zone: Zone::Midfield,
            possession: Side::Home,
            events: Vec::with_capacity(300),
            home_possession_ticks: 0,
            away_possession_ticks: 0,
            home_subs_made: 0,
            away_subs_made: 0,
            max_subs: 5,
            substitutions: Vec::new(),
            home_bench,
            away_bench,
            allows_extra_time,
            lol_map,
        }
    }

    /// Step one minute forward. Returns the events that occurred.
    pub fn step_minute<R: Rng>(&mut self, rng: &mut R) -> MinuteResult {
        match self.phase {
            MatchPhase::PreGame => self.start_match(rng),
            MatchPhase::Live => self.play_minute(rng),
            MatchPhase::Finished => self.make_result(true),
        }
    }

    /// Apply a command (substitution, tactic change, set piece assignment).
    pub fn apply_command(&mut self, cmd: MatchCommand) -> Result<(), String> {
        match cmd {
            MatchCommand::PreMatchSwap {
                side,
                player_off_id,
                player_on_id,
            } => {
                if self.phase != MatchPhase::PreGame {
                    return Err("Pre-match swap only allowed before kickoff".to_string());
                }
                self.apply_pre_match_swap(side, &player_off_id, &player_on_id)
            }
            MatchCommand::Substitute {
                side,
                player_off_id,
                player_on_id,
            } => self.apply_substitution(side, &player_off_id, &player_on_id),
            MatchCommand::ChangeFormation { side, formation } => {
                self.team_mut(side).formation = formation;
                Ok(())
            }
            MatchCommand::ChangePlayStyle { side, play_style } => {
                self.team_mut(side).play_style = play_style;
                Ok(())
            }
            MatchCommand::SetFreeKickTaker { side, player_id } => {
                let _ = (side, player_id);
                Ok(())
            }
            MatchCommand::SetCornerTaker { side, player_id } => {
                let _ = (side, player_id);
                Ok(())
            }
            MatchCommand::SetPenaltyTaker { side, player_id } => {
                let _ = (side, player_id);
                Ok(())
            }
            MatchCommand::SetCaptain { side, player_id } => {
                let _ = (side, player_id);
                Ok(())
            }
        }
    }

    /// Convert the finished match into a MatchReport.
    pub fn into_report(self) -> MatchReport {
        let tracked_player_ids = self
            .home
            .players
            .iter()
            .chain(self.away.players.iter())
            .map(|player| player.id.clone())
            .collect();

        MatchReport::from_events_with_lol_snapshot(
            self.events,
            self.home_possession_ticks,
            self.away_possession_ticks,
            self.current_minute,
            tracked_player_ids,
            &self.lol_map.units,
            self.lol_map.destroyed_nexus_by,
        )
    }

    /// Is the match finished?
    pub fn is_finished(&self) -> bool {
        self.phase == MatchPhase::Finished
    }

    /// Current phase
    pub fn phase(&self) -> MatchPhase {
        self.phase
    }

    /// Current minute
    pub fn minute(&self) -> u8 {
        self.current_minute
    }

    /// Get the bench for a side
    pub fn bench(&self, side: Side) -> &[PlayerData] {
        match side {
            Side::Home => &self.home_bench,
            Side::Away => &self.away_bench,
        }
    }

    /// Simulate a red card for a player (adds to sent_off set).
    /// Primarily used for testing substitution guards.
    pub fn test_send_off(&mut self, player_id: &str) {
        let _ = player_id;
    }

    pub(super) fn team_mut(&mut self, side: Side) -> &mut TeamData {
        match side {
            Side::Home => &mut self.home,
            Side::Away => &mut self.away,
        }
    }

    pub(super) fn add_goal(&mut self, side: Side) {
        match side {
            Side::Home => self.home_score = self.home_score.saturating_add(1),
            Side::Away => self.away_score = self.away_score.saturating_add(1),
        }
    }

    fn apply_substitution(
        &mut self,
        side: Side,
        player_off_id: &str,
        player_on_id: &str,
    ) -> Result<(), String> {
        let (team, bench, subs_made) = match side {
            Side::Home => (
                &mut self.home,
                &mut self.home_bench,
                &mut self.home_subs_made,
            ),
            Side::Away => (
                &mut self.away,
                &mut self.away_bench,
                &mut self.away_subs_made,
            ),
        };

        if *subs_made >= self.max_subs {
            return Err("Maximum substitutions reached".to_string());
        }

        let on_idx = bench
            .iter()
            .position(|p| p.id == player_on_id)
            .ok_or_else(|| "Incoming player not in bench".to_string())?;
        let off_idx = team
            .players
            .iter()
            .position(|p| p.id == player_off_id)
            .ok_or_else(|| "Outgoing player not in lineup".to_string())?;

        let incoming = bench.remove(on_idx);
        let outgoing = std::mem::replace(&mut team.players[off_idx], incoming);
        bench.push(outgoing.clone());
        *subs_made = subs_made.saturating_add(1);
        self.substitutions.push(SubstitutionRecord {
            minute: self.current_minute,
            side,
            player_off_id: player_off_id.to_string(),
            player_on_id: player_on_id.to_string(),
        });
        Ok(())
    }

    fn apply_pre_match_swap(
        &mut self,
        side: Side,
        player_off_id: &str,
        player_on_id: &str,
    ) -> Result<(), String> {
        let (team, bench) = match side {
            Side::Home => (&mut self.home, &mut self.home_bench),
            Side::Away => (&mut self.away, &mut self.away_bench),
        };

        let on_idx = bench
            .iter()
            .position(|p| p.id == player_on_id)
            .ok_or_else(|| "Incoming player not in bench".to_string())?;
        let off_idx = team
            .players
            .iter()
            .position(|p| p.id == player_off_id)
            .ok_or_else(|| "Outgoing player not in lineup".to_string())?;

        let incoming = bench.remove(on_idx);
        let outgoing = std::mem::replace(&mut team.players[off_idx], incoming);
        bench.push(outgoing);
        Ok(())
    }
}
