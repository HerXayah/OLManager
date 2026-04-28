mod fitness_warnings;
pub use fitness_warnings::check_squad_fitness_warnings;

use crate::game::Game;
use crate::potential::{calculate_lol_ovr, effective_potential_cap};
use crate::staff_effects::LolStaffEffects;
use chrono::Datelike;
use domain::message::{InboxMessage, MessageCategory, MessagePriority};
use domain::staff::CoachingSpecialization;
use domain::team::{MainFacilityModuleKind, TrainingFocus, TrainingIntensity, TrainingSchedule};
use std::collections::HashMap;

fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Computed coaching quality for a team's staff.
pub struct TeamCoachingBonus {
    pub coaching_mult: f64, // Overall coaching quality multiplier (1.0 = no staff)
    pub specialization_mult: f64, // Extra bonus if a coach specializes in the current focus
    pub physio_mult: f64,   // Recovery bonus from physio staff
}

/// Compute coaching bonuses from a team's staff.
fn compute_coaching_bonus(game: &Game, team_id: &str, focus: &TrainingFocus) -> TeamCoachingBonus {
    let focus_spec = match focus {
        TrainingFocus::Scrims => Some(CoachingSpecialization::Tactics),
        TrainingFocus::VODReview => Some(CoachingSpecialization::Tactics),
        TrainingFocus::IndividualCoaching => Some(CoachingSpecialization::Technique),
        TrainingFocus::ChampionPoolPractice => Some(CoachingSpecialization::Technique),
        TrainingFocus::MacroSystems => Some(CoachingSpecialization::Tactics),
        TrainingFocus::MentalResetRecovery => None,
    };

    let effects = LolStaffEffects::for_team(&game.staff, team_id);
    let specialization_mult =
        effects.focus_specialization_multiplier(&game.staff, team_id, focus_spec);

    TeamCoachingBonus {
        coaching_mult: effects.coaching,
        specialization_mult,
        physio_mult: effects.recovery,
    }
}

/// Per-team data collected before mutating players.
struct TeamTrainingPlan {
    team_id: String,
    default_focus: TrainingFocus,
    intensity: TrainingIntensity,
    schedule: TrainingSchedule,
    bonus: TeamCoachingBonus,
    medical_facility_mult: f64,
    training_facility_mult: f64,
}

#[derive(Clone)]
struct TeamScrimDayOutcome {
    gain_mult: f64,
    morale_penalty: u8,
    next_loss_streak: u8,
    played: u8,
    wins: u8,
    losses: u8,
    slot_results: Vec<(u8, u8, String, bool)>,
}

fn scrims_per_week_for_schedule(schedule: &TrainingSchedule) -> usize {
    match schedule {
        TrainingSchedule::Intense => 6,
        TrainingSchedule::Balanced => 4,
        TrainingSchedule::Light => 2,
    }
}

fn scrim_slot_weekdays(schedule: &TrainingSchedule) -> &'static [u32] {
    match schedule {
        // Redistributed to Tue/Wed/Thu to avoid match-day clashes.
        TrainingSchedule::Intense => &[1, 1, 2, 2, 3, 3],
        TrainingSchedule::Balanced => &[1, 2, 2, 3],
        TrainingSchedule::Light => &[1, 3],
    }
}

fn scrim_slots_for_day(schedule: &TrainingSchedule, weekday_num: u32) -> Vec<usize> {
    scrim_slot_weekdays(schedule)
        .iter()
        .enumerate()
        .filter_map(|(index, day)| {
            if *day == weekday_num {
                Some(index)
            } else {
                None
            }
        })
        .take(scrims_per_week_for_schedule(schedule))
        .collect()
}

