use crate::finances::calc_annual_wages;
use crate::game::Game;
use chrono::{Datelike, NaiveDate};
use domain::negotiation::{NegotiationFeedback, NegotiationMood};
use domain::player::Position;
use domain::player::TransferOfferStatus;
use domain::season::TransferWindowStatus;
use domain::team::TeamKind;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

const TRANSFER_NEGOTIATION_STALE_DAYS: i64 = 14;
const PLAYER_INCOMING_OFFER_COOLDOWN_DAYS: i64 = 7;
const TRANSFER_BUDGET_SELLING_REALLOCATION_PCT: i64 = 60;
const CONTRACT_RELEASE_PENALTY_PCT: i64 = 40;
const MAX_INCOMING_OFFERS_PER_DAY: usize = 1;
const MAX_AI_FREE_AGENT_SIGNINGS_PER_DAY: usize = 2;
const MAX_AI_INTERCLUB_TRANSFERS_PER_DAY: usize = 1;
const LOL_CORE_ROLES: [&str; 5] = ["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferNegotiationDecision {
    Accepted,
    Rejected,
    CounterOffer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferNegotiationOutcome {
    pub decision: TransferNegotiationDecision,
    pub suggested_fee: Option<u64>,
    pub is_terminal: bool,
    pub feedback: NegotiationFeedback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferBidFinancialProjection {
    pub transfer_budget_before: i64,
    pub transfer_budget_after: i64,
    pub finance_before: i64,
    pub finance_after: i64,
    pub annual_wage_bill_before: i64,
    pub annual_wage_bill_after: i64,
    pub annual_wage_budget: i64,
    pub projected_wage_budget_usage_pct: i64,
    pub exceeds_transfer_budget: bool,
    pub exceeds_finance: bool,
}

enum PlayerImportance {
    Key,
    Regular,
    Fringe,
}

fn contract_days_remaining(current_date: NaiveDate, contract_end: Option<&str>) -> Option<i64> {
    let contract_end = contract_end?;
    let contract_end_date = NaiveDate::parse_from_str(contract_end, "%Y-%m-%d").ok()?;
    Some((contract_end_date - current_date).num_days())
}

fn infer_player_importance(
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
) -> PlayerImportance {
    if owner_team.starting_xi_ids.iter().any(|id| id == &player.id) {
        return PlayerImportance::Key;
    }

    if player.market_value >= 1_500_000 {
        return PlayerImportance::Regular;
    }

    PlayerImportance::Fringe
}

fn minimum_acceptable_fee(
    current_date: NaiveDate,
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
    buyer_team: &domain::team::Team,
) -> u64 {
    let mut multiplier: f64 = if player.transfer_listed { 0.8 } else { 1.2 };

    if let Some(days_remaining) =
        contract_days_remaining(current_date, player.contract_end.as_deref())
    {
        if days_remaining <= 60 {
            multiplier -= 0.25;
        } else if days_remaining <= 180 {
            multiplier -= 0.15;
        } else if days_remaining <= 365 {
            multiplier -= 0.05;
        }
    }

    match infer_player_importance(player, owner_team) {
        PlayerImportance::Key => multiplier += 0.2,
        PlayerImportance::Regular => multiplier += 0.1,
        PlayerImportance::Fringe => {}
    }

    if player.morale <= 40 {
        multiplier -= 0.05;
    }

    let openness_score = player_move_openness_score(current_date, player, owner_team, buyer_team);
    if openness_score >= 60 {
        multiplier -= 0.20;
    } else if openness_score >= 40 {
        multiplier -= 0.10;
    }

    let multiplier = multiplier.clamp(0.55, 1.6);
    ((player.market_value as f64) * multiplier).round() as u64
}

fn player_move_openness_score(
    current_date: NaiveDate,
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
    buyer_team: &domain::team::Team,
) -> i32 {
    let mut score = 0;

    if player.morale <= 45 {
        score += 20;
    } else if player.morale <= 60 {
        score += 10;
    }

    if player.stats.appearances <= 2 {
        score += 15;
    } else if player.stats.appearances <= 5 {
        score += 8;
    }

    if let Some(days_remaining) =
        contract_days_remaining(current_date, player.contract_end.as_deref())
    {
        if days_remaining <= 180 {
            score += 20;
        } else if days_remaining <= 365 {
            score += 10;
        }
    }

    let reputation_gap = buyer_team.reputation as i32 - owner_team.reputation as i32;
    if reputation_gap >= 200 {
        score += 25;
    } else if reputation_gap >= 75 {
        score += 15;
    }

    if player.transfer_listed {
        score += 10;
    }

    score
}

fn apply_blocked_move_consequences(player: &mut domain::player::Player, openness_score: i32) {
    if openness_score < 40 {
        return;
    }

    let morale_drop = if openness_score >= 60 { 10 } else { 6 };
    player.morale = (i16::from(player.morale) - morale_drop).clamp(0, 100) as u8;
    player.morale_core.manager_trust =
        (i16::from(player.morale_core.manager_trust) - 5).clamp(0, 100) as u8;
    player.morale_core.unresolved_issue = Some(domain::player::PlayerIssue {
        category: domain::player::PlayerIssueCategory::Contract,
        severity: if openness_score >= 60 { 75 } else { 60 },
    });
}

fn incoming_interest_score(current_date: NaiveDate, player: &domain::player::Player) -> i32 {
    let mut score = 8;

    if player.transfer_listed {
        score += 30;
    }

    if let Some(days_remaining) =
        contract_days_remaining(current_date, player.contract_end.as_deref())
    {
        if days_remaining <= 60 {
            score += 40;
        } else if days_remaining <= 180 {
            score += 25;
        } else if days_remaining <= 365 {
            score += 10;
        }
    }

    if player.market_value >= 1_000_000 {
        score += 20;
    } else if player.market_value >= 500_000 {
        score += 10;
    }

    if player.morale <= 45 {
        score += 10;
    }

    score
}

fn suggested_incoming_fee(
    current_date: NaiveDate,
    player: &domain::player::Player,
    buyer_team: &domain::team::Team,
    buyer_id: &str,
) -> u64 {
    let mut multiplier: f64 = if player.transfer_listed { 0.75 } else { 0.8 };

    if let Some(days_remaining) =
        contract_days_remaining(current_date, player.contract_end.as_deref())
    {
        if days_remaining <= 60 {
            multiplier -= 0.15;
        } else if days_remaining <= 180 {
            multiplier -= 0.1;
        }
    }

    if player.morale <= 45 {
        multiplier -= 0.05;
    }

    // Stronger clubs and richer clubs tend to bid more aggressively.
    if buyer_team.reputation >= 1300 {
        multiplier += 0.06;
    } else if buyer_team.reputation >= 1100 {
        multiplier += 0.03;
    }

    if buyer_team.transfer_budget >= 4_000_000 {
        multiplier += 0.04;
    } else if buyer_team.transfer_budget >= 2_000_000 {
        multiplier += 0.02;
    }

    // Deterministic per club+player+day jitter so offers are not all identical.
    let mut hasher = DefaultHasher::new();
    player.id.hash(&mut hasher);
    buyer_id.hash(&mut hasher);
    current_date.num_days_from_ce().hash(&mut hasher);
    let bucket = (hasher.finish() % 17) as i32; // 0..16
    let jitter = (bucket - 8) as f64 * 0.01; // -8% .. +8%
    multiplier += jitter;

    let multiplier = multiplier.clamp(0.42, 1.0);
    ((player.market_value as f64) * multiplier).round() as u64
}

fn has_open_incoming_offer_from_club(player: &domain::player::Player, club_id: &str) -> bool {
    player
        .transfer_offers
        .iter()
        .any(|offer| offer.from_team_id == club_id && offer.status == TransferOfferStatus::Pending)
}

fn offer_is_stale(current_date: NaiveDate, offer: &domain::player::TransferOffer) -> bool {
    if offer.status != TransferOfferStatus::Pending {
        return false;
    }

    let Ok(offer_date) = NaiveDate::parse_from_str(&offer.date, "%Y-%m-%d") else {
        return false;
    };

    (current_date - offer_date).num_days() >= TRANSFER_NEGOTIATION_STALE_DAYS
}

fn has_recent_incoming_offer(
    current_date: NaiveDate,
    player: &domain::player::Player,
    cooldown_days: i64,
) -> bool {
    player.transfer_offers.iter().any(|offer| {
        let Ok(offer_date) = NaiveDate::parse_from_str(&offer.date, "%Y-%m-%d") else {
            return false;
        };
        let age_days = (current_date - offer_date).num_days();
        age_days >= 0 && age_days < cooldown_days
    })
}

fn allow_unsolicited_offer_for_player(
    current_date: NaiveDate,
    player: &domain::player::Player,
    owner_team: Option<&domain::team::Team>,
) -> bool {
    if player.transfer_listed {
        return true;
    }

    if has_recent_incoming_offer(current_date, player, PLAYER_INCOMING_OFFER_COOLDOWN_DAYS) {
        return false;
    }

    if let Some(team) = owner_team {
        let is_key_player = team.starting_xi_ids.iter().any(|id| id == &player.id);
        if is_key_player {
            return false;
        }
    }

    let low_morale = player.morale <= 45;
    let low_minutes = player.stats.minutes_played <= 180;
    let contract_short = contract_days_remaining(current_date, player.contract_end.as_deref())
        .map(|days| days <= 365)
        .unwrap_or(false);

    low_morale || low_minutes || contract_short
}

fn expire_stale_transfer_offers(game: &mut Game) {
    let current_date = game.clock.current_date.date_naive();

    for player in &mut game.players {
        for offer in &mut player.transfer_offers {
            if offer_is_stale(current_date, offer) {
                offer.status = TransferOfferStatus::Withdrawn;
                offer.suggested_counter_fee = None;
            }
        }
    }
}

fn find_open_offer_from_club<'a>(
    player: &'a domain::player::Player,
    club_id: &str,
) -> Option<&'a domain::player::TransferOffer> {
    player
        .transfer_offers
        .iter()
        .find(|offer| offer.from_team_id == club_id && offer.status == TransferOfferStatus::Pending)
}

