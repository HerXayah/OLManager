import { describe, expect, it } from "vitest";
import {
  type SocialContentPack,
  validateSocialContent,
} from "./schema";

const validPack = (): SocialContentPack => ({
  schemaVersion: 1,
  outlets: [
    {
      id: "outlet-general",
      name: "Rift Desk",
      scope: { type: "general" },
      weight: 1,
    },
    {
      id: "outlet-league",
      name: "LEC Beat",
      scope: { type: "league", leagueIds: ["lec"] },
      weight: 2,
    },
  ],
  personas: [
    {
      id: "real-analyst",
      displayName: "Verified Analyst",
      outletId: "outlet-general",
      type: "real",
      allowedTones: ["professional", "analytical"],
      scope: { type: "general" },
      weight: 1,
    },
    {
      id: "fictional-spicy",
      displayName: "The Rift Instigator",
      outletId: "outlet-league",
      type: "fictional",
      allowedTones: ["spicy", "pressure"],
      scope: { type: "league", leagueIds: ["lec"] },
      weight: 1,
    },
  ],
  effects: [
    {
      id: "press_squad_morale_small_up",
      target: "squad",
      moraleDelta: 3,
    },
    {
      id: "press_player_pressure_small_down",
      target: "player",
      moraleDelta: -2,
    },
  ],
  responses: [
    {
      id: "response-credit-players",
      labelKey: "content.lol.social.responses.creditPlayers.label",
      textKey: "content.lol.social.responses.creditPlayers.text",
      tone: "professional",
      effectId: "press_squad_morale_small_up",
      target: "squad",
    },
    {
      id: "response-demand-reset",
      labelKey: "content.lol.social.responses.demandReset.label",
      textKey: "content.lol.social.responses.demandReset.text",
      tone: "pressure",
      effectId: "press_player_pressure_small_down",
      target: "player",
    },
  ],
  questions: [
    {
      id: "question-clean-win",
      textKey: "content.lol.social.questions.cleanWin.text",
      scope: { type: "general" },
      personaIds: ["real-analyst"],
      tones: ["professional", "analytical"],
      requiredTags: ["win", "stomp"],
      excludedTags: ["loss"],
      responseIds: ["response-credit-players"],
      weight: 3,
    },
    {
      id: "question-pressure-loss",
      textKey: "content.lol.social.questions.pressureLoss.text",
      scope: { type: "league", leagueIds: ["lec"] },
      personaIds: ["fictional-spicy"],
      tones: ["spicy", "pressure"],
      requiredTags: ["loss", "underperformance"],
      responseIds: ["response-demand-reset"],
      weight: 1,
    },
  ],
  events: [
    {
      id: "event-fan-clean-win",
      templateKey: "content.lol.social.events.fanCleanWin.body",
      scope: { type: "general" },
      personaIds: ["fictional-spicy"],
      effectId: "press_squad_morale_small_up",
      tags: ["win"],
      weight: 1,
    },
  ],
  conversations: [
    {
      id: "conversation-player-pressure",
      templateKey: "content.lol.social.conversations.playerPressure.body",
      scope: { type: "league", leagueIds: ["lec"] },
      effectId: "press_player_pressure_small_down",
      tags: ["underperformance"],
      weight: 1,
    },
  ],
  news: [
    {
      id: "news-roundup-maps",
      templateKey: "be.news.roundup.body",
      scope: { type: "general" },
      tags: ["roundup", "maps"],
      weight: 1,
    },
  ],
});

describe("validateSocialContent", () => {
  it("accepts valid packs with general and league-scoped personas, templates, and effect refs", () => {
    expect(validateSocialContent(validPack())).toEqual({ ok: true, errors: [] });
  });

  it("rejects missing outlet, persona, response, and effect references with actionable paths", () => {
    const pack = validPack();
    pack.personas[0] = { ...pack.personas[0], outletId: "missing-outlet" };
    pack.questions[0] = {
      ...pack.questions[0],
      personaIds: ["missing-persona"],
      responseIds: ["missing-response"],
    };
    pack.responses[0] = { ...pack.responses[0], effectId: "missing-effect" };
    pack.events[0] = { ...pack.events[0], effectId: "missing-event-effect" };
    pack.conversations[0] = {
      ...pack.conversations[0],
      effectId: "missing-conversation-effect",
    };

    expect(validateSocialContent(pack)).toEqual({
      ok: false,
      errors: [
        "personas[0].outletId references missing outlet 'missing-outlet'",
        "questions[0].personaIds[0] references missing persona 'missing-persona'",
        "questions[0].responseIds[0] references missing response 'missing-response'",
        "responses[0].effectId references missing effect 'missing-effect'",
        "events[0].effectId references missing effect 'missing-event-effect'",
        "conversations[0].effectId references missing effect 'missing-conversation-effect'",
      ],
    });
  });

  it("rejects invalid weights and empty league scopes", () => {
    const pack = validPack();
    pack.outlets[0] = { ...pack.outlets[0], weight: 0 };
    pack.personas[1] = {
      ...pack.personas[1],
      scope: { type: "league", leagueIds: [] },
    };
    pack.questions[1] = { ...pack.questions[1], weight: -1 };
    pack.events[0] = { ...pack.events[0], weight: Number.NaN };
    pack.news[0] = {
      ...pack.news[0],
      scope: { type: "league", leagueIds: [] },
      weight: 0,
    };

    expect(validateSocialContent(pack).errors).toEqual([
      "outlets[0].weight must be greater than 0",
      "personas[1].scope.leagueIds must include at least one league id for league scope",
      "questions[1].weight must be greater than 0",
      "events[0].weight must be greater than 0",
      "news[0].weight must be greater than 0",
      "news[0].scope.leagueIds must include at least one league id for league scope",
    ]);
  });

  it("rejects real personas with spicy or pressure tones and questions that assign unsafe tones to real personas", () => {
    const pack = validPack();
    pack.personas[0] = {
      ...pack.personas[0],
      allowedTones: ["professional", "spicy"],
    };
    pack.questions[0] = {
      ...pack.questions[0],
      tones: ["pressure"],
      personaIds: ["real-analyst"],
    };

    expect(validateSocialContent(pack).errors).toEqual([
      "personas[0].allowedTones contains unsafe tone 'spicy' for real persona 'real-analyst'",
      "questions[0].tones contains unsafe tone 'pressure' for real persona 'real-analyst'",
    ]);
  });

  it("rejects duplicate IDs across each content collection", () => {
    const pack = validPack();
    pack.effects.push({ ...pack.effects[0] });
    pack.responses.push({ ...pack.responses[0] });
    pack.news.push({ ...pack.news[0] });

    expect(validateSocialContent(pack).errors).toEqual([
      "effects[2].id duplicates 'press_squad_morale_small_up'",
      "responses[2].id duplicates 'response-credit-players'",
      "news[1].id duplicates 'news-roundup-maps'",
    ]);
  });
});
