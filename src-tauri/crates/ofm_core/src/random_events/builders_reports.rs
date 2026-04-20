use super::{action, params};
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
    let mut rng = rand::rng();
    let petitions = [
        (
            "Fan Petition — More Attacking Football",
            format!(
                "A group of {} supporters has organized a petition calling for more attacking football.\n\n\
                \"We pay good money to watch exciting football. We want to see the team go forward and entertain us!\"\n\n\
                Over 500 signatures so far. How do you respond?",
                team_name
            ),
        ),
        (
            "Fan Petition — Give Youth a Chance",
            format!(
                "Supporters of {} have started a campaign urging you to give more opportunities to young players from the academy.\n\n\
                \"The future of our club depends on developing homegrown talent. Stop overlooking the kids!\"\n\n\
                It's getting traction on social media. What's your response?",
                team_name
            ),
        ),
        (
            "Fan Open Letter — Transparency",
            format!(
                "An open letter from the {} Supporters Trust has been published, asking for more transparency from the management.\n\n\
                \"We want to understand the club's vision. Where are we heading? What's the long-term plan?\"\n\n\
                The local press is covering it. How do you handle this?",
                team_name
            ),
        ),
    ];
    let idx = rng.random_range(0..petitions.len());
    let (subject, body) = &petitions[idx];

    InboxMessage::new(
        msg_id.to_string(),
        subject.to_string(),
        body.clone(),
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
                    label: "Focus on football".to_string(),
                    description: "Politely decline — football decisions stay in the dressing room.".to_string(),
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
        &format!("be.msg.fanPetition.subject{}", idx),
        &format!("be.msg.fanPetition.body{}", idx),
        params(&[("team", team_name)]),
    )
    .with_sender_i18n("be.sender.communityManager", "be.role.communityManager")
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
            "We've received word that {} have been making enquiries about {}.\n\n\
            Their scouts were spotted at our last few matches, and our sources suggest \
            they may approach with a formal offer soon.\n\n\
            How would you like us to respond if they make contact?",
            rival_name, player_name
        ),
        format!(
            "The press are reporting that {} is a target for {}.\n\n\
            According to sources, the player has attracted attention after their recent performances. \
            No formal bid yet, but it's only a matter of time.\n\n\
            What's your stance?",
            player_name, rival_name
        ),
    ];
    let idx = rng.random_range(0..variations.len());

    InboxMessage::new(
        msg_id.to_string(),
        format!(
            "Transfer Rumour — {} linked with {}",
            player_name, rival_name
        ),
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