fn negotiation_round_from_offer(offer: Option<&domain::player::TransferOffer>) -> u8 {
    offer
        .map(|offer| offer.negotiation_round.max(1).saturating_add(1))
        .unwrap_or(1)
}

fn transfer_negotiation_metrics(round: u8, stalled: bool, respected_signal: bool) -> (u8, u8) {
    let mut tension = 34_i16 + (i16::from(round.saturating_sub(1)) * 16);
    let mut patience = 82_i16 - (i16::from(round.saturating_sub(1)) * 18);

    if stalled {
        tension += 12;
        patience -= 12;
    }

    if respected_signal {
        tension -= 8;
        patience += 8;
    }

    (tension.clamp(20, 90) as u8, patience.clamp(18, 86) as u8)
}

fn upsert_transfer_offer(
    player: &mut domain::player::Player,
    from_team_id: &str,
    fee: u64,
    status: TransferOfferStatus,
    date: &str,
    last_manager_fee: Option<u64>,
    negotiation_round: u8,
    suggested_counter_fee: Option<u64>,
) -> String {
    if let Some(offer) = player.transfer_offers.iter_mut().find(|offer| {
        offer.from_team_id == from_team_id && offer.status == TransferOfferStatus::Pending
    }) {
        offer.fee = fee;
        offer.status = status;
        offer.date = date.to_string();
        offer.last_manager_fee = last_manager_fee;
        offer.negotiation_round = negotiation_round;
        offer.suggested_counter_fee = suggested_counter_fee;
        return offer.id.clone();
    }

    let offer_id = Uuid::new_v4().to_string();
    player.transfer_offers.push(domain::player::TransferOffer {
        id: offer_id.clone(),
        from_team_id: from_team_id.to_string(),
        fee,
        wage_offered: 0,
        last_manager_fee,
        negotiation_round,
        suggested_counter_fee,
        status,
        date: date.to_string(),
    });
    offer_id
}

fn transfer_window_is_open(game: &Game) -> bool {
    matches!(
        game.season_context.transfer_window.status,
        TransferWindowStatus::Open | TransferWindowStatus::DeadlineDay
    )
}

