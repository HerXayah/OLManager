use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeContentPack {
    pub schema_version: u8,
    pub outlets: Vec<OutletProfile>,
    pub personas: Vec<PersonaProfile>,
    pub effects: Vec<EffectDefinition>,
    #[serde(default)]
    pub events: Vec<SocialEventTemplate>,
    #[serde(default)]
    pub conversations: Vec<SocialConversationTemplate>,
    #[serde(default)]
    pub news: Vec<SocialNewsTemplate>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentScope {
    General,
    League {
        #[serde(rename = "leagueIds")]
        league_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OutletProfile {
    pub id: String,
    pub name: String,
    pub scope: ContentScope,
    pub weight: i32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersonaProfile {
    pub id: String,
    pub display_name: String,
    pub outlet_id: String,
    #[serde(rename = "type")]
    pub persona_type: PersonaType,
    pub allowed_tones: Vec<SocialTone>,
    pub scope: ContentScope,
    pub weight: i32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EffectDefinition {
    pub id: String,
    pub target: EffectTarget,
    pub morale_delta: Option<i16>,
    pub player_flag: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SocialEventTemplate {
    pub id: String,
    pub template_key: String,
    pub scope: ContentScope,
    pub persona_ids: Vec<String>,
    pub effect_id: String,
    pub tags: Vec<String>,
    pub weight: i32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SocialConversationTemplate {
    pub id: String,
    pub template_key: String,
    pub scope: ContentScope,
    pub effect_id: String,
    pub tags: Vec<String>,
    pub weight: i32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SocialNewsTemplate {
    pub id: String,
    pub template_key: String,
    pub scope: ContentScope,
    pub tags: Vec<String>,
    pub weight: i32,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PersonaType {
    Real,
    Fictional,
    Inspired,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum SocialTone {
    Professional,
    Analytical,
    Community,
    Close,
    Dramatic,
    Spicy,
    Pressure,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum EffectTarget {
    Squad,
    Player,
    None,
}

pub fn load_default_content_pack() -> Result<NarrativeContentPack, String> {
    let pack = NarrativeContentPack {
        schema_version: 1,
        outlets: parse_json(
            "outlets",
            include_str!("../../../../../src/content/lol/social/outlets.json"),
        )?,
        personas: parse_json(
            "personas",
            include_str!("../../../../../src/content/lol/social/personas.json"),
        )?,
        effects: parse_json(
            "effects",
            include_str!("../../../../../src/content/lol/social/effects.json"),
        )?,
        events: parse_json(
            "events",
            include_str!("../../../../../src/content/lol/social/events.json"),
        )?,
        conversations: parse_json(
            "conversations",
            include_str!("../../../../../src/content/lol/social/conversations.json"),
        )?,
        news: parse_json(
            "news",
            include_str!("../../../../../src/content/lol/social/news.json"),
        )?,
    };

    validate_content_pack(&pack).map_err(|errors| errors.join("; "))?;
    Ok(pack)
}

pub fn validate_content_pack(pack: &NarrativeContentPack) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let outlet_ids = ids(&pack.outlets);
    let persona_ids = ids(&pack.personas);
    let effect_ids = ids(&pack.effects);

    validate_collection(
        "outlets",
        &pack.outlets,
        &mut errors,
        |path, outlet, errors| {
            validate_weight(path, outlet.weight, errors);
            validate_scope(path, &outlet.scope, errors);
        },
    );
    validate_collection(
        "personas",
        &pack.personas,
        &mut errors,
        |path, persona, errors| {
            validate_weight(path, persona.weight, errors);
            validate_scope(path, &persona.scope, errors);
            if !outlet_ids.contains(&persona.outlet_id) {
                errors.push(format!(
                    "{path}.outletId references missing outlet '{}'",
                    persona.outlet_id
                ));
            }
            if persona.persona_type == PersonaType::Real {
                for tone in &persona.allowed_tones {
                    if !is_real_persona_safe_tone(*tone) {
                        errors.push(format!(
                            "{path}.allowedTones contains unsafe tone '{}' for real persona '{}'",
                            tone.as_json_value(),
                            persona.id
                        ));
                    }
                }
            }
        },
    );
    validate_collection(
        "effects",
        &pack.effects,
        &mut errors,
        |_path, _effect, _errors| {},
    );
    validate_collection(
        "events",
        &pack.events,
        &mut errors,
        |path, event, errors| {
            validate_weight(path, event.weight, errors);
            validate_scope(path, &event.scope, errors);
            for (index, persona_id) in event.persona_ids.iter().enumerate() {
                if !persona_ids.contains(persona_id) {
                    errors.push(format!(
                        "{path}.personaIds[{index}] references missing persona '{persona_id}'"
                    ));
                }
            }
            validate_effect_ref(path, &event.effect_id, &effect_ids, errors);
        },
    );
    validate_collection(
        "conversations",
        &pack.conversations,
        &mut errors,
        |path, conversation, errors| {
            validate_weight(path, conversation.weight, errors);
            validate_scope(path, &conversation.scope, errors);
            validate_effect_ref(path, &conversation.effect_id, &effect_ids, errors);
        },
    );
    validate_collection("news", &pack.news, &mut errors, |path, template, errors| {
        validate_weight(path, template.weight, errors);
        validate_scope(path, &template.scope, errors);
    });

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub struct NarrativeSelector<'a> {
    pack: &'a NarrativeContentPack,
}

impl<'a> NarrativeSelector<'a> {
    pub fn new(pack: &'a NarrativeContentPack) -> Self {
        Self { pack }
    }

    pub fn select_event(
        &self,
        league_id: Option<&str>,
        tags: &[&str],
        allowed_tones: &[&str],
    ) -> Option<&'a SocialEventTemplate> {
        self.pack.events.iter().find(|event| {
            scope_matches(&event.scope, league_id)
                && tags_match(&event.tags, tags)
                && event.persona_ids.iter().any(|persona_id| {
                    self.pack
                        .personas
                        .iter()
                        .find(|persona| persona.id == *persona_id)
                        .is_some_and(|persona| persona_allows_tone(persona, allowed_tones))
                })
        })
    }

    pub fn select_conversation(
        &self,
        league_id: Option<&str>,
        tags: &[&str],
    ) -> Option<&'a SocialConversationTemplate> {
        self.pack.conversations.iter().find(|conversation| {
            scope_matches(&conversation.scope, league_id) && tags_match(&conversation.tags, tags)
        })
    }
}

fn parse_json<T>(name: &str, json: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(json).map_err(|err| format!("failed to parse {name}: {err}"))
}

trait HasId {
    fn id(&self) -> &str;
}

impl HasId for OutletProfile {
    fn id(&self) -> &str {
        &self.id
    }
}
impl HasId for PersonaProfile {
    fn id(&self) -> &str {
        &self.id
    }
}
impl HasId for EffectDefinition {
    fn id(&self) -> &str {
        &self.id
    }
}
impl HasId for SocialEventTemplate {
    fn id(&self) -> &str {
        &self.id
    }
}
impl HasId for SocialConversationTemplate {
    fn id(&self) -> &str {
        &self.id
    }
}
impl HasId for SocialNewsTemplate {
    fn id(&self) -> &str {
        &self.id
    }
}

fn ids<T: HasId>(items: &[T]) -> HashSet<String> {
    items.iter().map(|item| item.id().to_string()).collect()
}

fn validate_collection<T, F>(name: &str, items: &[T], errors: &mut Vec<String>, mut validate: F)
where
    T: HasId,
    F: FnMut(&str, &T, &mut Vec<String>),
{
    let mut seen = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        let path = format!("{name}[{index}]");
        if !seen.insert(item.id().to_string()) {
            errors.push(format!("{path}.id duplicates '{}'", item.id()));
        }
        validate(&path, item, errors);
    }
}

fn validate_weight(path: &str, weight: i32, errors: &mut Vec<String>) {
    if weight <= 0 {
        errors.push(format!("{path}.weight must be greater than 0"));
    }
}

fn validate_scope(path: &str, scope: &ContentScope, errors: &mut Vec<String>) {
    if let ContentScope::League { league_ids } = scope
        && league_ids.is_empty()
    {
        errors.push(format!(
            "{path}.scope.leagueIds must include at least one league id for league scope"
        ));
    }
}

fn validate_effect_ref(
    path: &str,
    effect_id: &str,
    effect_ids: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    if !effect_ids.contains(effect_id) {
        errors.push(format!(
            "{path}.effectId references missing effect '{effect_id}'"
        ));
    }
}

fn is_real_persona_safe_tone(tone: SocialTone) -> bool {
    matches!(
        tone,
        SocialTone::Professional
            | SocialTone::Analytical
            | SocialTone::Community
            | SocialTone::Close
    )
}

impl SocialTone {
    fn as_json_value(self) -> &'static str {
        match self {
            SocialTone::Professional => "professional",
            SocialTone::Analytical => "analytical",
            SocialTone::Community => "community",
            SocialTone::Close => "close",
            SocialTone::Dramatic => "dramatic",
            SocialTone::Spicy => "spicy",
            SocialTone::Pressure => "pressure",
        }
    }
}

fn scope_matches(scope: &ContentScope, league_id: Option<&str>) -> bool {
    match scope {
        ContentScope::General => true,
        ContentScope::League { league_ids } => {
            league_id.is_some_and(|id| league_ids.iter().any(|league_id| league_id == id))
        }
    }
}

fn tags_match(required_tags: &[String], actual_tags: &[&str]) -> bool {
    required_tags
        .iter()
        .all(|required| actual_tags.iter().any(|actual| actual == required))
}

fn persona_allows_tone(persona: &PersonaProfile, allowed_tones: &[&str]) -> bool {
    persona.allowed_tones.iter().any(|tone| {
        allowed_tones
            .iter()
            .any(|allowed_tone| *allowed_tone == tone.as_json_value())
    })
}
