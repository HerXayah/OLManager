use ofm_core::narrative::{
    NarrativeContentPack, NarrativeSelector, load_default_content_pack, validate_content_pack,
};

#[test]
fn default_lol_narrative_pack_loads_source_controlled_events_and_conversations() {
    let pack = load_default_content_pack().expect("default LoL social narrative pack should load");

    assert_eq!(pack.schema_version, 1);
    assert!(
        pack.events
            .iter()
            .any(|event| event.id == "fan-clean-win-objectives"
                && event.effect_id == "press_squad_morale_small_up"
                && event.tags == vec!["win", "neutral_objectives"]),
        "expected source-controlled LoL fan/media event with stable effect id"
    );
    assert!(
        pack.conversations
            .iter()
            .any(|conversation| conversation.id == "player-pressure-reset"
                && conversation.effect_id == "player_conversation_pressure_reset"
                && conversation.tags == vec!["underperformance", "pressure"]),
        "expected source-controlled LoL player conversation with stable effect id"
    );
    assert!(
        pack.news.iter().any(|template| template.id == "league-roundup-maps"
            && template.template_key == "be.news.roundup.body"
            && template.tags == vec!["roundup", "maps"]),
        "expected source-controlled LoL news/social template metadata"
    );
}

#[test]
fn invalid_narrative_pack_reports_missing_references_and_unsafe_real_tones() {
    let json = r#"
    {
      "schemaVersion": 1,
      "outlets": [{ "id": "desk", "name": "Desk", "scope": { "type": "general" }, "weight": 1 }],
      "personas": [{ "id": "real-host", "displayName": "Real Host", "outletId": "missing-outlet", "type": "real", "allowedTones": ["spicy"], "scope": { "type": "league", "leagueIds": [] }, "weight": 0 }],
      "effects": [{ "id": "known-effect", "target": "player", "moraleDelta": -2 }],
      "events": [{ "id": "bad-event", "templateKey": "event.key", "scope": { "type": "general" }, "personaIds": ["missing-persona"], "effectId": "missing-effect", "tags": ["win"], "weight": 1 }],
      "conversations": [{ "id": "bad-convo", "templateKey": "conversation.key", "scope": { "type": "league", "leagueIds": [] }, "effectId": "missing-effect", "tags": ["pressure"], "weight": 0 }],
      "news": [{ "id": "bad-news", "templateKey": "news.key", "scope": { "type": "league", "leagueIds": [] }, "tags": ["roundup"], "weight": 0 }]
    }
    "#;
    let pack: NarrativeContentPack = serde_json::from_str(json).expect("fixture should parse");

    let errors = validate_content_pack(&pack).expect_err("invalid pack should fail fast");

    assert!(
        errors.iter().any(|error| error.contains("personas[0].outletId references missing outlet 'missing-outlet'")),
        "expected missing outlet error, got {errors:?}"
    );
    assert!(
        errors.iter().any(|error| error.contains("personas[0].allowedTones contains unsafe tone 'spicy' for real persona 'real-host'")),
        "expected unsafe real persona tone error, got {errors:?}"
    );
    assert!(
        errors.iter().any(|error| error.contains("events[0].personaIds[0] references missing persona 'missing-persona'")),
        "expected missing event persona error, got {errors:?}"
    );
    assert!(
        errors.iter().any(|error| error.contains("events[0].effectId references missing effect 'missing-effect'")),
        "expected missing event effect error, got {errors:?}"
    );
    assert!(
        errors.iter().any(|error| error.contains("conversations[0].weight must be greater than 0")),
        "expected invalid conversation weight error, got {errors:?}"
    );
    assert!(
        errors.iter().any(|error| error.contains("news[0].scope.leagueIds must include at least one league id for league scope")),
        "expected invalid news scope error, got {errors:?}"
    );
}

#[test]
fn selectors_filter_scope_persona_tone_and_tags_before_weighted_pick() {
    let pack = load_default_content_pack().expect("default LoL social narrative pack should load");
    let selector = NarrativeSelector::new(&pack);

    let eligible_event = selector
        .select_event(
            Some("default"),
            &["win", "neutral_objectives"],
            &["spicy", "pressure"],
        )
        .expect("event should match league, tags, and fictional persona tone policy");
    assert_eq!(eligible_event.id, "fan-clean-win-objectives");
    assert_eq!(eligible_event.effect_id, "press_squad_morale_small_up");

    let no_false_premise = selector.select_event(Some("default"), &["loss"], &["spicy"]);
    assert!(
        no_false_premise.is_none(),
        "win/objective event must be excluded when required tags are absent"
    );

    let eligible_conversation = selector
        .select_conversation(Some("default"), &["underperformance", "pressure"])
        .expect("conversation should match league scope and required tags");
    assert_eq!(eligible_conversation.id, "player-pressure-reset");
    assert_eq!(eligible_conversation.effect_id, "player_conversation_pressure_reset");

    let wrong_league = selector.select_conversation(Some("other-league"), &["underperformance", "pressure"]);
    assert!(
        wrong_league.is_none(),
        "league-scoped conversation must not leak into unrelated leagues"
    );
}
