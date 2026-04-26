use domain::team::{Team, TeamKind};
use ofm_core::academy::{
    eligible_academy_acquisition_options, validate_academy_acquisition,
    validate_parent_academy_link, AcademyAcquisitionOption, AcademyError, ErlAcademyCandidate,
    ErlAssignmentRule, ErlLeagueDefinition,
};

fn team(id: &str, country: &str, finance: i64) -> Team {
    let mut team = Team::new(
        id.to_string(),
        id.to_string(),
        id.to_uppercase(),
        country.to_string(),
        "City".to_string(),
        "Arena".to_string(),
        10_000,
    );
    team.finance = finance;
    team
}

fn erl(id: &str, country: &str, region: &str, reputation: u8, nearby: &[&str]) -> ErlLeagueDefinition {
    ErlLeagueDefinition {
        id: id.to_string(),
        name: id.to_string(),
        country_code: country.to_string(),
        region: region.to_string(),
        reputation,
        nearby_country_codes: nearby.iter().map(|country| country.to_string()).collect(),
    }
}

fn candidate(id: &str, league_id: &str, country: &str, reputation: u8, development_level: u8) -> ErlAcademyCandidate {
    ErlAcademyCandidate {
        source_team_id: id.to_string(),
        name: id.to_string(),
        short_name: id.to_uppercase(),
        logo_url: Some(format!("logos/{id}.svg")),
        erl_league_id: league_id.to_string(),
        country_code: country.to_string(),
        reputation,
        development_level,
    }
}

#[test]
fn domestic_candidate_teams_have_priority_over_nearby_fallbacks() {
    let options = eligible_academy_acquisition_options(
        "FR",
        &[
            erl("lfl", "FR", "western", 5, &[]),
            erl("superliga", "ES", "western", 4, &["FR"]),
        ],
        &[
            candidate("kcb", "lfl", "FR", 5, 4),
            candidate("heretics", "superliga", "ES", 4, 3),
        ],
    );

    assert_eq!(options.len(), 1);
    assert_eq!(options[0].source_team_id, "kcb");
    assert_eq!(options[0].erl_league_id, "lfl");
    assert_eq!(options[0].assignment_rule, ErlAssignmentRule::Domestic);
}

#[test]
fn nearby_candidate_fallbacks_are_offered_only_when_no_domestic_erl_exists() {
    let options = eligible_academy_acquisition_options(
        "BE",
        &[
            erl("lfl", "FR", "western", 5, &["BE"]),
            erl("superliga", "ES", "western", 4, &[]),
        ],
        &[
            candidate("kcb", "lfl", "FR", 5, 4),
            candidate("heretics", "superliga", "ES", 4, 3),
        ],
    );

    assert_eq!(options.len(), 1);
    assert_eq!(options[0].source_team_id, "kcb");
    assert_eq!(options[0].erl_league_id, "lfl");
    assert_eq!(options[0].assignment_rule, ErlAssignmentRule::Fallback);
    assert_eq!(
        options[0].fallback_reason.as_deref(),
        Some("BE has no domestic ERL; lfl is configured as nearby")
    );
}

#[test]
fn higher_reputation_candidate_costs_more_than_lower_reputation_candidate() {
    let options = eligible_academy_acquisition_options(
        "BE",
        &[
            erl("lfl", "FR", "western", 5, &["BE"]),
            erl("elite_series", "NL", "benelux", 2, &["BE"]),
        ],
        &[
            candidate("kcb", "lfl", "FR", 5, 4),
            candidate("elite-academy", "elite_series", "NL", 2, 1),
        ],
    );

    let high_reputation = options
        .iter()
        .find(|option| option.erl_league_id == "lfl")
        .unwrap();
    let low_reputation = options
        .iter()
        .find(|option| option.erl_league_id == "elite_series")
        .unwrap();

    assert!(high_reputation.acquisition_cost > low_reputation.acquisition_cost);
}

#[test]
fn insufficient_funds_blocks_academy_acquisition() {
    let parent = team("lec-team", "FR", 99_999);
    let option = AcademyAcquisitionOption {
        source_team_id: "kcb".to_string(),
        name: "Karmine Corp Blue".to_string(),
        short_name: "KCB".to_string(),
        logo_url: None,
        erl_league_id: "lfl".to_string(),
        erl_league_name: "LFL".to_string(),
        country_code: "FR".to_string(),
        assignment_rule: ErlAssignmentRule::Domestic,
        fallback_reason: None,
        reputation: 5,
        development_level: 4,
        acquisition_cost: 100_000,
    };

    assert_eq!(
        validate_academy_acquisition(&parent, &option),
        Err(AcademyError::InsufficientFunds {
            available: 99_999,
            required: 100_000,
        })
    );
}

#[test]
fn unrelated_parent_academy_movement_is_rejected() {
    let parent = team("lec-team", "FR", 1_000_000);
    let mut academy = team("other-academy", "FR", 0);
    academy.team_kind = TeamKind::Academy;
    academy.parent_team_id = Some("different-parent".to_string());

    assert_eq!(
        validate_parent_academy_link(&parent, &academy),
        Err(AcademyError::UnrelatedAcademy {
            parent_team_id: "lec-team".to_string(),
            academy_team_id: "other-academy".to_string(),
        })
    );
}
