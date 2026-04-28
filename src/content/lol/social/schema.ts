export type PersonaType = "real" | "fictional" | "inspired";
export type SocialTone =
  | "professional"
  | "analytical"
  | "community"
  | "close"
  | "dramatic"
  | "spicy"
  | "pressure";

export type ContentScope =
  | { type: "general" }
  | { type: "league"; leagueIds: string[] };

export interface OutletProfile {
  id: string;
  name: string;
  scope: ContentScope;
  weight: number;
}

export interface PersonaProfile {
  id: string;
  displayName: string;
  outletId: string;
  type: PersonaType;
  allowedTones: SocialTone[];
  scope: ContentScope;
  weight: number;
}

export interface EffectDefinition {
  id: string;
  target: "squad" | "player" | "none";
  moraleDelta?: number;
  playerFlag?: string;
}

export interface SocialResponse {
  id: string;
  labelKey: string;
  textKey: string;
  tone: SocialTone;
  effectId: string;
  target: "squad" | "player" | "none";
}

export interface SocialQuestion {
  id: string;
  textKey: string;
  scope: ContentScope;
  personaIds: string[];
  tones: SocialTone[];
  requiredTags?: string[];
  excludedTags?: string[];
  requiredFacts?: string[];
  excludedFacts?: string[];
  responseIds: string[];
  weight: number;
}

export interface SocialEventTemplate {
  id: string;
  templateKey: string;
  scope: ContentScope;
  personaIds: string[];
  effectId: string;
  tags: string[];
  weight: number;
}

export interface SocialConversationTemplate {
  id: string;
  templateKey: string;
  scope: ContentScope;
  effectId: string;
  tags: string[];
  weight: number;
}

export interface SocialNewsTemplate {
  id: string;
  templateKey: string;
  scope: ContentScope;
  tags: string[];
  weight: number;
}

export interface SocialContentPack {
  schemaVersion: 1;
  outlets: OutletProfile[];
  personas: PersonaProfile[];
  effects: EffectDefinition[];
  responses: SocialResponse[];
  questions: SocialQuestion[];
  events: SocialEventTemplate[];
  conversations: SocialConversationTemplate[];
  news: SocialNewsTemplate[];
}

export interface ValidationResult {
  ok: boolean;
  errors: string[];
}

const REAL_PERSONA_SAFE_TONES: ReadonlySet<SocialTone> = new Set([
  "professional",
  "analytical",
  "community",
  "close",
]);

function idSet<T extends { id: string }>(items: T[]): Set<string> {
  return new Set(items.map((item) => item.id));
}

function validateUniqueIds<T extends { id: string }>(
  collectionName: string,
  items: T[],
  errors: string[],
): void {
  const seen = new Set<string>();

  items.forEach((item, index) => {
    if (seen.has(item.id)) {
      errors.push(`${collectionName}[${index}].id duplicates '${item.id}'`);
      return;
    }

    seen.add(item.id);
  });
}

function validateWeight(path: string, weight: number, errors: string[]): void {
  if (!Number.isFinite(weight) || weight <= 0) {
    errors.push(`${path}.weight must be greater than 0`);
  }
}

function validateScope(path: string, scope: ContentScope, errors: string[]): void {
  if (scope.type === "league" && scope.leagueIds.length === 0) {
    errors.push(
      `${path}.scope.leagueIds must include at least one league id for league scope`,
    );
  }
}

function validateKnownRefs(
  path: string,
  values: string[],
  known: Set<string>,
  refName: string,
  errors: string[],
): void {
  values.forEach((value, index) => {
    if (!known.has(value)) {
      errors.push(`${path}.${refName}[${index}] references missing ${refName.slice(0, -3)} '${value}'`);
    }
  });
}

function validateEffectRef(
  path: string,
  effectId: string,
  effectIds: Set<string>,
  errors: string[],
): void {
  if (!effectIds.has(effectId)) {
    errors.push(`${path}.effectId references missing effect '${effectId}'`);
  }
}

