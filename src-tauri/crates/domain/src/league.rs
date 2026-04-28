use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct League {
    pub id: String,
    pub name: String,
    pub season: u32,
    pub fixtures: Vec<Fixture>,
    pub standings: Vec<StandingEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FixtureCompetition {
    #[default]
    League,
    Friendly,
    PreseasonTournament,
    Playoffs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Fixture {
    pub id: String,
    pub matchday: u32,
    pub date: String,
    pub home_team_id: String,
    pub away_team_id: String,
    pub competition: FixtureCompetition,
    #[serde(default = "default_best_of")]
    pub best_of: u8,
    pub status: FixtureStatus,
    pub result: Option<MatchResult>,
}

fn default_best_of() -> u8 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FixtureStatus {
    Scheduled,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MatchEndReason {
    NexusDestroyed,
    TimeLimit,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MatchResult {
    #[serde(alias = "home_goals")]
    pub home_wins: u8,
    #[serde(alias = "away_goals")]
    pub away_wins: u8,
    pub ended_by: MatchEndReason,
    pub game_duration_seconds: u32,
    pub report: Option<CompactMatchReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct CompactMatchReport {
    #[serde(default, skip_serializing)]
    pub total_minutes: u32,
    pub game_duration_seconds: u32,
    pub home_stats: CompactTeamMatchStats,
    pub away_stats: CompactTeamMatchStats,
    pub events: Vec<CompactMatchEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct CompactTeamMatchStats {
    #[serde(default, skip_serializing)]
    pub possession_pct: u8,
    pub kills: u16,
    pub deaths: u16,
    pub gold_earned: u32,
    pub damage_dealt: u32,
    pub objectives: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct CompactMatchEvent {
    pub minute: u8,
    pub event_type: String,
    pub side: String,
    pub player_id: Option<String>,
    pub secondary_player_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandingEntry {
    pub team_id: String,
    pub played: u32,
    pub won: u32,
    pub drawn: u32,
    pub lost: u32,
    pub goals_for: u32,
    pub goals_against: u32,
    pub points: u32,
}

impl StandingEntry {
    pub fn new(team_id: String) -> Self {
        Self {
            team_id,
            played: 0,
            won: 0,
            drawn: 0,
            lost: 0,
            goals_for: 0,
            goals_against: 0,
            points: 0,
        }
    }

    pub fn goal_difference(&self) -> i32 {
        self.goals_for as i32 - self.goals_against as i32
    }

    pub fn record_result(&mut self, goals_for: u8, goals_against: u8) {
        self.played += 1;
        self.goals_for += goals_for as u32;
        self.goals_against += goals_against as u32;
        if goals_for > goals_against {
            self.won += 1;
            self.points += 3;
        } else if goals_for == goals_against {
            self.drawn += 1;
            self.points += 1;
        } else {
            self.lost += 1;
        }
    }
}

impl Fixture {
    pub fn counts_for_league_standings(&self) -> bool {
        matches!(self.competition, FixtureCompetition::League)
    }
}

impl League {
    pub fn new(id: String, name: String, season: u32, team_ids: &[String]) -> Self {
        let standings = team_ids
            .iter()
            .map(|tid| StandingEntry::new(tid.clone()))
            .collect();

        Self {
            id,
            name,
            season,
            fixtures: Vec::new(),
            standings,
        }
    }

    pub fn sorted_standings(&self) -> Vec<StandingEntry> {
        let mut sorted = self.standings.clone();
        sorted.sort_by(|a, b| {
            b.points
                .cmp(&a.points)
                .then(b.goal_difference().cmp(&a.goal_difference()))
                .then(b.goals_for.cmp(&a.goals_for))
        });
        sorted
    }
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            id: String::new(),
            matchday: 0,
            date: String::new(),
            home_team_id: String::new(),
            away_team_id: String::new(),
            competition: FixtureCompetition::League,
            best_of: default_best_of(),
            status: FixtureStatus::Scheduled,
            result: None,
        }
    }
}
