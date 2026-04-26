use domain::team::{Team, TeamKind};
use serde::{Deserialize, Serialize};

pub use domain::team::ErlAssignmentRule;

const BASE_ACADEMY_ACQUISITION_COST: i64 = 100_000;
const REPUTATION_COST_MULTIPLIER: i64 = 40_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErlLeagueDefinition {
    pub id: String,
    pub name: String,
    pub country_code: String,
    pub region: String,
    pub reputation: u8,
    pub nearby_country_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcademyAcquisitionOption {
    pub source_team_id: String,
    pub name: String,
    pub short_name: String,
    pub logo_url: Option<String>,
    pub erl_league_id: String,
    pub erl_league_name: String,
    pub country_code: String,
    pub assignment_rule: ErlAssignmentRule,
    pub fallback_reason: Option<String>,
    pub reputation: u8,
    pub development_level: u8,
    pub acquisition_cost: i64,
}

pub type AcademyCreationOption = AcademyAcquisitionOption;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErlAcademyCandidate {
    pub source_team_id: String,
    pub name: String,
    pub short_name: String,
    pub logo_url: Option<String>,
    pub erl_league_id: String,
    pub country_code: String,
    pub reputation: u8,
    pub development_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcademyError {
    ParentMustBeMainTeam { team_id: String },
    AcademyAlreadyExists { parent_team_id: String, academy_team_id: String },
    InsufficientFunds { available: i64, required: i64 },
    UnrelatedAcademy { parent_team_id: String, academy_team_id: String },
}

pub fn eligible_academy_acquisition_options(
    team_country_code: &str,
    leagues: &[ErlLeagueDefinition],
    candidates: &[ErlAcademyCandidate],
) -> Vec<AcademyAcquisitionOption> {
    let domestic_leagues: Vec<_> = leagues
        .iter()
        .filter(|erl| country_matches(&erl.country_code, team_country_code))
        .collect();

    if !domestic_leagues.is_empty() {
        return acquisition_options_for_leagues(candidates, &domestic_leagues, ErlAssignmentRule::Domestic, None);
    }

    let fallback_leagues: Vec<_> = leagues
        .iter()
        .filter(|erl| {
            erl.nearby_country_codes
                .iter()
                .any(|country| country_matches(country, team_country_code))
        })
        .collect();

    acquisition_options_for_leagues(
        candidates,
        &fallback_leagues,
        ErlAssignmentRule::Fallback,
        Some(team_country_code),
    )
}

pub fn eligible_academy_creation_options(
    team_country_code: &str,
    catalog: &[ErlLeagueDefinition],
) -> Vec<AcademyCreationOption> {
    let domestic: Vec<_> = catalog
        .iter()
        .filter(|erl| country_matches(&erl.country_code, team_country_code))
        .map(|erl| acquisition_option_from_league(erl, ErlAssignmentRule::Domestic, None))
        .collect();

    if !domestic.is_empty() {
        return domestic;
    }

    catalog
        .iter()
        .filter(|erl| {
            erl.nearby_country_codes
                .iter()
                .any(|country| country_matches(country, team_country_code))
        })
        .map(|erl| {
            acquisition_option_from_league(
                erl,
                ErlAssignmentRule::Fallback,
                Some(format!(
                    "{} has no domestic ERL; {} is configured as nearby",
                    team_country_code, erl.id
                )),
            )
        })
        .collect()
}

pub fn validate_academy_creation(
    parent: &Team,
    option: &AcademyCreationOption,
) -> Result<(), AcademyError> {
    validate_academy_acquisition(parent, option)
}

pub fn validate_academy_acquisition(
    parent: &Team,
    option: &AcademyAcquisitionOption,
) -> Result<(), AcademyError> {
    if !parent.is_main() {
        return Err(AcademyError::ParentMustBeMainTeam {
            team_id: parent.id.clone(),
        });
    }

    if let Some(academy_team_id) = &parent.academy_team_id {
        return Err(AcademyError::AcademyAlreadyExists {
            parent_team_id: parent.id.clone(),
            academy_team_id: academy_team_id.clone(),
        });
    }

    if parent.finance < option.acquisition_cost {
        return Err(AcademyError::InsufficientFunds {
            available: parent.finance,
            required: option.acquisition_cost,
        });
    }

    Ok(())
}

pub fn validate_parent_academy_link(parent: &Team, academy: &Team) -> Result<(), AcademyError> {
    let linked_from_parent = parent.academy_team_id.as_deref() == Some(academy.id.as_str());
    let linked_from_academy = academy.parent_team_id.as_deref() == Some(parent.id.as_str());

    if academy.team_kind == TeamKind::Academy && (linked_from_parent || linked_from_academy) {
        return Ok(());
    }

    Err(AcademyError::UnrelatedAcademy {
        parent_team_id: parent.id.clone(),
        academy_team_id: academy.id.clone(),
    })
}

fn acquisition_option_from_league(
    erl: &ErlLeagueDefinition,
    assignment_rule: ErlAssignmentRule,
    fallback_reason: Option<String>,
) -> AcademyCreationOption {
    AcademyAcquisitionOption {
        source_team_id: format!("{}-academy-candidate", erl.id),
        name: erl.name.clone(),
        short_name: erl.id.clone(),
        logo_url: None,
        erl_league_id: erl.id.clone(),
        erl_league_name: erl.name.clone(),
        country_code: erl.country_code.clone(),
        assignment_rule,
        fallback_reason,
        reputation: erl.reputation,
        development_level: erl.reputation,
        acquisition_cost: acquisition_cost_for_reputation(erl.reputation),
    }
}

fn acquisition_options_for_leagues(
    candidates: &[ErlAcademyCandidate],
    leagues: &[&ErlLeagueDefinition],
    assignment_rule: ErlAssignmentRule,
    fallback_team_country_code: Option<&str>,
) -> Vec<AcademyAcquisitionOption> {
    candidates
        .iter()
        .filter_map(|candidate| {
            let league = leagues
                .iter()
                .copied()
                .find(|league| league.id == candidate.erl_league_id)?;
            let fallback_reason = fallback_team_country_code.map(|country| {
                format!(
                    "{} has no domestic ERL; {} is configured as nearby",
                    country, league.id
                )
            });
            Some(AcademyAcquisitionOption {
                source_team_id: candidate.source_team_id.clone(),
                name: candidate.name.clone(),
                short_name: candidate.short_name.clone(),
                logo_url: candidate.logo_url.clone(),
                erl_league_id: league.id.clone(),
                erl_league_name: league.name.clone(),
                country_code: candidate.country_code.clone(),
                assignment_rule: assignment_rule.clone(),
                fallback_reason,
                reputation: candidate.reputation,
                development_level: candidate.development_level,
                acquisition_cost: acquisition_cost_for_candidate(candidate),
            })
        })
        .collect()
}

fn acquisition_cost_for_candidate(candidate: &ErlAcademyCandidate) -> i64 {
    BASE_ACADEMY_ACQUISITION_COST
        + i64::from(candidate.reputation) * REPUTATION_COST_MULTIPLIER
        + i64::from(candidate.development_level) * 20_000
}

fn acquisition_cost_for_reputation(reputation: u8) -> i64 {
    BASE_ACADEMY_ACQUISITION_COST + i64::from(reputation) * REPUTATION_COST_MULTIPLIER
}

fn country_matches(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}