function validateRealPersonaTones(
  path: string,
  persona: PersonaProfile,
  errors: string[],
): void {
  if (persona.type !== "real") {
    return;
  }

  persona.allowedTones.forEach((tone) => {
    if (!REAL_PERSONA_SAFE_TONES.has(tone)) {
      errors.push(
        `${path}.allowedTones contains unsafe tone '${tone}' for real persona '${persona.id}'`,
      );
    }
  });
}

function validateQuestionTonePolicy(
  path: string,
  question: SocialQuestion,
  personasById: Map<string, PersonaProfile>,
  errors: string[],
): void {
  question.personaIds.forEach((personaId) => {
    const persona = personasById.get(personaId);

    if (!persona || persona.type !== "real") {
      return;
    }

    question.tones.forEach((tone) => {
      if (!REAL_PERSONA_SAFE_TONES.has(tone)) {
        errors.push(
          `${path}.tones contains unsafe tone '${tone}' for real persona '${persona.id}'`,
        );
      }
    });
  });
}

export function validateSocialContent(pack: SocialContentPack): ValidationResult {
  const errors: string[] = [];
  const outletIds = idSet(pack.outlets);
  const personaIds = idSet(pack.personas);
  const effectIds = idSet(pack.effects);
  const responseIds = idSet(pack.responses);
  const personasById = new Map(pack.personas.map((persona) => [persona.id, persona]));

  validateUniqueIds("outlets", pack.outlets, errors);
  validateUniqueIds("personas", pack.personas, errors);
  validateUniqueIds("effects", pack.effects, errors);
  validateUniqueIds("responses", pack.responses, errors);
  validateUniqueIds("questions", pack.questions, errors);
  validateUniqueIds("events", pack.events, errors);
  validateUniqueIds("conversations", pack.conversations, errors);
  validateUniqueIds("news", pack.news, errors);

  pack.outlets.forEach((outlet, index) => {
    validateWeight(`outlets[${index}]`, outlet.weight, errors);
    validateScope(`outlets[${index}]`, outlet.scope, errors);
  });

  pack.personas.forEach((persona, index) => {
    const path = `personas[${index}]`;
    validateWeight(path, persona.weight, errors);
    validateScope(path, persona.scope, errors);
    if (!outletIds.has(persona.outletId)) {
      errors.push(`${path}.outletId references missing outlet '${persona.outletId}'`);
    }
    validateRealPersonaTones(path, persona, errors);
  });

  pack.questions.forEach((question, index) => {
    const path = `questions[${index}]`;
    validateWeight(path, question.weight, errors);
    validateScope(path, question.scope, errors);
    validateKnownRefs(path, question.personaIds, personaIds, "personaIds", errors);
    validateKnownRefs(path, question.responseIds, responseIds, "responseIds", errors);
    validateQuestionTonePolicy(path, question, personasById, errors);
  });

  pack.responses.forEach((response, index) => {
    validateEffectRef(`responses[${index}]`, response.effectId, effectIds, errors);
  });

  pack.events.forEach((event, index) => {
    const path = `events[${index}]`;
    validateWeight(path, event.weight, errors);
    validateScope(path, event.scope, errors);
    validateKnownRefs(path, event.personaIds, personaIds, "personaIds", errors);
    validateEffectRef(path, event.effectId, effectIds, errors);
  });

  pack.conversations.forEach((conversation, index) => {
    const path = `conversations[${index}]`;
    validateWeight(path, conversation.weight, errors);
    validateScope(path, conversation.scope, errors);
    validateEffectRef(path, conversation.effectId, effectIds, errors);
  });

  pack.news.forEach((template, index) => {
    const path = `news[${index}]`;
    validateWeight(path, template.weight, errors);
    validateScope(path, template.scope, errors);
  });

  return { ok: errors.length === 0, errors };
}
