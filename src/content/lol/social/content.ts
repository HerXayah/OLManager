import conversations from "./conversations.json";
import effects from "./effects.json";
import events from "./events.json";
import news from "./news.json";
import outlets from "./outlets.json";
import personas from "./personas.json";
import questions from "./questions.json";
import responses from "./responses.json";
import type {
  EffectDefinition,
  OutletProfile,
  PersonaProfile,
  SocialContentPack,
  SocialConversationTemplate,
  SocialEventTemplate,
  SocialNewsTemplate,
  SocialQuestion,
  SocialResponse,
} from "./schema";

export const SOCIAL_CONTENT_PACK: SocialContentPack = {
  schemaVersion: 1,
  outlets: outlets as OutletProfile[],
  personas: personas as PersonaProfile[],
  effects: effects as EffectDefinition[],
  responses: responses as SocialResponse[],
  questions: questions as SocialQuestion[],
  events: events as SocialEventTemplate[],
  conversations: conversations as SocialConversationTemplate[],
  news: news as SocialNewsTemplate[],
};