pub fn generate_incoming_transfer_offers(game: &mut Game) {
    expire_stale_transfer_offers(game);

    if !transfer_window_is_open(game) {
        return;
    }

    let Some(user_team_id) = game.manager.team_id.clone() else {
        return;
    };

    let current_date = game.clock.current_date.date_naive();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let academy_offer_window_open = matches!(
        current_date.weekday(),
        chrono::Weekday::Mon | chrono::Weekday::Wed | chrono::Weekday::Fri
    );

    let mut managed_team_ids = std::collections::HashSet::new();
    managed_team_ids.insert(user_team_id.clone());
    for team in &game.teams {
        if team.team_kind == TeamKind::Academy
            && team.parent_team_id.as_deref() == Some(&user_team_id)
        {
            managed_team_ids.insert(team.id.clone());
        }
    }

    let buyer_ids: Vec<String> = game
        .teams
        .iter()
        .filter(|team| team.id != user_team_id && team.team_kind == TeamKind::Main)
        .map(|team| team.id.clone())
        .collect();

    let existing_offers_today = game
        .players
        .iter()
        .filter(|player| {
            player
                .team_id
                .as_deref()
                .map(|team_id| managed_team_ids.contains(team_id))
                .unwrap_or(false)
        })
        .flat_map(|player| player.transfer_offers.iter())
        .filter(|offer| offer.status == TransferOfferStatus::Pending && offer.date == today)
        .count();

    if existing_offers_today >= MAX_INCOMING_OFFERS_PER_DAY {
        simulate_ai_free_agent_signings(game, &user_team_id);
        simulate_ai_club_to_club_transfers(game, &user_team_id);
        return;
    }

    let mut created_offers = existing_offers_today;
    let mut academy_offer_created_today = false;
    for buyer_id in buyer_ids {
        if created_offers >= MAX_INCOMING_OFFERS_PER_DAY {
            break;
        }

        let Some(buyer_team) = game.teams.iter().find(|team| team.id == buyer_id) else {
            continue;
        };

        let mut chosen_player_id: Option<String> = None;
        let mut chosen_score = i32::MIN;
        let mut chosen_fee = 0_u64;
        let mut chosen_is_academy = false;

        let mut academy_player_id: Option<String> = None;
        let mut academy_score = i32::MIN;
        let mut academy_fee = 0_u64;

        for player in &game.players {
            let Some(player_team_id) = player.team_id.as_deref() else {
                continue;
            };

            if !managed_team_ids.contains(player_team_id) {
                continue;
            }

            let player_team = game.teams.iter().find(|team| team.id == player_team_id);
            let is_academy_player = player_team
                .map(|team| team.team_kind == TeamKind::Academy)
                .unwrap_or(false);

            if !allow_unsolicited_offer_for_player(current_date, player, player_team) {
                continue;
            }

            if is_academy_player {
                if !academy_offer_window_open {
                    continue;
                }

                if player_team.and_then(|team| team.parent_team_id.as_deref())
                    == Some(buyer_id.as_str())
                {
                    continue;
                }
            }

            if has_open_incoming_offer_from_club(player, &buyer_id) {
                continue;
            }

            let score = incoming_interest_score(current_date, player);
            let minimum_score = if is_academy_player { 12 } else { 8 };
            if score < minimum_score {
                continue;
            }

            let fee = suggested_incoming_fee(current_date, player, buyer_team, &buyer_id);
            if buyer_team.transfer_budget < fee as i64 || buyer_team.finance < fee as i64 {
                continue;
            }

            if score > chosen_score {
                chosen_player_id = Some(player.id.clone());
                chosen_score = score;
                chosen_fee = fee;
                chosen_is_academy = is_academy_player;
            }

            if is_academy_player && score > academy_score {
                academy_player_id = Some(player.id.clone());
                academy_score = score;
                academy_fee = fee;
            }
        }

        if academy_offer_window_open && !academy_offer_created_today {
            if let Some(academy_id) = academy_player_id {
                if !chosen_is_academy {
                    chosen_player_id = Some(academy_id);
                    chosen_fee = academy_fee;
                    chosen_is_academy = true;
                }
            }
        }

        let Some(player_id) = chosen_player_id else {
            continue;
        };

        let Some(player) = game
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
        else {
            continue;
        };

        let offer_id = Uuid::new_v4().to_string();

        player.transfer_offers.push(domain::player::TransferOffer {
            id: offer_id.clone(),
            from_team_id: buyer_id.clone(),
            fee: chosen_fee,
            wage_offered: 0,
            last_manager_fee: None,
            negotiation_round: 1,
            suggested_counter_fee: None,
            status: TransferOfferStatus::Pending,
            date: today.clone(),
        });

        let player_name = player.match_name.clone();
        let buyer_name = buyer_team.name.clone();
        let message = crate::messages::incoming_transfer_offer_message(
            &offer_id,
            &player_id,
            &player_name,
            &buyer_name,
            chosen_fee,
            &today,
        );
        game.messages.push(message);
        created_offers = created_offers.saturating_add(1);
        if chosen_is_academy {
            academy_offer_created_today = true;
        }
    }

    simulate_ai_free_agent_signings(game, &user_team_id);
    simulate_ai_club_to_club_transfers(game, &user_team_id);
}

fn simulate_ai_free_agent_signings(game: &mut Game, user_team_id: &str) {
    let mut candidate_team_ids: Vec<String> = game
        .teams
        .iter()
        .filter(|team| team.id != user_team_id && team.team_kind == TeamKind::Main)
        .filter(|team| team.transfer_budget > 0 && team.finance > 0)
        .map(|team| team.id.clone())
        .collect();

    candidate_team_ids.sort_by(|left, right| {
        let left_budget = game
            .teams
            .iter()
            .find(|team| team.id == *left)
            .map(|team| team.transfer_budget)
            .unwrap_or(0);
        let right_budget = game
            .teams
            .iter()
            .find(|team| team.id == *right)
            .map(|team| team.transfer_budget)
            .unwrap_or(0);
        right_budget.cmp(&left_budget)
    });

    let mut completed = 0_usize;

    for team_id in candidate_team_ids {
        if completed >= MAX_AI_FREE_AGENT_SIGNINGS_PER_DAY {
            break;
        }

        let roster_size = game
            .players
            .iter()
            .filter(|player| player.team_id.as_deref() == Some(team_id.as_str()))
            .count();
        if roster_size >= 7 {
            continue;
        }

        let Some(team) = game.teams.iter().find(|team| team.id == team_id) else {
            continue;
        };
        let budget_cap = team.transfer_budget.min(team.finance);
        if budget_cap <= 25_000 {
            continue;
        }

        let preferred_role = ai_team_priority_role(game, &team_id);

        let role_candidate = game
            .players
            .iter()
            .filter(|player| player.team_id.is_none())
            .filter(|player| lol_role_for_position(&player.natural_position) == preferred_role)
            .filter_map(|player| {
                let asking_price = (player.market_value as i64).max(25_000) / 5;
                (asking_price > 0 && asking_price <= budget_cap).then_some((
                    player.id.clone(),
                    asking_price as u64,
                    player.market_value,
                ))
            })
            .max_by_key(|(_, _, market_value)| *market_value);

        let candidate = game
            .players
            .iter()
            .filter(|player| player.team_id.is_none())
            .filter_map(|player| {
                let asking_price = (player.market_value as i64).max(25_000) / 5;
                (asking_price > 0 && asking_price <= budget_cap).then_some((
                    player.id.clone(),
                    asking_price as u64,
                    player.market_value,
                ))
            })
            .max_by_key(|(_, _, market_value)| *market_value);

        let Some((player_id, fee, _)) = role_candidate.or(candidate) else {
            continue;
        };

        if execute_free_agent_signing(game, &player_id, &team_id, fee).is_ok() {
            completed = completed.saturating_add(1);
        }
    }
}

