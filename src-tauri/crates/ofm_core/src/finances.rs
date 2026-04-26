use crate::game::Game;
use chrono::Datelike;
use domain::message::*;
use domain::team::{
    main_facility_module_catalog, Facilities, MainFacilityModuleKind, Sponsorship,
    SponsorshipBonusCriterion, Team,
};
use rand::RngExt;

const MAIN_HUB_UPKEEP_PER_EXTRA_LEVEL: i64 = 20_000;
const ESPORTS_SPONSOR_THEME_MULTIPLIER: f64 = 1.15;

pub struct FacilityUpkeepBreakdown {
    pub monthly_total: i64,
    pub hub_extra_level_total: i64,
    pub module_extra_level_total: i64,
}

fn action(id: &str, label: &str, label_key: &str, action_type: ActionType) -> MessageAction {
    MessageAction {
        id: id.to_string(),
        label: label.to_string(),
        action_type,
        resolved: false,
        label_key: Some(label_key.to_string()),
    }
}

pub fn calc_wages(game: &Game, team_id: &str) -> i64 {
    let player_wages: i64 = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(team_id))
        .map(|player| player.wage as i64 / 52)
        .sum();

    let staff_wages: i64 = game
        .staff
        .iter()
        .filter(|staff_member| staff_member.team_id.as_deref() == Some(team_id))
        .map(|staff_member| staff_member.wage as i64 / 52)
        .sum();

    player_wages + staff_wages
}

pub fn calc_annual_wages(game: &Game, team_id: &str) -> i64 {
    let player_wages: i64 = game
        .players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(team_id))
        .map(|player| player.wage as i64)
        .sum();

    let staff_wages: i64 = game
        .staff
        .iter()
        .filter(|staff_member| staff_member.team_id.as_deref() == Some(team_id))
        .map(|staff_member| staff_member.wage as i64)
        .sum();

    player_wages + staff_wages
}

pub fn calc_cash_runway_weeks(balance: i64, projected_weekly_net: i64) -> Option<i64> {
    if projected_weekly_net >= 0 {
        return None;
    }

    Some(std::cmp::max(0, balance / projected_weekly_net.abs()))
}

pub fn calc_matchday(
    stadium_capacity: u32,
    home_match_count: i64,
    attendance_pct: f64,
    avg_ticket: f64,
) -> i64 {
    let revenue_per_match = (stadium_capacity as f64 * attendance_pct * avg_ticket) as i64;

    revenue_per_match * home_match_count
}

pub fn calc_upkeep(team: &Team) -> i64 {
    facility_upkeep_breakdown(&team.facilities).monthly_total
}

pub fn facility_upkeep_breakdown(facilities: &Facilities) -> FacilityUpkeepBreakdown {
    let hub_level = facilities.as_main_facility_hub().level;
    let hub_extra_level_total = i64::from(hub_level.saturating_sub(1))
        * MAIN_HUB_UPKEEP_PER_EXTRA_LEVEL;
    let module_extra_level_total = main_facility_module_catalog()
        .iter()
        .map(|definition| canonical_module_upkeep(facilities, definition.kind))
        .sum();

    FacilityUpkeepBreakdown {
        monthly_total: hub_extra_level_total + module_extra_level_total,
        hub_extra_level_total,
        module_extra_level_total,
    }
}

fn canonical_module_upkeep(facilities: &Facilities, module: MainFacilityModuleKind) -> i64 {
    let extra_levels = i64::from(facilities.module_level(module).saturating_sub(1));
    let per_level = match module {
        MainFacilityModuleKind::ScrimsRoom => 20_000,
        MainFacilityModuleKind::AnalysisRoom => 15_000,
        MainFacilityModuleKind::BootcampArea => 15_000,
        MainFacilityModuleKind::RecoverySuite => 10_000,
        MainFacilityModuleKind::ScoutingLab => 10_000,
        MainFacilityModuleKind::ContentStudio => 0,
    };

    extra_levels * per_level
}

