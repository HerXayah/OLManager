use ofm_core::player_events::build_player_conversation_from_narrative;

#[test]
fn low_morale_conversation_uses_lol_competitive_registry_copy_and_effect_id() {
    let message = build_player_conversation_from_narrative(
        "morale_talk_player-1",
        "low_morale",
        "player-1",
        "Caps",
        Some(24),
        "2026-04-25",
    )
    .expect("low morale conversation should resolve through narrative registry");

    assert_eq!(message.subject, "Caps — Mental Reset Needed");
    assert!(
        message.body.contains("scrim block")
            && message.body.contains("stage confidence")
            && message.body.contains("team comms"),
        "expected LoL competitive morale copy, got {}",
        message.body
    );
    assert!(!message.body.contains("dressing room"));
    assert_eq!(
        message.body_key.as_deref(),
        Some("content.lol.social.conversations.playerPressureReset.body")
    );
    assert_eq!(
        message.i18n_params.get("effectId"),
        Some(&"player_conversation_pressure_reset".to_string())
    );
    assert_eq!(message.context.player_id.as_deref(), Some("player-1"));
}

#[test]
fn bench_happy_and_contract_conversations_preserve_actions_without_football_framing() {
    let bench = build_player_conversation_from_narrative(
        "bench_complaint_player-2",
        "bench_complaint",
        "player-2",
        "Oscarinin",
        None,
        "2026-04-25",
    )
    .expect("bench conversation should resolve through narrative registry");
    let happy = build_player_conversation_from_narrative(
        "happy_player_player-3",
        "happy_player",
        "player-3",
        "Mikyx",
        None,
        "2026-04-25",
    )
    .expect("happy conversation should resolve through narrative registry");
    let contract = build_player_conversation_from_narrative(
        "contract_concern_player-4_final",
        "contract_concern",
        "player-4",
        "Hans sama",
        Some(19),
        "2026-04-25",
    )
    .expect("contract conversation should resolve through narrative registry");

    assert!(bench.body.contains("stage reps") && bench.body.contains("scrims"));
    assert!(happy.body.contains("solo queue") && happy.body.contains("draft room"));
    assert!(contract.body.contains("league window") && contract.body.contains("roster plan"));

    for message in [&bench, &happy, &contract] {
        assert!(
            !message.body.contains("football")
                && !message.body.contains("pitch")
                && !message.body.contains("dressing room"),
            "conversation still has football framing: {}",
            message.body
        );
        let action = message
            .actions
            .first()
            .expect("conversation should remain actionable");
        assert_eq!(action.id, "respond");
    }

    let bench_options = match &bench.actions[0].action_type {
        domain::message::ActionType::ChooseOption { options } => options,
        other => panic!("expected choice options, got {other:?}"),
    };
    assert_eq!(bench_options[0].id, "explain");
    assert_eq!(bench_options[1].id, "promise_chance");
    assert_eq!(bench_options[2].id, "prove_yourself");
    assert_eq!(
        contract.i18n_params.get("effectId"),
        Some(&"player_conversation_contract_pressure".to_string())
    );
}