fn simulate_ai_club_to_club_transfers(game: &mut Game, user_team_id: &str) {
    let current_date = game.clock.current_date.date_naive();

    let mut completed = 0_usize;
    let buyer_ids: Vec<String> = game
        .teams
        .iter()
        .filter(|team| team.id != user_team_id && team.team_kind == TeamKind::Main)
        .filter(|team| team.transfer_budget > 0 && team.finance > 0)
        .map(|team| team.id.clone())
        .collect();

    for buyer_id in buyer_ids {
        if completed >= MAX_AI_INTERCLUB_TRANSFERS_PER_DAY {
            break;
        }

        let Some(buyer_team) = game.teams.iter().find(|team| team.id == buyer_id) else {
            continue;
        };
        let budget_cap = buyer_team.transfer_budget.min(buyer_team.finance);
        if budget_cap <= 100_000 {
            continue;
        }

        let preferred_role = ai_team_priority_role(game, &buyer_id);

        let role_candidate = game
            .players
            .iter()
            .filter_map(|player| {
                if lol_role_for_position(&player.natural_position) != preferred_role {
                    return None;
                }

                let seller_id = player.team_id.as_deref()?;
                if seller_id == user_team_id || seller_id == buyer_id {
                    return None;
                }

                let seller_team = game.teams.iter().find(|team| team.id == seller_id)?;
                if seller_team.team_kind != TeamKind::Main {
                    return None;
                }

                let fee = suggested_incoming_fee(current_date, player, buyer_team, &buyer_id);
                if fee == 0 || (fee as i64) > budget_cap {
                    return None;
                }

                let attractiveness = incoming_interest_score(current_date, player)
                    + if player.transfer_listed { 20 } else { 0 }
                    + if player.market_value >= 1_000_000 {
                        10
                    } else {
                        0
                    };

                (attractiveness >= 35).then_some((
                    player.id.clone(),
                    seller_id.to_string(),
                    fee,
                    attractiveness,
                ))
            })
            .max_by_key(|(_, _, _, attractiveness)| *attractiveness);

        let candidate = game
            .players
            .iter()
            .filter_map(|player| {
                let seller_id = player.team_id.as_deref()?;
                if seller_id == user_team_id || seller_id == buyer_id {
                    return None;
                }

                let seller_team = game.teams.iter().find(|team| team.id == seller_id)?;
                if seller_team.team_kind != TeamKind::Main {
                    return None;
                }

                let fee = suggested_incoming_fee(current_date, player, buyer_team, &buyer_id);
                if fee == 0 || (fee as i64) > budget_cap {
                    return None;
                }

                let attractiveness = incoming_interest_score(current_date, player)
                    + if player.transfer_listed { 20 } else { 0 }
                    + if player.market_value >= 1_000_000 {
                        10
                    } else {
                        0
                    };

                (attractiveness >= 35).then_some((
                    player.id.clone(),
                    seller_id.to_string(),
                    fee,
                    attractiveness,
                ))
            })
            .max_by_key(|(_, _, _, attractiveness)| *attractiveness);

        let Some((player_id, seller_id, fee, _)) = role_candidate.or(candidate) else {
            continue;
        };

        if execute_transfer(game, &player_id, &buyer_id, &seller_id, fee).is_ok() {
            completed = completed.saturating_add(1);
        }
    }
}

fn ai_team_priority_role(game: &Game, team_id: &str) -> &'static str {
    let mut role_counts = [0_usize; 5];

    for player in &game.players {
        if player.team_id.as_deref() != Some(team_id) {
            continue;
        }

        let role = lol_role_for_position(&player.natural_position);
        if let Some(index) = LOL_CORE_ROLES
            .iter()
            .position(|candidate| *candidate == role)
        {
            role_counts[index] = role_counts[index].saturating_add(1);
        }
    }

    let mut best_index = 0_usize;
    let mut best_count = role_counts[0];

    for (index, count) in role_counts.iter().enumerate().skip(1) {
        if *count < best_count {
            best_count = *count;
            best_index = index;
        }
    }

    LOL_CORE_ROLES[best_index]
}

fn buyer_counter_offer_ceiling(
    current_date: NaiveDate,
    player: &domain::player::Player,
    current_offer_fee: u64,
    buyer_team: &domain::team::Team,
) -> u64 {
    let baseline_fee =
        suggested_incoming_fee(current_date, player, buyer_team, &buyer_team.id)
            .max(current_offer_fee);
    let ceiling = ((baseline_fee as f64) * 1.2).round() as u64;
    ceiling
        .min(buyer_team.transfer_budget.max(0) as u64)
        .min(buyer_team.finance.max(0) as u64)
}

fn should_generate_major_transfer_news(player: &domain::player::Player, fee: u64) -> bool {
    fee >= 1_000_000 || player.market_value >= 1_000_000
}

fn transfer_outcome(
    decision: TransferNegotiationDecision,
    suggested_fee: Option<u64>,
    is_terminal: bool,
    feedback: NegotiationFeedback,
) -> TransferNegotiationOutcome {
    TransferNegotiationOutcome {
        decision,
        suggested_fee,
        is_terminal,
        feedback,
    }
}

pub fn project_transfer_bid_financial_impact(
    game: &Game,
    player_id: &str,
    fee: u64,
) -> Result<TransferBidFinancialProjection, String> {
    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or_else(|| "No user team".to_string())?;

    let player = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .ok_or_else(|| "Player not found".to_string())?;

    if player.team_id.as_deref() == Some(user_team_id.as_str()) {
        return Err("Cannot bid on your own player".to_string());
    }

    let team = game
        .teams
        .iter()
        .find(|team| team.id == user_team_id)
        .ok_or_else(|| "User team not found".to_string())?;

    let annual_wage_bill_before = calc_annual_wages(game, &team.id);
    let annual_wage_bill_after = annual_wage_bill_before + player.wage as i64;
    let projected_wage_budget_usage_pct = if team.wage_budget > 0 {
        ((annual_wage_bill_after as f64 / team.wage_budget as f64) * 100.0).round() as i64
    } else {
        0
    };

    let transfer_budget_after = team.transfer_budget - fee as i64;
    let finance_after = team.finance - fee as i64;

    Ok(TransferBidFinancialProjection {
        transfer_budget_before: team.transfer_budget,
        transfer_budget_after,
        finance_before: team.finance,
        finance_after,
        annual_wage_bill_before,
        annual_wage_bill_after,
        annual_wage_budget: team.wage_budget,
        projected_wage_budget_usage_pct,
        exceeds_transfer_budget: transfer_budget_after < 0,
        exceeds_finance: finance_after < 0,
    })
}