fn team_lol_strength(game: &Game, team_id: &str) -> f64 {
    let starting_ids: Vec<String> = game
        .teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.starting_xi_ids.clone())
        .unwrap_or_default();

    let mut values: Vec<f64> = if !starting_ids.is_empty() {
        let mut from_starting: Vec<f64> = starting_ids
            .iter()
            .filter_map(|pid| game.players.iter().find(|p| p.id == *pid))
            .filter(|player| player.team_id.as_deref() == Some(team_id))
            .take(5)
            .map(|player| f64::from(calculate_lol_ovr(player)))
            .collect();

        if from_starting.len() < 5 {
            let mut fallback: Vec<f64> = game
                .players
                .iter()
                .filter(|player| player.team_id.as_deref() == Some(team_id))
                .map(|player| f64::from(calculate_lol_ovr(player)))
                .collect();
            fallback.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
            for candidate in fallback {
                if from_starting.len() >= 5 {
                    break;
                }
                from_starting.push(candidate);
            }
        }
        from_starting
    } else {
        game.players
            .iter()
            .filter(|player| player.team_id.as_deref() == Some(team_id))
            .map(|player| f64::from(calculate_lol_ovr(player)))
            .collect()
    };

    if values.is_empty() {
        return 74.0;
    }
    values.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let sample = values.iter().take(5).copied().collect::<Vec<_>>();
    sample.iter().sum::<f64>() / sample.len() as f64
}

fn compute_scrim_gain_multiplier(own_strength: f64, opponent_strength: f64) -> f64 {
    let diff = (opponent_strength - own_strength).clamp(-12.0, 12.0);
    (1.0 + diff * 0.016).clamp(0.85, 1.25)
}