pub fn facility_module_sponsorship_multiplier(facilities: &Facilities) -> f64 {
    let extra_content_levels = facilities
        .module_level(MainFacilityModuleKind::ContentStudio)
        .saturating_sub(1);

    1.0 + f64::from(extra_content_levels) * 0.02
}

fn sponsorship_theme_multiplier(sponsor_name: &str) -> f64 {
    let normalized = sponsor_name.to_lowercase();
    if normalized.contains("esport")
        || normalized.contains("gaming")
        || normalized.contains("pc")
        || normalized.contains("hardware")
        || normalized.contains("tech")
    {
        ESPORTS_SPONSOR_THEME_MULTIPLIER
    } else {
        1.0
    }
}

pub fn calc_sponsorship_income(
    current_position: Option<u32>,
    recent_form: &[String],
    sponsorship: &Sponsorship,
) -> i64 {
    let theme_multiplier = sponsorship_theme_multiplier(&sponsorship.sponsor_name);
    let base_income = (sponsorship.base_value as f64 * theme_multiplier).round() as i64;
    base_income + evaluate_sponsorship_bonus(current_position, recent_form, sponsorship)
}

pub fn evaluate_sponsorship_bonus(
    current_position: Option<u32>,
    recent_form: &[String],
    sponsorship: &Sponsorship,
) -> i64 {
    sponsorship
        .bonus_criteria
        .iter()
        .map(|criterion| match criterion {
            SponsorshipBonusCriterion::LeaguePosition {
                max_position,
                bonus_amount,
            } => {
                if current_position.is_some_and(|position| position <= *max_position) {
                    *bonus_amount
                } else {
                    0
                }
            }
            SponsorshipBonusCriterion::UnbeatenRun {
                required_matches,
                bonus_amount,
            } => {
                if recent_form.len() >= *required_matches
                    && recent_form
                        .iter()
                        .rev()
                        .take(*required_matches)
                        .all(|result| result != "L")
                {
                    *bonus_amount
                } else {
                    0
                }
            }
        })
        .sum()
}

fn current_league_position(game: &Game, team_id: &str) -> Option<u32> {
    let league = game.league.as_ref()?;

    league
        .sorted_standings()
        .iter()
        .position(|standing| standing.team_id == team_id)
        .map(|index| index as u32 + 1)
}

fn count_recent_home_matches(game: &Game, team_id: &str) -> i64 {
    let Some(league) = &game.league else {
        return 0;
    };

    let current = game.clock.current_date.date_naive();
    let week_ago = current - chrono::Duration::days(7);

    league
        .fixtures
        .iter()
        .filter(|fixture| {
            fixture.status == domain::league::FixtureStatus::Completed
                && fixture.home_team_id == team_id
                && fixture.result.is_some()
        })
        .filter(|fixture| {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d") {
                date > week_ago && date <= current
            } else {
                false
            }
        })
        .count() as i64
}

fn should_apply_monthly_upkeep(game: &Game) -> bool {
    let date = game.clock.current_date.date_naive();
    game.clock.current_date.weekday().num_days_from_monday() == 0 && date.day() <= 7
}