/// Submit a transfer bid from user's team for a player.
/// The AI evaluates the bid and can accept, reject, or counter based on club context.
pub fn make_transfer_bid(
    game: &mut Game,
    player_id: &str,
    fee: u64,
) -> Result<TransferNegotiationOutcome, String> {
    expire_stale_transfer_offers(game);

    if !transfer_window_is_open(game) {
        return Err("Transfer window is closed".into());
    }

    let user_team_id = game.manager.team_id.clone().ok_or("No user team")?;

    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .ok_or("Player not found")?;

    if player.team_id.as_deref() == Some(&user_team_id) {
        return Err("Cannot bid on your own player".into());
    }

    let my_team = game
        .teams
        .iter()
        .find(|t| t.id == user_team_id)
        .ok_or("User team not found")?;

    if (my_team.finance as u64) < fee {
        return Err("Insufficient funds".into());
    }

    if my_team.transfer_budget < fee as i64 {
        return Err("Transfer budget too low".into());
    }

    let date = game.clock.current_date.format("%Y-%m-%d").to_string();

    if player.team_id.is_none() {
        if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
            upsert_transfer_offer(
                p,
                &user_team_id,
                fee,
                TransferOfferStatus::Accepted,
                &date,
                Some(fee),
                1,
                None,
            );
        }

        execute_free_agent_signing(game, player_id, &user_team_id, fee)?;

        let player_name = game
            .players
            .iter()
            .find(|p| p.id == player_id)
            .map(|p| p.match_name.clone())
            .unwrap_or_default();

        let msg = crate::messages::transfer_complete_message(&player_name, fee, &date);
        game.messages.push(msg);

        return Ok(transfer_outcome(
            TransferNegotiationDecision::Accepted,
            None,
            true,
            build_transfer_feedback(
                "transfers.transferFeedbackAcceptedHeadline",
                "transfers.transferFeedbackAcceptedDetail",
                NegotiationMood::Positive,
                24,
                84,
                1,
                &[("fee", fee.to_string())],
            ),
        ));
    }

    let owner_team_id = player.team_id.clone().ok_or("Player has no team")?;

    let owner_team = game
        .teams
        .iter()
        .find(|t| t.id == owner_team_id)
        .ok_or("Owner team not found")?;

    let buyer_team = my_team;

    let current_date = game.clock.current_date.date_naive();

    let threshold = minimum_acceptable_fee(current_date, player, owner_team, buyer_team);
    let existing_offer = find_open_offer_from_club(player, &user_team_id);
    let previous_fee = existing_offer.map(|offer| offer.fee);
    let previous_counter_fee = existing_offer.and_then(|offer| offer.suggested_counter_fee);
    let round = negotiation_round_from_offer(existing_offer);
    let respected_signal = previous_counter_fee
        .map(|counter| fee >= counter.saturating_mul(95) / 100)
        .unwrap_or(false);
    let stalled = previous_fee
        .map(|previous| fee <= previous.saturating_add(50_000))
        .unwrap_or(false);
    let concession = if respected_signal {
        ((threshold as f64) * 0.04).round() as u64
    } else if round >= 3 && !stalled {
        ((threshold as f64) * 0.02).round() as u64
    } else {
        0
    };
    let adjusted_threshold = threshold.saturating_sub(concession);
    let counter_floor_ratio = if round >= 2 && stalled {
        0.94
    } else if round >= 3 {
        0.92
    } else {
        0.88
    };
    let counter_floor = ((adjusted_threshold as f64) * counter_floor_ratio).round() as u64;
    let openness_score = player_move_openness_score(current_date, player, owner_team, buyer_team);
    let (tension, patience) = transfer_negotiation_metrics(round, stalled, respected_signal);

    if fee >= adjusted_threshold {
        if !player_accepts_transfer(current_date, player, owner_team, buyer_team) {
            if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
                upsert_transfer_offer(
                    p,
                    &user_team_id,
                    fee,
                    TransferOfferStatus::Rejected,
                    &date,
                    Some(fee),
                    round,
                    None,
                );
            }

            return Ok(transfer_outcome(
                TransferNegotiationDecision::Rejected,
                None,
                true,
                build_transfer_feedback(
                    "transfers.transferFeedbackRejectedHeadline",
                    "transfers.transferFeedbackPlayerRejectedDetail",
                    NegotiationMood::Guarded,
                    tension.saturating_add(6).min(90),
                    patience.saturating_sub(8),
                    round,
                    &[("fee", round_transfer_fee(adjusted_threshold).to_string())],
                ),
            ));
        }

        if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
            upsert_transfer_offer(
                p,
                &user_team_id,
                fee,
                TransferOfferStatus::Accepted,
                &date,
                Some(fee),
                round,
                None,
            );
        }

        // Execute transfer
        execute_transfer(game, player_id, &user_team_id, &owner_team_id, fee)?;

        // Generate message
        let player_name = game
            .players
            .iter()
            .find(|p| p.id == player_id)
            .map(|p| p.match_name.clone())
            .unwrap_or_default();

        let msg = crate::messages::transfer_complete_message(&player_name, fee, &date);
        game.messages.push(msg);

        return Ok(transfer_outcome(
            TransferNegotiationDecision::Accepted,
            None,
            true,
            build_transfer_feedback(
                "transfers.transferFeedbackAcceptedHeadline",
                "transfers.transferFeedbackAcceptedDetail",
                NegotiationMood::Positive,
                tension.saturating_sub(8),
                patience.saturating_add(6).min(90),
                round,
                &[("fee", fee.to_string())],
            ),
        ));
    }

    if fee >= counter_floor {
        let suggested_fee = round_transfer_fee(adjusted_threshold);
        if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
            upsert_transfer_offer(
                p,
                &user_team_id,
                fee,
                TransferOfferStatus::Pending,
                &date,
                Some(fee),
                round,
                Some(suggested_fee),
            );
        }

        return Ok(transfer_outcome(
            TransferNegotiationDecision::CounterOffer,
            Some(suggested_fee),
            false,
            build_transfer_feedback(
                "transfers.transferFeedbackCounterHeadline",
                "transfers.transferFeedbackCounterDetail",
                if openness_score >= 45 {
                    NegotiationMood::Firm
                } else {
                    NegotiationMood::Tense
                },
                if openness_score >= 45 {
                    tension.saturating_sub(6)
                } else {
                    tension.saturating_add(6).min(90)
                },
                if openness_score >= 45 {
                    patience.saturating_add(4).min(90)
                } else {
                    patience.saturating_sub(4)
                },
                round,
                &[("fee", suggested_fee.to_string())],
            ),
        ));
    }

    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
        upsert_transfer_offer(
            p,
            &user_team_id,
            fee,
            TransferOfferStatus::Rejected,
            &date,
            Some(fee),
            round,
            None,
        );
    }

    Ok(transfer_outcome(
        TransferNegotiationDecision::Rejected,
        None,
        true,
        build_transfer_feedback(
            "transfers.transferFeedbackRejectedHeadline",
            "transfers.transferFeedbackRejectedDetail",
            NegotiationMood::Guarded,
            tension.saturating_add(10).min(92),
            patience.saturating_sub(14),
            round,
            &[("fee", round_transfer_fee(adjusted_threshold).to_string())],
        ),
    ))
}

fn execute_free_agent_signing(
    game: &mut Game,
    player_id: &str,
    to_team_id: &str,
    fee: u64,
) -> Result<(), String> {
    if let Some(player) = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
    {
        player.team_id = Some(to_team_id.to_string());
        player.transfer_listed = false;
        player.loan_listed = false;
        player
            .transfer_offers
            .retain(|offer| offer.status == TransferOfferStatus::Accepted);
        if player.contract_end.is_none() {
            let renewal_year = game.clock.current_date.year() + 2;
            player.contract_end = Some(format!("{}-11-30", renewal_year));
        }
    } else {
        return Err("Player not found".to_string());
    }

    if let Some(team) = game.teams.iter_mut().find(|team| team.id == to_team_id) {
        team.finance -= fee as i64;
        team.transfer_budget -= fee as i64;
        if let Some(pos) = team.starting_xi_ids.iter().position(|id| id == player_id) {
            team.starting_xi_ids.remove(pos);
        }
    }

    Ok(())
}