/// Process daily training for all teams.
/// On non-match days each team's players train according to the team's
/// current focus, intensity, and schedule. Rest days (determined by the
/// weekly schedule) give full condition recovery with no training cost.
/// Scrims focus can gain extra efficiency from stronger weekly scrim opponents.
/// `weekday_num` is 0=Mon .. 6=Sun (chrono Weekday::num_days_from_monday()).
pub fn process_training(game: &mut Game, weekday_num: u32) {
    let manager_team_id = game.manager.team_id.clone();
    let rival_player_ids: Vec<String> = game
        .players
        .iter()
        .filter(|player| {
            player.team_id.as_ref().is_some_and(|team_id| {
                manager_team_id.as_ref().is_none_or(|manager_id| team_id != manager_id)
            })
        })
        .map(|player| player.id.clone())
        .collect();
    for player_id in rival_player_ids {
        crate::champions::ensure_training_targets_from_mastery(game, &player_id);
    }

    // Collect plans for all teams (immutable borrow)
    let team_plans: Vec<TeamTrainingPlan> = game
        .teams
        .iter()
        .map(|t| {
            let bonus = compute_coaching_bonus(game, &t.id, &t.training_focus);
            let medical_facility_mult = t.facilities.recovery_suite_condition_multiplier();
            TeamTrainingPlan {
                team_id: t.id.clone(),
                default_focus: t.training_focus.clone(),
                intensity: t.training_intensity.clone(),
                schedule: t.training_schedule.clone(),
                bonus,
                medical_facility_mult,
                training_facility_mult: 1.0
                    + f64::from(
                        t.facilities
                            .module_level(MainFacilityModuleKind::ScrimsRoom)
                            .saturating_sub(1),
                    ) * 0.03,
            }
        })
        .collect();

    let strength_by_team: HashMap<String, f64> = game
        .teams
        .iter()
        .map(|team| (team.id.clone(), team_lol_strength(game, &team.id)))
        .collect();

    let mut scrim_outcome_by_team: HashMap<String, TeamScrimDayOutcome> = HashMap::new();
    let week_seed = format!(
        "{}-W{}",
        game.clock.current_date.iso_week().year(),
        game.clock.current_date.iso_week().week()
    );

    for team in game.teams.iter() {
        let day_slots = scrim_slots_for_day(&team.training_schedule, weekday_num);
        if day_slots.is_empty() {
            continue;
        }

        let mut opponent_pool: Vec<String> = team
            .weekly_scrim_opponent_ids
            .iter()
            .filter(|candidate| candidate.as_str() != team.id.as_str())
            .filter(|candidate| strength_by_team.contains_key(candidate.as_str()))
            .cloned()
            .collect();

        if opponent_pool.is_empty() {
            opponent_pool = game
                .teams
                .iter()
                .filter(|candidate| candidate.id != team.id)
                .map(|candidate| candidate.id.clone())
                .collect();
        }

        if opponent_pool.is_empty() {
            continue;
        }

        let own_strength = *strength_by_team.get(&team.id).unwrap_or(&74.0);
        let staff_effects = LolStaffEffects::for_team(&game.staff, &team.id);
        let mut gain_sum = 0.0;
        let mut played: u8 = 0;
        let mut wins: u8 = 0;
        let mut losses: u8 = 0;
        let mut next_loss_streak = team.scrim_loss_streak;
        let mut slot_results: Vec<(u8, u8, String, bool)> = Vec::new();

        for slot_idx in day_slots {
            let configured = team
                .weekly_scrim_opponent_ids
                .get(slot_idx)
                .cloned()
                .unwrap_or_default();
            let opponent_id = if configured.is_empty()
                || configured == team.id
                || !strength_by_team.contains_key(&configured)
            {
                let selector_seed = format!("{}:{}:{}", week_seed, team.id, slot_idx);
                let selector_roll = {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    selector_seed.hash(&mut hasher);
                    hasher.finish() as usize
                };
                opponent_pool[selector_roll % opponent_pool.len()].clone()
            } else {
                configured
            };

            let opponent_strength = *strength_by_team.get(&opponent_id).unwrap_or(&own_strength);
            let gain_mult = compute_scrim_gain_multiplier(own_strength, opponent_strength)
                * ((staff_effects.tactics * 0.55) + (staff_effects.analysis * 0.45))
                    .clamp(0.90, 1.15);
            gain_sum += gain_mult;
            played = played.saturating_add(1);

            let diff = (own_strength - opponent_strength).clamp(-14.0, 14.0);
            let win_prob = (0.5 + diff * 0.022).clamp(0.2, 0.8);
            let seed = format!(
                "scrim:{}:{}:{}:{}:{}",
                week_seed, team.id, opponent_id, weekday_num, slot_idx
            );
            let roll = {
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                seed.hash(&mut hasher);
                (hasher.finish() % 10_000) as f64 / 10_000.0
            };
            let won_scrim = roll <= win_prob;

            if won_scrim {
                wins = wins.saturating_add(1);
                next_loss_streak = 0;
            } else {
                losses = losses.saturating_add(1);
                next_loss_streak = next_loss_streak.saturating_add(1);
            }

            slot_results.push((slot_idx as u8, weekday_num as u8, opponent_id, won_scrim));
        }

        if played == 0 {
            continue;
        }

        let base_morale_penalty = if next_loss_streak >= 5 {
            4
        } else if next_loss_streak >= 4 {
            3
        } else if next_loss_streak >= 3 {
            2
        } else {
            0
        };
        let morale_softening = ((staff_effects.morale - 1.0).max(0.0)
            + (staff_effects.recovery - 1.0).max(0.0))
        .clamp(0.0, 0.35);
        let morale_penalty =
            ((f64::from(base_morale_penalty)) * (1.0 - morale_softening)).round() as u8;

        scrim_outcome_by_team.insert(
            team.id.clone(),
            TeamScrimDayOutcome {
                gain_mult: (gain_sum / f64::from(played.max(1))).clamp(0.80, 1.30),
                morale_penalty,
                next_loss_streak,
                played,
                wins,
                losses,
                slot_results,
            },
        );
    }

    let mut mastery_training_ticks: Vec<(String, String, f64, u8)> = Vec::new();

    for plan in &team_plans {
        let is_training_day = plan.schedule.is_training_day(weekday_num);

        let intensity_mult = match &plan.intensity {
            TrainingIntensity::Low => 0.5,
            TrainingIntensity::Medium => 1.0,
            TrainingIntensity::High => 1.5,
        };

        for player in game.players.iter_mut() {
            if player.team_id.as_deref() != Some(&plan.team_id) {
                continue;
            }

            // Determine this player's effective focus:
            // player override > team default
            let player_focus = player
                .training_focus
                .as_ref()
                .unwrap_or(&plan.default_focus);

            // On rest days or recovery-focused plans: no training cost
            let condition_cost: u8 = if !is_training_day {
                0
            } else {
                match (player_focus, &plan.intensity) {
                    (focus, _) if focus.is_recovery_plan() => 0,
                    (_, TrainingIntensity::Low) => 3,
                    (_, TrainingIntensity::Medium) => 6,
                    (_, TrainingIntensity::High) => 10,
                }
            };

            // Recovery amount: rest days get boosted recovery (like mental reset days)
            let recovery_base: f64 = if !is_training_day {
                7.0 * plan.bonus.physio_mult * plan.medical_facility_mult
            } else {
                match player_focus {
                    TrainingFocus::MentalResetRecovery => {
                        9.0 * plan.bonus.physio_mult * plan.medical_facility_mult
                    }
                    _ => 3.0 * plan.bonus.physio_mult * plan.medical_facility_mult,
                }
            };

            // Age, morale, and current condition all affect recovery rate.
            // Older players recover more slowly; high morale aids recovery;
            // severely fatigued players have a harder time bouncing back.
            let age = estimate_age(&player.date_of_birth);
            let age_rec = recovery_factor_from_age(age);
            let morale_rec = recovery_factor_from_morale(player.morale);
            let condition_rec = recovery_factor_from_condition(player.condition);
            let fitness_rec = recovery_factor_from_fitness(player.fitness);

            // Injured players: half base recovery, scaled by age and morale.
            // Fitness decays slowly during injury (inactive = losing sharpness).
            if player.injury.is_some() {
                let recovery = (recovery_base * 0.5 * age_rec * morale_rec * fitness_rec) as u8;
                player.condition = (player.condition + recovery).min(100);
                player.fitness = clamp_fitness(player.fitness as i16 - 1);
                continue;
            }

            // On rest days: only recovery, no attribute gains
            if !is_training_day {
                let stamina_factor = player.attributes.stamina as f64 / 100.0;
                let recovery = (recovery_base
                    * (0.5 + stamina_factor * 0.5)
                    * age_rec
                    * morale_rec
                    * condition_rec
                    * fitness_rec) as u8;
                player.condition = (player.condition + recovery).min(100);
                continue;
            }

            // Age factor for attribute gains: younger players grow faster, older players slower
            let age_factor = if age <= 21 {
                1.5
            } else if age <= 25 {
                1.2
            } else if age <= 29 {
                1.0
            } else if age <= 33 {
                0.6
            } else {
                0.3
            };

            // Base gain per session for the underlying model, boosted by coaching staff.
            // The selected attributes are tuned so the LoL-facing roster/profile stats
            // shown to the user move in the expected direction without rewriting the
            // whole legacy player model.
            let gain = 0.15
                * intensity_mult
                * age_factor
                * plan.bonus.coaching_mult
                * plan.bonus.specialization_mult
                * plan.training_facility_mult;

            let scrim_gain_mult = if matches!(player_focus, TrainingFocus::Scrims) {
                scrim_outcome_by_team
                    .get(&plan.team_id)
                    .map(|outcome| outcome.gain_mult)
                    .unwrap_or(1.0)
            } else {
                1.0
            };
            let gain = gain * scrim_gain_mult;

            // Apply LoL stat gains only when the player's current LoL OVR is below potential cap.
            let capped = is_lol_training_capped(player);
            apply_focus_gains(&mut player.attributes, player_focus, gain, capped);

            if is_training_day && !player_focus.is_recovery_plan() {
                let targets = crate::champions::training_targets_for_player(player);
                let (focus_mult, attempts): (f64, u8) = match player_focus {
                    TrainingFocus::ChampionPoolPractice => (1.4, 4),
                    TrainingFocus::IndividualCoaching => (1.15, 3),
                    TrainingFocus::Scrims => (1.0, 3),
                    TrainingFocus::MacroSystems => (0.9, 2),
                    TrainingFocus::VODReview => (0.85, 2),
                    TrainingFocus::MentalResetRecovery => (0.0, 0),
                };

                if attempts > 0 && !targets.is_empty() {
                    let priority_weights: [f64; 3] = [1.0, 0.65, 0.4];
                    for (index, champion_id) in targets.iter().enumerate() {
                        let weight = priority_weights.get(index).copied().unwrap_or(0.3);
                        let weighted_attempts = ((attempts as f64) * weight).round() as u8;
                        if weighted_attempts == 0 {
                            continue;
                        }
                        let mastery_gain_factor = gain * focus_mult * (0.85 + weight * 0.35);
                        mastery_training_ticks.push((
                            player.id.clone(),
                            champion_id.clone(),
                            mastery_gain_factor,
                            weighted_attempts.max(1),
                        ));
                    }
                }
            }

            // Apply fitness changes based on training focus.
            // Scrims best preserve fitness; recovery plans give a tiny boost.
            apply_fitness_change(&mut player.fitness, player_focus, intensity_mult);

            // Apply condition: deplete from training, then recover
            player.condition = player.condition.saturating_sub(condition_cost);
            let stamina_factor = player.attributes.stamina as f64 / 100.0;
            let recovery = (recovery_base
                * (0.5 + stamina_factor * 0.5)
                * age_rec
                * morale_rec
                * condition_rec
                * fitness_rec) as u8;
            player.condition = (player.condition + recovery).min(100);
        }
    }

    for team in game.teams.iter_mut() {
        if let Some(outcome) = scrim_outcome_by_team.get(&team.id) {
            team.scrim_loss_streak = outcome.next_loss_streak;
            team.scrim_weekly_played = team.scrim_weekly_played.saturating_add(outcome.played);
            team.scrim_weekly_wins = team.scrim_weekly_wins.saturating_add(outcome.wins);
            team.scrim_weekly_losses = team.scrim_weekly_losses.saturating_add(outcome.losses);

            for (slot_index, weekday, opponent_team_id, won) in &outcome.slot_results {
                let already_exists = team
                    .scrim_slot_results
                    .iter()
                    .any(|entry| entry.week_key == week_seed && entry.slot_index == *slot_index);
                if already_exists {
                    continue;
                }

                team.scrim_slot_results.push(domain::team::ScrimSlotResult {
                    week_key: week_seed.clone(),
                    slot_index: *slot_index,
                    weekday: *weekday,
                    opponent_team_id: opponent_team_id.clone(),
                    won: *won,
                    simulated_on: game.clock.current_date.format("%Y-%m-%d").to_string(),
                });
            }

            // Keep only recent history to avoid save growth.
            if team.scrim_slot_results.len() > 96 {
                let start = team.scrim_slot_results.len().saturating_sub(96);
                team.scrim_slot_results = team.scrim_slot_results.split_off(start);
            }
        }
    }

    for player in game.players.iter_mut() {
        let Some(team_id) = player.team_id.as_ref() else {
            continue;
        };
        let Some(outcome) = scrim_outcome_by_team.get(team_id) else {
            continue;
        };
        if outcome.morale_penalty == 0 {
            continue;
        }

        player.morale = player.morale.saturating_sub(outcome.morale_penalty);
    }

    for (player_id, champion_id, gain, attempts) in mastery_training_ticks {
        let soloq_mult = crate::champions::mastery_gain_multiplier_for_player(game, &player_id);
        let effective_gain = gain * soloq_mult;
        for _ in 0..attempts {
            crate::champions::apply_training_mastery_progress(
                game,
                &player_id,
                &champion_id,
                effective_gain,
            );
        }
    }

    if weekday_num == 6
        && let Some(manager_team_id) = game.manager.team_id.clone()
        && let Some(team) = game
            .teams
            .iter_mut()
            .find(|candidate| candidate.id == manager_team_id)
    {
        if team.scrim_weekly_played > 0 {
            let body = format!(
                "Weekly scrim report:\n\nPlayed: {}\nWins: {}\nLosses: {}\nCurrent loss streak: {}\n\nScrim progress applies even on losses, but extended losing streaks are hurting morale.",
                team.scrim_weekly_played,
                team.scrim_weekly_wins,
                team.scrim_weekly_losses,
                team.scrim_loss_streak,
            );

            let msg = InboxMessage::new(
                format!("msg_scrim_weekly_{}", uuid::Uuid::new_v4()),
                "Weekly Scrim Staff Report".to_string(),
                body,
                "Coaching Staff".to_string(),
                game.clock.current_date.to_rfc3339(),
            )
            .with_category(MessageCategory::System)
            .with_priority(MessagePriority::Normal)
            .with_sender_role("Coaching Staff")
            .with_i18n(
                "be.msg.scrimWeekly.subject",
                "be.msg.scrimWeekly.body",
                params(&[
                    ("played", &team.scrim_weekly_played.to_string()),
                    ("wins", &team.scrim_weekly_wins.to_string()),
                    ("losses", &team.scrim_weekly_losses.to_string()),
                    ("lossStreak", &team.scrim_loss_streak.to_string()),
                ]),
            )
            .with_sender_i18n("be.sender.coachingStaff", "be.role.coachingStaff");

            game.messages.push(msg);
        }

        team.scrim_weekly_played = 0;
        team.scrim_weekly_wins = 0;
        team.scrim_weekly_losses = 0;
    }
}