/// Process weekly financial operations (called every Monday = weekday 0).
/// - Deduct player wages (weekly = annual / 52)
/// - Deduct staff wages
/// - Add matchday revenue for home matches played that week
/// - Check financial health and generate warnings
pub fn process_weekly_finances(game: &mut Game) {
    let weekday = game.clock.current_date.weekday().num_days_from_monday();
    if weekday != 0 {
        return; // Only process on Mondays
    }

    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let team_expenses: Vec<(String, i64)> = game
        .teams
        .iter()
        .map(|team| {
            let wages = calc_wages(game, &team.id);
            let upkeep = if should_apply_monthly_upkeep(game) {
                calc_upkeep(team)
            } else {
                0
            };

            (team.id.clone(), wages + upkeep)
        })
        .collect();
    let team_positions: Vec<(String, Option<u32>)> = game
        .teams
        .iter()
        .map(|team| (team.id.clone(), current_league_position(game, &team.id)))
        .collect();

    for team in game.teams.iter_mut() {
        let total_expenses = team_expenses
            .iter()
            .find(|(team_id, _)| team_id == &team.id)
            .map(|(_, total)| *total)
            .unwrap_or(0);

        team.finance -= total_expenses;
        team.season_expenses += total_expenses;

        let current_position = team_positions
            .iter()
            .find(|(team_id, _)| team_id == &team.id)
            .and_then(|(_, position)| *position);

        let sponsorship_income = team
            .sponsorship
            .as_ref()
            .map(|sponsorship| calc_sponsorship_income(current_position, &team.form, sponsorship))
            .unwrap_or(0);

        if sponsorship_income > 0 {
            team.finance += sponsorship_income;
            team.season_income += sponsorship_income;
        }

        if let Some(sponsorship) = team.sponsorship.as_mut() {
            sponsorship.remaining_weeks = sponsorship.remaining_weeks.saturating_sub(1);
            if sponsorship.remaining_weeks == 0 {
                team.sponsorship = None;
            }
        }
    }

    // --- Matchday income for home matches completed in last 7 days ---
    if game.league.is_some() {
        let home_match_counts: Vec<(String, i64)> = game
            .teams
            .iter()
            .map(|team| (team.id.clone(), count_recent_home_matches(game, &team.id)))
            .collect();

        for team in game.teams.iter_mut() {
            let home_count = home_match_counts
                .iter()
                .find(|(team_id, _)| team_id == &team.id)
                .map(|(_, count)| *count)
                .unwrap_or(0);

            if home_count > 0 {
                let mut rng = rand::rng();
                let attendance_pct = rng.random_range(60..=92) as f64 / 100.0;
                let avg_ticket = rng.random_range(15..=25) as f64;
                let total_revenue = calc_matchday(
                    team.stadium_capacity,
                    home_count,
                    attendance_pct,
                    avg_ticket,
                );

                team.finance += total_revenue;
                team.season_income += total_revenue;
            }
        }
    }

    // --- Financial health warnings for user's team ---
    generate_financial_warnings(game, &today);
}