/// Respond to an incoming transfer offer on one of user's players.
pub fn respond_to_offer(
    game: &mut Game,
    player_id: &str,
    offer_id: &str,
    accept: bool,
) -> Result<(), String> {
    expire_stale_transfer_offers(game);

    if accept && !transfer_window_is_open(game) {
        return Err("Transfer window is closed".into());
    }

    let user_team_id = game.manager.team_id.clone().ok_or("No user team")?;

    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id && p.team_id.as_deref() == Some(&user_team_id))
        .ok_or("Player not found or not yours")?;

    let offer = player
        .transfer_offers
        .iter()
        .find(|o| o.id == offer_id && o.status == TransferOfferStatus::Pending)
        .ok_or("Offer not found or not pending")?;

    let from_team_id = offer.from_team_id.clone();
    let fee = offer.fee;
    let current_date = game.clock.current_date.date_naive();
    let (openness_score, player_accepts_move) = {
        let owner_team = game
            .teams
            .iter()
            .find(|team| team.id == user_team_id)
            .ok_or("User team not found")?;
        let buyer_team = game
            .teams
            .iter()
            .find(|team| team.id == from_team_id)
            .ok_or("Buying team not found")?;
        (
            player_move_openness_score(current_date, player, owner_team, buyer_team),
            player_accepts_transfer(current_date, player, owner_team, buyer_team),
        )
    };

    // Update offer status
    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id)
        && let Some(o) = p.transfer_offers.iter_mut().find(|o| o.id == offer_id)
    {
        o.status = if accept {
            TransferOfferStatus::Accepted
        } else {
            TransferOfferStatus::Rejected
        };
    }

    if accept {
        if player_accepts_move {
            execute_transfer(game, player_id, &from_team_id, &user_team_id, fee)?;
        } else if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id)
            && let Some(o) = p.transfer_offers.iter_mut().find(|o| o.id == offer_id)
        {
            o.status = TransferOfferStatus::Rejected;
        }
    } else if let Some(player) = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
    {
        apply_blocked_move_consequences(player, openness_score);
    }

    Ok(())
}

pub fn counter_offer(
    game: &mut Game,
    player_id: &str,
    offer_id: &str,
    requested_fee: u64,
) -> Result<TransferNegotiationOutcome, String> {
    expire_stale_transfer_offers(game);

    if !transfer_window_is_open(game) {
        return Err("Transfer window is closed".into());
    }

    let user_team_id = game.manager.team_id.clone().ok_or("No user team")?;

    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id && p.team_id.as_deref() == Some(&user_team_id))
        .ok_or("Player not found or not yours")?;

    let offer = player
        .transfer_offers
        .iter()
        .find(|offer| offer.id == offer_id && offer.status == TransferOfferStatus::Pending)
        .ok_or("Offer not found or not pending")?;
    let player_snapshot = player.clone();

    if requested_fee <= offer.fee {
        return Err("Counter offer must exceed current offer".into());
    }

    let buyer_team = game
        .teams
        .iter()
        .find(|team| team.id == offer.from_team_id)
        .ok_or("Buying team not found")?;

    let buyer_team_id = buyer_team.id.clone();
    let current_date = game.clock.current_date.date_naive();
    let round = offer.negotiation_round.max(1).saturating_add(1);
    let respected_signal = offer
        .suggested_counter_fee
        .map(|suggested| requested_fee <= suggested.saturating_add(50_000))
        .unwrap_or(false);
    let stalled = requested_fee > offer.fee.saturating_add(175_000);
    let (tension, patience) = transfer_negotiation_metrics(round, stalled, respected_signal);
    let counter_ceiling =
        buyer_counter_offer_ceiling(current_date, &player_snapshot, offer.fee, buyer_team);
    let budget_cap =
        (buyer_team.transfer_budget.max(0) as u64).min(buyer_team.finance.max(0) as u64);
    let goodwill_margin = if respected_signal { 50_000 } else { 0 };
    let accepted = requested_fee
        <= counter_ceiling
            .saturating_add(goodwill_margin)
            .min(budget_cap);
    let counter_window =
        ((counter_ceiling as f64) * if round >= 3 && stalled { 1.03 } else { 1.08 }).round() as u64;
    let date = game.clock.current_date.format("%Y-%m-%d").to_string();

    if let Some(player) = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
        && let Some(offer) = player
            .transfer_offers
            .iter_mut()
            .find(|offer| offer.id == offer_id)
    {
        if accepted {
            offer.fee = requested_fee;
            offer.status = TransferOfferStatus::Accepted;
            offer.last_manager_fee = Some(requested_fee);
            offer.negotiation_round = round;
            offer.suggested_counter_fee = None;
        } else if requested_fee > counter_window {
            offer.status = TransferOfferStatus::Rejected;
            offer.last_manager_fee = Some(requested_fee);
            offer.negotiation_round = round;
            offer.suggested_counter_fee = None;
        }
        offer.date = date.clone();
    }

    if accepted {
        let owner_team = game
            .teams
            .iter()
            .find(|team| team.id == user_team_id)
            .ok_or("User team not found")?;
        let buyer_team = game
            .teams
            .iter()
            .find(|team| team.id == buyer_team_id)
            .ok_or("Buying team not found")?;

        if !player_accepts_transfer(current_date, &player_snapshot, owner_team, buyer_team) {
            if let Some(player) = game
                .players
                .iter_mut()
                .find(|player| player.id == player_id)
                && let Some(offer) = player
                    .transfer_offers
                    .iter_mut()
                    .find(|offer| offer.id == offer_id)
            {
                offer.status = TransferOfferStatus::Rejected;
                offer.last_manager_fee = Some(requested_fee);
                offer.negotiation_round = round;
                offer.suggested_counter_fee = None;
                offer.date = date;
            }

            return Ok(transfer_outcome(
                TransferNegotiationDecision::Rejected,
                None,
                true,
                build_transfer_feedback(
                    "transfers.transferFeedbackRejectedHeadline",
                    "transfers.transferFeedbackPlayerRejectedDetail",
                    NegotiationMood::Guarded,
                    tension.saturating_add(6).min(90),
                    patience.saturating_sub(8),
                    round,
                    &[("fee", requested_fee.to_string())],
                ),
            ));
        }

        execute_transfer(
            game,
            player_id,
            &buyer_team_id,
            &user_team_id,
            requested_fee,
        )?;
        return Ok(transfer_outcome(
            TransferNegotiationDecision::Accepted,
            None,
            true,
            build_transfer_feedback(
                "transfers.transferFeedbackAcceptedHeadline",
                "transfers.transferFeedbackAcceptedDetail",
                NegotiationMood::Positive,
                tension.saturating_sub(8),
                patience.saturating_add(8).min(92),
                round,
                &[("fee", requested_fee.to_string())],
            ),
        ));
    }

    if requested_fee <= counter_window {
        let suggested_fee = round_transfer_fee(counter_ceiling);
        if let Some(player) = game
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
            && let Some(offer) = player
                .transfer_offers
                .iter_mut()
                .find(|offer| offer.id == offer_id)
        {
            offer.fee = suggested_fee;
            offer.status = TransferOfferStatus::Pending;
            offer.last_manager_fee = Some(requested_fee);
            offer.negotiation_round = round;
            offer.suggested_counter_fee = Some(suggested_fee);
            offer.date = date;
        }

        return Ok(transfer_outcome(
            TransferNegotiationDecision::CounterOffer,
            Some(suggested_fee),
            false,
            build_transfer_feedback(
                "transfers.transferFeedbackCounterHeadline",
                "transfers.transferFeedbackCounterDetail",
                NegotiationMood::Firm,
                tension,
                patience,
                round,
                &[("fee", suggested_fee.to_string())],
            ),
        ));
    }

    Ok(transfer_outcome(
        TransferNegotiationDecision::Rejected,
        None,
        true,
        build_transfer_feedback(
            "transfers.transferFeedbackRejectedHeadline",
            "transfers.transferFeedbackRejectedDetail",
            NegotiationMood::Tense,
            tension.saturating_add(10).min(92),
            patience.saturating_sub(12),
            round,
            &[("fee", round_transfer_fee(counter_ceiling).to_string())],
        ),
    ))
}

