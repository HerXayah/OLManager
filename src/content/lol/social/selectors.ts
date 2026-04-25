import type {
  ContentScope,
  OutletProfile,
  PersonaProfile,
  SocialQuestion,
  SocialResponse,
  SocialTone,
} from "./schema";

export interface LeagueFilter {
  leagueId?: string;
}

export interface PersonaFilter extends LeagueFilter {
  outletIds?: string[];
  allowedTones?: SocialTone[];
}

export interface QuestionFilter extends LeagueFilter {
  personaId?: string;
  allowedTones?: SocialTone[];
  contextTags: string[];
}

export interface ResponseFilter {
  responseIds: string[];
  allowedTones?: SocialTone[];
}

interface WeightedItem {
  weight: number;
}

function scopeMatches(scope: ContentScope, leagueId?: string): boolean {
  if (scope.type === "general") return true;
  return Boolean(leagueId && scope.leagueIds.includes(leagueId));
}

function intersects<T>(left: readonly T[], right: readonly T[]): boolean {
  return left.some((value) => right.includes(value));
}

function includesAll(values: ReadonlySet<string>, required: readonly string[] = []): boolean {
  return required.every((tag) => values.has(tag));
}

function includesNone(values: ReadonlySet<string>, excluded: readonly string[] = []): boolean {
  return excluded.every((tag) => !values.has(tag));
}

export function filterEligibleOutlets(
  outlets: OutletProfile[],
  filter: LeagueFilter,
): OutletProfile[] {
  return outlets.filter((outlet) => scopeMatches(outlet.scope, filter.leagueId));
}

export function filterEligiblePersonas(
  personas: PersonaProfile[],
  filter: PersonaFilter,
): PersonaProfile[] {
  const outletIds = filter.outletIds ? new Set(filter.outletIds) : null;

  return personas.filter((persona) => {
    if (!scopeMatches(persona.scope, filter.leagueId)) return false;
    if (outletIds && !outletIds.has(persona.outletId)) return false;
    if (filter.allowedTones && !intersects(persona.allowedTones, filter.allowedTones)) return false;
    return true;
  });
}

export function filterEligibleQuestions(
  questions: SocialQuestion[],
  filter: QuestionFilter,
): SocialQuestion[] {
  const contextTagSet = new Set(filter.contextTags);

  return questions.filter((question) => {
    if (!scopeMatches(question.scope, filter.leagueId)) return false;
    if (filter.personaId && !question.personaIds.includes(filter.personaId)) return false;
    if (filter.allowedTones && !intersects(question.tones, filter.allowedTones)) return false;
    if (!includesAll(contextTagSet, question.requiredTags)) return false;
    if (!includesNone(contextTagSet, question.excludedTags)) return false;
    return true;
  });
}

export function filterEligibleResponses(
  responses: SocialResponse[],
  filter: ResponseFilter,
): SocialResponse[] {
  const responseIds = new Set(filter.responseIds);

  return responses.filter((response) => {
    if (!responseIds.has(response.id)) return false;
    if (filter.allowedTones && !filter.allowedTones.includes(response.tone)) return false;
    return true;
  });
}

export function selectWeighted<T extends WeightedItem>(
  items: T[],
  random: () => number = Math.random,
): T | null {
  if (items.length === 0) return null;

  const totalWeight = items.reduce((sum, item) => sum + Math.max(0, item.weight), 0);
  if (totalWeight <= 0) return null;

  let needle = random() * totalWeight;
  for (const item of items) {
    needle -= Math.max(0, item.weight);
    if (needle <= 0) return item;
  }

  return items[items.length - 1] ?? null;
}