/// Generate inbox messages warning about financial issues.
fn generate_financial_warnings(game: &mut Game, today: &str) {
    let user_team_id = match &game.manager.team_id {
        Some(id) => id.clone(),
        None => return,
    };

    let team = match game.teams.iter().find(|t| t.id == user_team_id) {
        Some(t) => t,
        None => return,
    };

    let existing_ids: std::collections::HashSet<String> =
        game.messages.iter().map(|m| m.id.clone()).collect();

    let mut new_messages: Vec<InboxMessage> = Vec::new();

    let weekly_wages = calc_wages(game, &user_team_id);
    let annual_wages = calc_annual_wages(game, &user_team_id);
    let current_position = current_league_position(game, &user_team_id);
    let weekly_sponsorship_income = team
        .sponsorship
        .as_ref()
        .map(|s| calc_sponsorship_income(current_position, &team.form, s))
        .unwrap_or(0);
    let projected_weekly_net = weekly_sponsorship_income - weekly_wages;
    let weeks_left = calc_cash_runway_weeks(team.finance, projected_weekly_net).unwrap_or(999);

    // Critical: finances negative
    if team.finance < 0 {
        let msg_id = format!("finance_critical_{}", today);
        if !existing_ids.contains(&msg_id) {
            new_messages.push(
                InboxMessage::new(
                    msg_id,
                    "URGENT: Club in Debt".to_string(),
                    format!(
                        "The club is currently €{} in debt. This is an unsustainable situation.\n\n\
                        The board demands immediate action to address the financial crisis. \
                        Consider selling players, reducing staff, or finding alternative income.\n\n\
                        Failure to resolve this may have serious consequences for your position.",
                        format_money((-team.finance) as u64)
                    ),
                    "Board of Directors".to_string(),
                    today.to_string(),
                )
                .with_category(MessageCategory::Finance)
                .with_priority(MessagePriority::Urgent)
                .with_sender_role("Chairman")
                .with_i18n(
                    "be.msg.financeCritical.subject",
                    "be.msg.financeCritical.body",
                    {
                        let mut p = std::collections::HashMap::new();
                        p.insert("amount".to_string(), format_money((-team.finance) as u64));
                        p
                    },
                )
                .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman")
                .with_action(action("view_finances", "View Finances", "be.msg.event.ack",
                    ActionType::NavigateTo { route: "/dashboard?tab=Finances".to_string() }))
            );
        }
    }
    // Warning: less than 4 weeks of runway
    else if (0..4).contains(&weeks_left) {
        let msg_id = format!("finance_warning_{}", today);
        if !existing_ids.contains(&msg_id) {
            new_messages.push(
                InboxMessage::new(
                    msg_id,
                    "Financial Warning — Low Reserves".to_string(),
                    format!(
                        "Our financial reserves are running low. At the current burn rate (€{}/week in wages), \
                        we have approximately {} weeks of funding remaining.\n\n\
                        I'd recommend reviewing the wage bill and exploring ways to boost income.",
                        format_money(weekly_wages as u64), weeks_left
                    ),
                    "Financial Director".to_string(),
                    today.to_string(),
                )
                .with_category(MessageCategory::Finance)
                .with_priority(MessagePriority::High)
                .with_sender_role("Financial Director")
                .with_i18n(
                    "be.msg.financeWarning.subject",
                    "be.msg.financeWarning.body",
                    {
                        let mut p = std::collections::HashMap::new();
                        p.insert("weeklyWages".to_string(), format_money(weekly_wages as u64));
                        p.insert("weeksLeft".to_string(), weeks_left.to_string());
                        p
                    },
                )
                .with_sender_i18n("be.sender.financialDirector", "be.role.financialDirector")
                .with_action(action("view_finances", "View Finances", "be.msg.event.ack",
                    ActionType::NavigateTo { route: "/dashboard?tab=Finances".to_string() }))
            );
        }
    }
    // Over budget warning: wages exceed budget
    else if annual_wages > team.wage_budget {
        let msg_id = format!("wage_over_budget_{}", today);
        if !existing_ids.contains(&msg_id) {
            new_messages.push(
                InboxMessage::new(
                    msg_id,
                    "Wage Bill Exceeds Budget".to_string(),
                    format!(
                        "Our annual wage bill (€{}) currently exceeds the allocated wage budget (€{}).\n\n\
                        While we can sustain this in the short term, the board would prefer \
                        to see the wage bill brought under control.",
                        format_money(annual_wages as u64),
                        format_money(team.wage_budget as u64)
                    ),
                    "Financial Director".to_string(),
                    today.to_string(),
                )
                .with_category(MessageCategory::Finance)
                .with_priority(MessagePriority::Normal)
                .with_sender_role("Financial Director")
                .with_i18n(
                    "be.msg.wageOverBudget.subject",
                    "be.msg.wageOverBudget.body",
                    {
                        let mut p = std::collections::HashMap::new();
                        p.insert("annualWages".to_string(), format_money(annual_wages as u64));
                        p.insert("wageBudget".to_string(), format_money(team.wage_budget as u64));
                        p
                    },
                )
                .with_sender_i18n("be.sender.financialDirector", "be.role.financialDirector")
                .with_action(action("view_finances", "View Finances", "be.msg.event.ack",
                    ActionType::NavigateTo { route: "/dashboard?tab=Finances".to_string() }))
            );
        }
    }

    game.messages.extend(new_messages);
}

fn format_money(amount: u64) -> String {
    if amount >= 1_000_000 {
        format!("{:.1}M", amount as f64 / 1_000_000.0)
    } else if amount >= 1_000 {
        format!("{}K", amount / 1_000)
    } else {
        amount.to_string()
    }
}