fn round_transfer_fee(value: u64) -> u64 {
    if value == 0 {
        return 0;
    }

    ((value + 49_999) / 50_000) * 50_000
}

fn remove_player_from_team_references(team: &mut domain::team::Team, player_id: &str) {
    team.starting_xi_ids.retain(|id| id != player_id);

    for group in &mut team.training_groups {
        group.player_ids.retain(|id| id != player_id);
    }

    if team.match_roles.captain.as_deref() == Some(player_id) {
        team.match_roles.captain = None;
    }
    if team.match_roles.vice_captain.as_deref() == Some(player_id) {
        team.match_roles.vice_captain = None;
    }
    if team.match_roles.penalty_taker.as_deref() == Some(player_id) {
        team.match_roles.penalty_taker = None;
    }
    if team.match_roles.free_kick_taker.as_deref() == Some(player_id) {
        team.match_roles.free_kick_taker = None;
    }
    if team.match_roles.corner_taker.as_deref() == Some(player_id) {
        team.match_roles.corner_taker = None;
    }
}

fn remaining_contract_salary(player: &domain::player::Player, current_date: NaiveDate) -> i64 {
    let Some(days_remaining) =
        contract_days_remaining(current_date, player.contract_end.as_deref())
    else {
        return 0;
    };

    let bounded_days = days_remaining.max(0);
    if bounded_days == 0 {
        return 0;
    }

    ((i64::from(player.wage) * bounded_days) + 364) / 365
}

fn release_penalty_amount(player: &domain::player::Player, current_date: NaiveDate) -> i64 {
    let remaining_salary = remaining_contract_salary(player, current_date);
    if remaining_salary <= 0 {
        return 0;
    }

    (remaining_salary * CONTRACT_RELEASE_PENALTY_PCT + 99) / 100
}

fn player_accepts_transfer(
    current_date: NaiveDate,
    player: &domain::player::Player,
    owner_team: &domain::team::Team,
    buyer_team: &domain::team::Team,
) -> bool {
    let openness = player_move_openness_score(current_date, player, owner_team, buyer_team);
    let reputation_gap = buyer_team.reputation as i32 - owner_team.reputation as i32;
    let days_remaining =
        contract_days_remaining(current_date, player.contract_end.as_deref()).unwrap_or(365);

    let mut acceptance_score = openness;
    if reputation_gap >= 0 {
        acceptance_score += 12;
    } else if reputation_gap <= -75 {
        acceptance_score -= 18;
    }

    if days_remaining <= 180 {
        acceptance_score += 10;
    } else if days_remaining <= 365 {
        acceptance_score += 6;
    }

    if player.morale >= 80 {
        acceptance_score -= 8;
    }

    acceptance_score >= 22
}

pub fn release_player_contract(game: &mut Game, player_id: &str) -> Result<i64, String> {
    if !transfer_window_is_open(game) {
        return Err("Transfer window is closed".to_string());
    }

    let user_team_id = game
        .manager
        .team_id
        .clone()
        .ok_or_else(|| "No user team".to_string())?;

    let mut managed_team_ids = std::collections::HashSet::new();
    managed_team_ids.insert(user_team_id.clone());
    for team in &game.teams {
        if team.team_kind == TeamKind::Academy
            && team.parent_team_id.as_deref() == Some(&user_team_id)
        {
            managed_team_ids.insert(team.id.clone());
        }
    }

    let current_date = game.clock.current_date.date_naive();
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();

    let (owning_team_id, player_name, penalty) = {
        let player = game
            .players
            .iter()
            .find(|player| player.id == player_id)
            .ok_or_else(|| "Player not found".to_string())?;

        let owning_team_id = player
            .team_id
            .clone()
            .ok_or_else(|| "Player is already a free agent".to_string())?;

        if !managed_team_ids.contains(&owning_team_id) {
            return Err("Player not found or not yours".to_string());
        }

        if player.contract_end.is_none() {
            return Err("Player has no active contract".to_string());
        }

        (
            owning_team_id,
            player.match_name.clone(),
            release_penalty_amount(player, current_date),
        )
    };

    let team = game
        .teams
        .iter_mut()
        .find(|team| team.id == owning_team_id)
        .ok_or_else(|| "Owning team not found".to_string())?;

    if team.finance < penalty {
        return Err(format!(
            "Insufficient funds for contract termination: need €{}",
            penalty
        ));
    }

    team.finance -= penalty;
    team.season_expenses += penalty;
    remove_player_from_team_references(team, player_id);

    if let Some(player) = game
        .players
        .iter_mut()
        .find(|player| player.id == player_id)
    {
        player.team_id = None;
        player.contract_end = None;
        player.wage = 0;
        player.transfer_listed = false;
        player.loan_listed = false;
        player.transfer_offers.clear();
    }

    let message = domain::message::InboxMessage::new(
        format!("contract_terminated_{}", player_id),
        format!("Contract terminated: {}", player_name),
        format!(
            "You terminated {}'s contract. The player is now a free agent. Termination cost: €{}.",
            player_name, penalty
        ),
        "Director of Football".to_string(),
        today,
    )
    .with_category(domain::message::MessageCategory::Contract)
    .with_priority(domain::message::MessagePriority::High)
    .with_sender_role("Director of Football");
    game.messages.push(message);

    Ok(penalty)
}

fn lol_role_for_position(position: &Position) -> &'static str {
    match position {
        Position::Defender
        | Position::RightBack
        | Position::CenterBack
        | Position::LeftBack
        | Position::RightWingBack
        | Position::LeftWingBack => "TOP",
        Position::AttackingMidfielder | Position::RightMidfielder | Position::LeftMidfielder => {
            "MID"
        }
        Position::Forward | Position::RightWinger | Position::LeftWinger | Position::Striker => {
            "ADC"
        }
        Position::Goalkeeper | Position::DefensiveMidfielder => "SUPPORT",
        Position::Midfielder | Position::CentralMidfielder => "JUNGLE",
    }
}

fn position_for_lol_role(role: &str) -> Position {
    match role {
        "TOP" => Position::Defender,
        "JUNGLE" => Position::Midfielder,
        "MID" => Position::AttackingMidfielder,
        "ADC" => Position::Forward,
        "SUPPORT" => Position::DefensiveMidfielder,
        _ => Position::Midfielder,
    }
}