/// Apply fitness changes based on training focus.
/// Scrims best preserve fitness, while recovery plans give a tiny boost.
/// Other plans slowly decay very high fitness if not maintained.
fn apply_fitness_change(fitness: &mut u8, focus: &TrainingFocus, intensity_mult: f64) {
    use rand::RngExt;
    let mut rng = rand::rng();
    match focus {
        TrainingFocus::Scrims => {
            // Scrims are the closest MVP equivalent to high-load team practice.
            // Higher intensity → higher gain probability.
            let gain_prob = 0.012 * intensity_mult; // 0.006–0.018 per session
            let roll: f64 = rng.random_range(0.0..1.0);
            if roll < gain_prob && *fitness < 100 {
                *fitness = fitness.saturating_add(1);
            }
        }
        TrainingFocus::MentalResetRecovery => {
            // Recovery-focused days give a tiny fitness nudge.
            let roll: f64 = rng.random_range(0.0..1.0);
            if roll < 0.05 && *fitness < 100 {
                *fitness = fitness.saturating_add(1);
            }
        }
        _ => {
            // Non-physical training: very slight decay if player is already very fit
            // (fitness above 85 needs active maintenance).
            if *fitness > 85 {
                let roll: f64 = rng.random_range(0.0..1.0);
                if roll < 0.05 {
                    *fitness = fitness.saturating_sub(1);
                }
            }
        }
    }
}

