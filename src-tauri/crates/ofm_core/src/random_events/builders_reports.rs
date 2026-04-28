use super::{action, params};
use crate::narrative::{NarrativeSelector, load_default_content_pack};
use domain::message::*;
use rand::RngExt;

// ---------------------------------------------------------------------------
// Periodic / condition-triggered message builders
// ---------------------------------------------------------------------------

pub(super) fn mood_report_message(
    msg_id: &str,
    avg_morale: f64,
    low_count: usize,
    high_count: usize,
    total: usize,
    date: &str,
) -> InboxMessage {
    let mood = if avg_morale >= 75.0 {
        "common.moods.excellent"
    } else if avg_morale >= 60.0 {
        "common.moods.good"
    } else if avg_morale >= 45.0 {
        "common.moods.mixed"
    } else {
        "common.moods.poor"
    };

    let body = format!(
        "Here's your weekly dressing room report:\n\n\
        • Overall mood: {} (avg morale: {:.0})\n\
        • Players in high spirits (80+): {}\n\
        • Players with low morale (<40): {}\n\
        • Total squad: {}\n\n\
        {}",
        mood,
        avg_morale,
        high_count,
        low_count,
        total,
        if low_count >= 3 {
            "Several players are unhappy. You should address individual concerns before it spreads."
        } else if avg_morale >= 75.0 {
            "The dressing room is buzzing. Keep up the good work!"
        } else if avg_morale < 45.0 {
            "Morale is worryingly low. Consider positive team talks and results to turn things around."
        } else {
            "Morale is stable. A few good results would really lift the mood."
        }
    );

    InboxMessage::new(
        msg_id.to_string(),
        format!("Dressing Room Report — Mood: {}", mood),
        body,
        "Assistant Manager".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::PlayerMorale)
    .with_priority(if low_count >= 3 || avg_morale < 40.0 {
        MessagePriority::High
    } else {
        MessagePriority::Low
    })
    .with_sender_role("Assistant Manager")
    .with_action(action(
        "ack",
        "Thanks",
        "be.msg.event.ack",
        ActionType::Acknowledge,
    ))
    .with_i18n("be.msg.moodReport.subject", "be.msg.moodReport.body", {
        let mut p = params(&[("mood", mood)]);
        p.insert("avgMorale".to_string(), format!("{:.0}", avg_morale));
        p.insert("highCount".to_string(), high_count.to_string());
        p.insert("lowCount".to_string(), low_count.to_string());
        p.insert("total".to_string(), total.to_string());
        p
    })
    .with_sender_i18n("be.sender.assistantManager", "be.role.assistantManager")
}

pub(super) fn board_confidence_message(msg_id: &str, date: &str) -> InboxMessage {
    let mut rng = rand::rng();
    let variations = [
        "The board has called an urgent meeting. Three consecutive defeats have raised serious concerns about the team's direction.\n\n\
        \"We need to see improvement quickly. The fans are restless and results must change.\"\n\n\
        How do you respond?",
        "After a string of poor results, the chairman has summoned you for a difficult conversation.\n\n\
        \"We backed you with resources and time. The results simply aren't good enough. What's your plan?\"\n\n\
        Choose your response carefully.",
    ];
    let idx = rng.random_range(0..variations.len());

    InboxMessage::new(
        msg_id.to_string(),
        "Board Meeting — Results Under Scrutiny".to_string(),
        variations[idx].to_string(),
        "Board of Directors".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::BoardDirective)
    .with_priority(MessagePriority::Urgent)
    .with_sender_role("Chairman")
    .with_action(action(
        "respond",
        "Respond",
        "be.msg.event.respond",
        ActionType::ChooseOption {
            options: vec![
                ActionOption {
                    id: "reassure_board".to_string(),
                    label: "Reassure them with a plan".to_string(),
                    description:
                        "Present a clear strategy for turning things around. Buys you time."
                            .to_string(),
                    label_key: Some(
                        "be.msg.boardConfidence.options.reassureBoard.label".to_string(),
                    ),
                    description_key: Some(
                        "be.msg.boardConfidence.options.reassureBoard.description".to_string(),
                    ),
                },
                ActionOption {
                    id: "accept_pressure".to_string(),
                    label: "Accept responsibility".to_string(),
                    description: "Own the poor results. The board respects honesty.".to_string(),
                    label_key: Some(
                        "be.msg.boardConfidence.options.acceptPressure.label".to_string(),
                    ),
                    description_key: Some(
                        "be.msg.boardConfidence.options.acceptPressure.description".to_string(),
                    ),
                },
                ActionOption {
                    id: "blame_circumstances".to_string(),
                    label: "Point to injuries and bad luck".to_string(),
                    description: "Deflect blame to external factors. May or may not convince them."
                        .to_string(),
                    label_key: Some(
                        "be.msg.boardConfidence.options.blameCircumstances.label".to_string(),
                    ),
                    description_key: Some(
                        "be.msg.boardConfidence.options.blameCircumstances.description".to_string(),
                    ),
                },
            ],
        },
    ))
    .with_i18n(
        "be.msg.boardConfidence.subject",
        &format!("be.msg.boardConfidence.body{}", idx),
        params(&[]),
    )
    .with_sender_i18n("be.sender.boardOfDirectors", "be.role.chairman")
}

