use ofm_core::random_events::{
    build_fan_petition_from_narrative, build_media_story_from_narrative,
};

#[test]
fn positive_media_story_uses_lol_esports_registry_framing() {
    let message = build_media_story_from_narrative(
        "media_today",
        "G2 Esports",
        "player-1",
        "Caps",
        true,
        "2026-04-25",
    )
    .expect("positive media story should resolve through narrative registry");

    assert!(
        message.subject.contains("Rift") || message.subject.contains("Draft"),
        "expected LoL media headline, got {}",
        message.subject
    );
    assert!(
        message.body.contains("objective")
            || message.body.contains("draft")
            || message.body.contains("solo queue")
            || message.body.contains("Rift"),
        "expected LoL esports body, got {}",
        message.body
    );
    assert_eq!(
        message.i18n_params.get("effectId"),
        Some(&"press_squad_morale_small_up".to_string())
    );
    assert_eq!(message.context.player_id.as_deref(), Some("player-1"));
}

#[test]
fn negative_media_story_preserves_player_context_with_lol_pressure_copy() {
    let message = build_media_story_from_narrative(
        "media_today",
        "Fnatic",
        "player-2",
        "Oscarinin",
        false,
        "2026-04-25",
    )
    .expect("negative media story should resolve through narrative registry");

    assert!(
        message.subject.contains("Pressure") || message.subject.contains("Draft"),
        "expected LoL pressure headline, got {}",
        message.subject
    );
    assert!(
        message.body.contains("pressure")
            || message.body.contains("lane")
            || message.body.contains("scrim")
            || message.body.contains("Rift"),
        "expected LoL pressure body, got {}",
        message.body
    );
    assert_eq!(
        message.i18n_params.get("effectId"),
        Some(&"press_player_pressure_small_down".to_string())
    );
    assert_eq!(message.context.player_id.as_deref(), Some("player-2"));
}

#[test]
fn fan_petition_uses_lol_supporter_campaign_copy_and_preserves_response_options() {
    let message =
        build_fan_petition_from_narrative("fan_petition_2026-04-25", "G2 Esports", "2026-04-25")
            .expect("fan petition should resolve through narrative registry");

    assert!(
        message.subject.contains("Fan Petition") && message.subject.contains("Draft"),
        "expected LoL supporter petition subject, got {}",
        message.subject
    );
    assert!(
        message.body.contains("draft")
            && message.body.contains("objective setups")
            && message.body.contains("Rift"),
        "expected LoL supporter campaign body, got {}",
        message.body
    );
    assert!(
        !message.body.contains("football") && !message.body.contains("dressing room"),
        "fan petition still has football framing: {}",
        message.body
    );
    assert_eq!(
        message.i18n_params.get("effectId"),
        Some(&"press_squad_morale_small_up".to_string())
    );

    let options = match &message.actions[0].action_type {
        domain::message::ActionType::ChooseOption { options } => options,
        other => panic!("expected fan petition choice options, got {other:?}"),
    };
    assert_eq!(options[0].id, "listen_fans");
    assert_eq!(options[1].id, "ignore_fans");
    assert_eq!(options[2].id, "address_publicly");
}