fn try_gain(current: &mut u8, gain: f64) {
    use rand::RngExt;
    if *current >= 99 {
        return;
    }
    let mut rng = rand::rng();
    let roll: f64 = rng.random_range(0.0..1.0);
    if roll < gain {
        *current = (*current + 1).min(99);
    }
}

/// Apply attribute gains based on training focus.
/// We still mutate the legacy core attributes, but we prioritize the combinations
/// that feed the LoL-facing profile/roster stats the player actually sees.
fn apply_focus_gains(
    attrs: &mut domain::player::PlayerAttributes,
    focus: &TrainingFocus,
    gain: f64,
    capped: bool,
) {
    if capped {
        return;
    }

    // LoL-native stat mapping (1:1 over legacy fields):
    // mechanics -> dribbling
    // laning -> shooting
    // teamfighting -> teamwork
    // macro -> vision
    // consistency -> decisions
    // shotcalling -> leadership
    // champion pool -> agility
    // discipline -> composure
    // mental resilience -> stamina
    match focus {
        TrainingFocus::Scrims => {
            try_gain(&mut attrs.decisions, gain);
            try_gain(&mut attrs.teamwork, gain);
            try_gain(&mut attrs.composure, gain * 0.85);
            try_gain(&mut attrs.stamina, gain * 0.65);
            try_gain(&mut attrs.vision, gain * 0.55);
        }
        TrainingFocus::VODReview => {
            try_gain(&mut attrs.vision, gain);
            try_gain(&mut attrs.decisions, gain);
            try_gain(&mut attrs.composure, gain * 0.75);
            try_gain(&mut attrs.leadership, gain * 0.6);
        }
        TrainingFocus::IndividualCoaching => {
            try_gain(&mut attrs.shooting, gain);
            try_gain(&mut attrs.dribbling, gain);
            try_gain(&mut attrs.agility, gain);
            try_gain(&mut attrs.composure, gain * 0.8);
            try_gain(&mut attrs.teamwork, gain * 0.4);
        }
        TrainingFocus::ChampionPoolPractice => {
            try_gain(&mut attrs.dribbling, gain);
            try_gain(&mut attrs.agility, gain);
            try_gain(&mut attrs.vision, gain * 0.8);
            try_gain(&mut attrs.shooting, gain * 0.7);
            try_gain(&mut attrs.decisions, gain * 0.65);
        }
        TrainingFocus::MacroSystems => {
            try_gain(&mut attrs.vision, gain);
            try_gain(&mut attrs.decisions, gain);
            try_gain(&mut attrs.teamwork, gain * 0.8);
            try_gain(&mut attrs.leadership, gain * 0.7);
        }
        TrainingFocus::MentalResetRecovery => {
            // No attribute gains on recovery days
        }
    }
}