pub(super) fn fan_petition_message(msg_id: &str, team_name: &str, date: &str) -> InboxMessage {
    if let Some(message) = build_fan_petition_from_narrative(msg_id, team_name, date) {
        return message;
    }

    let subject = "Fan Petition — Draft Identity".to_string();
    let body = format!(
        "A group of {} supporters has organized a petition asking for a clearer draft identity and sharper objective setups.\n\n\
        \"We want proactive plans on the Rift, not passive scaling every series. Show us the team knows how it wants to win.\"\n\n\
        Over 500 signatures so far. How do you respond?",
        team_name
    );

    InboxMessage::new(
        msg_id.to_string(),
        subject,
        body,
        "Community Manager".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::Media)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("Community Manager")
    .with_action(action(
        "respond", "Respond", "be.msg.event.respond",
        ActionType::ChooseOption {
            options: vec![
                ActionOption {
                    id: "listen_fans".to_string(),
                    label: "Engage with the fans".to_string(),
                    description: "Meet with fan representatives and listen to their concerns. Good for morale.".to_string(),
                    label_key: Some("be.msg.fanPetition.options.listenFans.label".to_string()),
                    description_key: Some("be.msg.fanPetition.options.listenFans.description".to_string()),
                },
                ActionOption {
                    id: "ignore_fans".to_string(),
                    label: "Focus on prep".to_string(),
                    description: "Politely decline — competitive decisions stay with the staff.".to_string(),
                    label_key: Some("be.msg.fanPetition.options.ignoreFans.label".to_string()),
                    description_key: Some("be.msg.fanPetition.options.ignoreFans.description".to_string()),
                },
                ActionOption {
                    id: "address_publicly".to_string(),
                    label: "Make a public statement".to_string(),
                    description: "Address the petition in a press conference. Transparent and proactive.".to_string(),
                    label_key: Some("be.msg.fanPetition.options.addressPublicly.label".to_string()),
                    description_key: Some("be.msg.fanPetition.options.addressPublicly.description".to_string()),
                },
            ],
        },
    ))
        .with_i18n(
        "be.msg.fanPetition.subject",
        "be.msg.fanPetition.body",
        params(&[("team", team_name)]),
    )
    .with_sender_i18n("be.sender.communityManager", "be.role.communityManager")
}