fn academy_role_count(game: &Game, academy_team_id: &str, role: &str) -> usize {
    game.players
        .iter()
        .filter(|player| player.team_id.as_deref() == Some(academy_team_id))
        .filter(|player| lol_role_for_position(&player.natural_position) == role)
        .count()
}

fn try_assign_free_agent_by_role(game: &mut Game, academy_team_id: &str, role: &str) -> bool {
    let candidate_id = game
        .players
        .iter()
        .filter(|player| player.team_id.is_none())
        .filter(|player| lol_role_for_position(&player.natural_position) == role)
        .max_by_key(|player| player.market_value)
        .map(|player| player.id.clone());

    let Some(candidate_id) = candidate_id else {
        return false;
    };

    if let Some(player) = game
        .players
        .iter_mut()
        .find(|player| player.id == candidate_id)
    {
        player.team_id = Some(academy_team_id.to_string());
        player.transfer_listed = false;
        player.loan_listed = false;
        if player.contract_end.is_none() {
            player.contract_end = Some(format!("{}-11-30", game.clock.current_date.year() + 2));
        }
        return true;
    }

    false
}

fn spawn_academy_replacement(
    game: &mut Game,
    academy_team_id: &str,
    template: &domain::player::Player,
    role: &str,
) {
    let replacement_id = format!("academy-replacement-{}-{}", academy_team_id, Uuid::new_v4());
    let match_name = format!("{} Prospect", role);
    let mut replacement = domain::player::Player::new(
        replacement_id,
        match_name.clone(),
        match_name,
        "2006-01-01".to_string(),
        template.nationality.clone(),
        position_for_lol_role(role),
        template.attributes.clone(),
    );
    replacement.team_id = Some(academy_team_id.to_string());
    replacement.contract_end = Some(format!("{}-11-30", game.clock.current_date.year() + 2));
    replacement.wage = template.wage.max(6_000) / 2;
    replacement.market_value = template.market_value.max(120_000) / 2;
    replacement.morale = 62;
    replacement.condition = 100;
    replacement.potential_base = template.potential_base;
    game.players.push(replacement);
}

fn ensure_academy_roster_continuity(
    game: &mut Game,
    academy_team_id: &str,
    template: &domain::player::Player,
) {
    let required_roles = ["TOP", "JUNGLE", "MID", "ADC", "SUPPORT"];

    for _ in 0..8 {
        let roster_size = game
            .players
            .iter()
            .filter(|player| player.team_id.as_deref() == Some(academy_team_id))
            .count();
        let missing_role = required_roles
            .iter()
            .copied()
            .find(|role| academy_role_count(game, academy_team_id, role) == 0);

        if roster_size >= 5 && missing_role.is_none() {
            break;
        }

        let target_role =
            missing_role.unwrap_or_else(|| lol_role_for_position(&template.natural_position));
        if !try_assign_free_agent_by_role(game, academy_team_id, target_role) {
            spawn_academy_replacement(game, academy_team_id, template, target_role);
        }
    }
}

fn build_transfer_feedback(
    headline_key: &str,
    detail_key: &str,
    mood: NegotiationMood,
    tension: u8,
    patience: u8,
    round: u8,
    params: &[(&str, String)],
) -> NegotiationFeedback {
    NegotiationFeedback {
        mood,
        headline_key: headline_key.to_string(),
        detail_key: Some(detail_key.to_string()),
        tension,
        patience,
        round,
        params: params
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect(),
    }
}

/// Transfer a player between teams, adjusting finances.
fn execute_transfer(
    game: &mut Game,
    player_id: &str,
    to_team_id: &str,
    from_team_id: &str,
    fee: u64,
) -> Result<(), String> {
    let player_snapshot = game
        .players
        .iter()
        .find(|player| player.id == player_id)
        .cloned()
        .ok_or("Player not found")?;
    let from_team_name = game
        .teams
        .iter()
        .find(|team| team.id == from_team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| from_team_id.to_string());
    let to_team_name = game
        .teams
        .iter()
        .find(|team| team.id == to_team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| to_team_id.to_string());
    let today = game.clock.current_date.format("%Y-%m-%d").to_string();
    let selling_team_is_academy = game
        .teams
        .iter()
        .find(|team| team.id == from_team_id)
        .map(|team| team.team_kind == TeamKind::Academy)
        .unwrap_or(false);
    let departing_starter_ids: Vec<String> = game
        .teams
        .iter()
        .find(|team| team.id == from_team_id)
        .filter(|team| team.starting_xi_ids.iter().any(|id| id == player_id))
        .map(|team| {
            team.starting_xi_ids
                .iter()
                .filter(|id| id.as_str() != player_id)
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    // Move player
    if let Some(p) = game.players.iter_mut().find(|p| p.id == player_id) {
        p.team_id = Some(to_team_id.to_string());
        p.transfer_listed = false;
        p.loan_listed = false;
        // Remove from any starting XI
    }

    if !departing_starter_ids.is_empty() {
        for player in &mut game.players {
            if player.team_id.as_deref() == Some(from_team_id)
                && departing_starter_ids.iter().any(|id| id == &player.id)
            {
                player.morale = (i16::from(player.morale) - 4).clamp(0, 100) as u8;
            }
        }
    }

    // Debit buying team
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == to_team_id) {
        t.finance -= fee as i64;
        t.transfer_budget -= fee as i64;
        // Remove from starting XI if player was there
        if let Some(pos) = t.starting_xi_ids.iter().position(|id| id == player_id) {
            t.starting_xi_ids.remove(pos);
        }
    }

    let academy_owner_id = game
        .teams
        .iter()
        .find(|team| team.id == from_team_id && team.team_kind == TeamKind::Academy)
        .and_then(|team| team.parent_team_id.clone());

    // Credit selling team or academy owner
    let credit_target_id = academy_owner_id.as_deref().unwrap_or(from_team_id);
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == credit_target_id) {
        t.finance += fee as i64;
        t.transfer_budget += (fee as i64 * TRANSFER_BUDGET_SELLING_REALLOCATION_PCT) / 100;
    }

    // Remove sold player from selling team XI if present
    if let Some(t) = game.teams.iter_mut().find(|t| t.id == from_team_id) {
        if let Some(pos) = t.starting_xi_ids.iter().position(|id| id == player_id) {
            t.starting_xi_ids.remove(pos);
        }
    }

    if selling_team_is_academy {
        ensure_academy_roster_continuity(game, from_team_id, &player_snapshot);
    }

    if should_generate_major_transfer_news(&player_snapshot, fee) {
        let article_id = format!(
            "transfer_news_{}_{}_{}_{}",
            player_id, from_team_id, to_team_id, today
        );
        if !game.news.iter().any(|article| article.id == article_id) {
            game.news.push(crate::news::major_transfer_article(
                &article_id,
                player_id,
                &player_snapshot.match_name,
                from_team_id,
                &from_team_name,
                to_team_id,
                &to_team_name,
                fee,
                &today,
            ));
        }
    }

    Ok(())
}