fn is_lol_training_capped(player: &domain::player::Player) -> bool {
    calculate_lol_ovr(player) >= effective_potential_cap(player)
}

#[cfg(test)]
mod tests {
    use super::{apply_focus_gains, is_lol_training_capped};
    use domain::player::{Player, PlayerAttributes, Position};
    use domain::team::TrainingFocus;

    fn attrs(stat: u8) -> PlayerAttributes {
        PlayerAttributes {
            pace: stat,
            stamina: stat,
            strength: stat,
            agility: stat,
            passing: stat,
            shooting: stat,
            tackling: stat,
            dribbling: stat,
            defending: stat,
            positioning: stat,
            vision: stat,
            decisions: stat,
            composure: stat,
            aggression: stat,
            teamwork: stat,
            leadership: stat,
            handling: stat,
            reflexes: stat,
            aerial: stat,
        }
    }

    #[test]
    fn potential_cap_blocks_lol_stat_gains_when_ovr_reaches_cap() {
        let mut player = Player::new(
            "p-1".to_string(),
            "Cap".to_string(),
            "Cap".to_string(),
            "2002-01-01".to_string(),
            "GB".to_string(),
            Position::Midfielder,
            attrs(90),
        );
        player.potential_base = 90;

        assert!(is_lol_training_capped(&player));

        let before = player.attributes.clone();
        apply_focus_gains(
            &mut player.attributes,
            &TrainingFocus::IndividualCoaching,
            1.0,
            true,
        );
        assert_eq!(player.attributes.dribbling, before.dribbling);
        assert_eq!(player.attributes.shooting, before.shooting);
        assert_eq!(player.attributes.agility, before.agility);
    }
}