pub fn build_fan_petition_from_narrative(
    msg_id: &str,
    team_name: &str,
    date: &str,
) -> Option<InboxMessage> {
    let pack = load_default_content_pack().ok()?;
    let selector = NarrativeSelector::new(&pack);
    let template = selector.select_event(
        Some("default"),
        &["fan", "petition"],
        &["spicy", "community"],
    )?;

    Some(
        InboxMessage::new(
            msg_id.to_string(),
            "Fan Petition — Draft Identity".to_string(),
            format!(
                "A wave of {} supporters has organized a campaign asking for a clearer draft identity and sharper objective setups.\n\n\
                \"We want proactive plans on the Rift, not passive scaling every series. Show us the team knows how it wants to win.\"\n\n\
                The thread is gaining traction across esports social channels. How do you respond?",
                team_name
            ),
            "Community Manager".to_string(),
            date.to_string(),
        )
        .with_category(MessageCategory::Media)
        .with_priority(MessagePriority::Normal)
        .with_sender_role("Community Manager")
        .with_action(action(
            "respond", "Respond", "be.msg.event.respond",
            ActionType::ChooseOption {
                options: vec![
                    ActionOption {
                        id: "listen_fans".to_string(),
                        label: "Engage with the fans".to_string(),
                        description: "Meet with fan representatives and explain the team's competitive direction. Good for morale.".to_string(),
                        label_key: Some("be.msg.fanPetition.options.listenFans.label".to_string()),
                        description_key: Some("be.msg.fanPetition.options.listenFans.description".to_string()),
                    },
                    ActionOption {
                        id: "ignore_fans".to_string(),
                        label: "Focus on prep".to_string(),
                        description: "Politely decline — draft and roster decisions stay with the competitive staff.".to_string(),
                        label_key: Some("be.msg.fanPetition.options.ignoreFans.label".to_string()),
                        description_key: Some("be.msg.fanPetition.options.ignoreFans.description".to_string()),
                    },
                    ActionOption {
                        id: "address_publicly".to_string(),
                        label: "Make a public statement".to_string(),
                        description: "Address the petition in media. Transparent and proactive.".to_string(),
                        label_key: Some("be.msg.fanPetition.options.addressPublicly.label".to_string()),
                        description_key: Some("be.msg.fanPetition.options.addressPublicly.description".to_string()),
                    },
                ],
            },
        ))
        .with_i18n(
            "be.msg.fanPetitionLol.subject",
            &template.template_key,
            params(&[("team", team_name), ("effectId", &template.effect_id)]),
        )
        .with_sender_i18n("be.sender.communityManager", "be.role.communityManager"),
    )
}

pub(super) fn rival_interest_message(
    msg_id: &str,
    player_id: &str,
    player_name: &str,
    rival_name: &str,
    date: &str,
) -> InboxMessage {
    let mut rng = rand::rng();
    let variations = [
        format!(
            "We've received word that {} have started making moves for {}.\n\n\
            Their staff were seen reviewing recent series and solo queue data, and our sources \
            suggest a formal approach may arrive soon.\n\n\
            How would you like us to respond if they make contact?",
            rival_name, player_name
        ),
        format!(
            "Esports media are reporting that {} is now a concrete target for {}.\n\n\
            According to sources, the player has attracted attention after recent stage performances. \
            No formal bid yet, but pressure is building fast.\n\n\
            What's your stance?",
            player_name, rival_name
        ),
    ];
    let idx = rng.random_range(0..variations.len());

    InboxMessage::new(
        msg_id.to_string(),
        format!("Roster Rumour — {} linked with {}", player_name, rival_name),
        variations[idx].clone(),
        "Director of Football".to_string(),
        date.to_string(),
    )
    .with_category(MessageCategory::Transfer)
    .with_priority(MessagePriority::Normal)
    .with_sender_role("Director of Football")
    .with_action(action(
        "respond",
        "Respond",
        "be.msg.event.respond",
        ActionType::ChooseOption {
            options: vec![
                ActionOption {
                    id: "not_for_sale".to_string(),
                    label: "Not for sale".to_string(),
                    description: "Make it clear the player is going nowhere. Boosts their morale."
                        .to_string(),
                    label_key: Some("be.msg.rivalInterest.options.notForSale.label".to_string()),
                    description_key: Some(
                        "be.msg.rivalInterest.options.notForSale.description".to_string(),
                    ),
                },
                ActionOption {
                    id: "open_to_offers".to_string(),
                    label: "Open to offers".to_string(),
                    description: "Signal willingness to negotiate. Player may become unsettled."
                        .to_string(),
                    label_key: Some("be.msg.rivalInterest.options.openToOffers.label".to_string()),
                    description_key: Some(
                        "be.msg.rivalInterest.options.openToOffers.description".to_string(),
                    ),
                },
                ActionOption {
                    id: "no_comment".to_string(),
                    label: "No comment".to_string(),
                    description: "Stay quiet and let things play out. Neutral stance.".to_string(),
                    label_key: Some("be.msg.rivalInterest.options.noComment.label".to_string()),
                    description_key: Some(
                        "be.msg.rivalInterest.options.noComment.description".to_string(),
                    ),
                },
            ],
        },
    ))
    .with_context(MessageContext {
        player_id: Some(player_id.to_string()),
        ..Default::default()
    })
    .with_i18n(
        "be.msg.rivalInterest.subject",
        &format!("be.msg.rivalInterest.body{}", idx),
        params(&[("player", player_name), ("rival", rival_name)]),
    )
    .with_sender_i18n("be.sender.directorOfFootball", "be.role.directorOfFootball")
}
