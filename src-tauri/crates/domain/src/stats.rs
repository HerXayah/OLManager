use crate::league::FixtureCompetition;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StatsState {
    pub player_matches: Vec<PlayerMatchStatsRecord>,
    pub team_matches: Vec<TeamMatchStatsRecord>,
}

impl StatsState {
    pub fn append(&mut self, other: StatsState) {
        self.player_matches.extend(other.player_matches);
        self.team_matches.extend(other.team_matches);
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MatchOutcome {
    Win,
    #[serde(alias = "Draw")]
    #[default]
    Loss,
}

impl MatchOutcome {
    pub fn from_scores(team_score: u8, opponent_score: u8) -> Self {
        if team_score > opponent_score {
            Self::Win
        } else {
            // LoL no permite empate en el core path; cualquier no-victoria es derrota.
            Self::Loss
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TeamSide {
    #[serde(alias = "Home")]
    #[default]
    Blue,
    #[serde(alias = "Away")]
    Red,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum LolRole {
    Top,
    Jungle,
    Mid,
    Adc,
    Support,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct PlayerMatchStatsRecord {
    pub fixture_id: String,
    pub season: u32,
    pub matchday: u32,
    pub date: String,
    pub competition: FixtureCompetition,
    pub player_id: String,
    pub team_id: String,
    pub opponent_team_id: String,
    pub side: TeamSide,
    pub result: MatchOutcome,
    pub role: LolRole,
    #[serde(alias = "champion_id")]
    pub champion: Option<String>,
    pub duration_seconds: u32,
    pub kills: u16,
    pub deaths: u16,
    pub assists: u16,
    pub creep_score: u16,
    pub gold_earned: u32,
    pub damage_dealt: u32,
    pub vision_score: u16,
    pub wards_placed: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct TeamMatchStatsRecord {
    pub fixture_id: String,
    pub season: u32,
    pub matchday: u32,
    pub date: String,
    pub competition: FixtureCompetition,
    pub team_id: String,
    pub opponent_team_id: String,
    pub side: TeamSide,
    pub result: MatchOutcome,
    pub duration_seconds: u32,
    pub kills: u16,
    pub deaths: u16,
    pub gold_earned: u32,
    pub damage_dealt: u32,
    pub objectives: u16,
}