/// Estimate player age from date_of_birth string ("YYYY-MM-DD").
fn estimate_age(dob: &str) -> u32 {
    let parts: Vec<&str> = dob.split('-').collect();
    if parts.is_empty() {
        return 25; // fallback
    }
    let birth_year: u32 = parts[0].parse().unwrap_or(2000);
    // Use a rough estimate — the game clock year would be ideal but
    // this is close enough for growth factor purposes.
    let current_year: u32 = 2025;
    current_year.saturating_sub(birth_year)
}

/// Recovery multiplier from age: younger players bounce back faster.
fn recovery_factor_from_age(age: u32) -> f64 {
    if age <= 21 {
        1.10
    } else if age <= 25 {
        1.05
    } else if age <= 29 {
        1.00
    } else if age <= 33 {
        0.85
    } else {
        0.70
    }
}

/// Recovery multiplier from morale: players in good spirits recover better.
fn recovery_factor_from_morale(morale: u8) -> f64 {
    if morale >= 70 {
        1.10
    } else if morale >= 40 {
        1.00
    } else {
        0.90
    }
}

/// Recovery multiplier from current condition: severely fatigued players recover more slowly.
fn recovery_factor_from_condition(condition: u8) -> f64 {
    if condition < 30 {
        0.80
    } else if condition < 50 {
        0.90
    } else {
        1.00
    }
}

/// Recovery multiplier from fitness: fitter players recover condition faster.
fn recovery_factor_from_fitness(fitness: u8) -> f64 {
    if fitness < 30 {
        0.75
    } else if fitness < 50 {
        0.88
    } else if fitness < 70 {
        1.00
    } else if fitness < 90 {
        1.12
    } else {
        1.20
    }
}

/// Clamp a fitness value to 0–100.
fn clamp_fitness(val: i16) -> u8 {
    val.clamp(0, 100) as u8
}
