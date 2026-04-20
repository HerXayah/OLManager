use domain::team::{Facilities, FacilityType, Team};

pub const BASE_FACILITY_UPGRADE_COST: i64 = 250_000;

fn facility_level(facilities: &Facilities, facility_type: &FacilityType) -> u8 {
    match facility_type {
        FacilityType::Training => facilities.training,
        FacilityType::Medical => facilities.medical,
        FacilityType::Scouting => facilities.scouting,
    }
}

pub fn next_upgrade_cost(team: &Team, facility_type: &FacilityType) -> i64 {
    i64::from(facility_level(&team.facilities, facility_type)) * BASE_FACILITY_UPGRADE_COST
}

pub fn upgrade_facility(team: &mut Team, facility_type: FacilityType) -> Result<i64, String> {
    let cost = next_upgrade_cost(team, &facility_type);
    if team.finance < cost {
        return Err(format!(
            "Insufficient funds for facility upgrade: need €{}",
            cost
        ));
    }

    team.finance -= cost;
    team.season_expenses += cost;

    match facility_type {
        FacilityType::Training => {
            team.facilities.training = team.facilities.training.saturating_add(1);
        }
        FacilityType::Medical => {
            team.facilities.medical = team.facilities.medical.saturating_add(1);
        }
        FacilityType::Scouting => {
            team.facilities.scouting = team.facilities.scouting.saturating_add(1);
        }
    }

    Ok(cost)
}
