import { describe, expect, it } from "vitest";
import {
  filterEligibleOutlets,
  filterEligiblePersonas,
  filterEligibleQuestions,
  filterEligibleResponses,
  selectWeighted,
} from "./selectors";
import type {
  OutletProfile,
  PersonaProfile,
  SocialQuestion,
  SocialResponse,
} from "./schema";

const outlets: OutletProfile[] = [
  { id: "general-outlet", name: "Rift Desk", scope: { type: "general" }, weight: 1 },
  {
    id: "lec-outlet",
    name: "LEC Beat",
    scope: { type: "league", leagueIds: ["lec"] },
    weight: 10,
  },
  {
    id: "lck-outlet",
    name: "LCK Beat",
    scope: { type: "league", leagueIds: ["lck"] },
    weight: 100,
  },
];

const personas: PersonaProfile[] = [
  {
    id: "real-analyst",
    displayName: "Verified Analyst",
    outletId: "general-outlet",
    type: "real",
    allowedTones: ["professional", "analytical"],
    scope: { type: "general" },
    weight: 1,
  },
  {
    id: "lec-instigator",
    displayName: "LEC Instigator",
    outletId: "lec-outlet",
    type: "fictional",
    allowedTones: ["spicy", "pressure"],
    scope: { type: "league", leagueIds: ["lec"] },
    weight: 5,
  },
  {
    id: "lck-instigator",
    displayName: "LCK Instigator",
    outletId: "lck-outlet",
    type: "fictional",
    allowedTones: ["spicy"],
    scope: { type: "league", leagueIds: ["lck"] },
    weight: 100,
  },
];

const questions: SocialQuestion[] = [
  {
    id: "objective-win",
    textKey: "questions.objectiveWin",
    scope: { type: "general" },
    personaIds: ["real-analyst"],
    tones: ["professional"],
    requiredTags: ["win", "neutral_objectives"],
    excludedTags: ["loss"],
    responseIds: ["measured"],
    weight: 1,
  },
  {
    id: "false-comeback",
    textKey: "questions.falseComeback",
    scope: { type: "general" },
    personaIds: ["real-analyst"],
    tones: ["professional"],
    requiredTags: ["comeback"],
    responseIds: ["measured"],
    weight: 1000,
  },
  {
    id: "false-bot-praise",
    textKey: "questions.falseBotPraise",
    scope: { type: "general" },
    personaIds: ["real-analyst"],
    tones: ["professional"],
    requiredTags: ["win"],
    excludedTags: ["botlane_underperformed"],
    responseIds: ["measured"],
    weight: 1000,
  },
  {
    id: "lec-pressure-loss",
    textKey: "questions.lecPressureLoss",
    scope: { type: "league", leagueIds: ["lec"] },
    personaIds: ["lec-instigator"],
    tones: ["pressure"],
    requiredTags: ["loss", "underperformance"],
    responseIds: ["pressure"],
    weight: 20,
  },
  {
    id: "fact-aware-pressure",
    textKey: "questions.factAwarePressure",
    scope: { type: "general" },
    personaIds: ["real-analyst"],
    tones: ["professional"],
    requiredFacts: ["mvpPlayerId"],
    excludedFacts: ["worstPlayerId"],
    responseIds: ["measured"],
    weight: 2,
  },
];

const responses: SocialResponse[] = [
  {
    id: "measured",
    labelKey: "responses.measured.label",
    textKey: "responses.measured.text",
    tone: "professional",
    effectId: "press_no_effect",
    target: "none",
  },
  {
    id: "pressure",
    labelKey: "responses.pressure.label",
    textKey: "responses.pressure.text",
    tone: "pressure",
    effectId: "press_player_pressure_small_down",
    target: "player",
  },
  {
    id: "spicy-unreferenced",
    labelKey: "responses.spicy.label",
    textKey: "responses.spicy.text",
    tone: "spicy",
    effectId: "press_no_effect",
    target: "none",
  },
];

describe("social content selectors", () => {
  it("filters media outlets and personas by league and tone before weighted selection", () => {
    const eligibleOutlets = filterEligibleOutlets(outlets, { leagueId: "lec" });
    const eligiblePersonas = filterEligiblePersonas(personas, {
      leagueId: "lec",
      outletIds: eligibleOutlets.map((outlet) => outlet.id),
      allowedTones: ["pressure"],
    });

    expect(eligibleOutlets.map((outlet) => outlet.id)).toEqual([
      "general-outlet",
      "lec-outlet",
    ]);
    expect(eligiblePersonas.map((persona) => persona.id)).toEqual(["lec-instigator"]);
    expect(selectWeighted(eligiblePersonas, () => 0.99)?.id).toBe("lec-instigator");
  });

  it("excludes false-premise questions before weighted randomization can see high-weight invalid templates", () => {
    const eligibleQuestions = filterEligibleQuestions(questions, {
      leagueId: "lec",
      personaId: "real-analyst",
      allowedTones: ["professional"],
      contextTags: ["win", "neutral_objectives", "botlane_underperformed"],
    });

    expect(eligibleQuestions.map((question) => question.id)).toEqual([
      "objective-win",
    ]);
    expect(selectWeighted(eligibleQuestions, () => 0.99)?.id).toBe("objective-win");
  });

  it("filters responses by the selected question response IDs and allowed tones", () => {
    const eligibleResponses = filterEligibleResponses(responses, {
      responseIds: ["measured", "pressure"],
      allowedTones: ["pressure"],
    });

    expect(eligibleResponses.map((response) => response.id)).toEqual(["pressure"]);
  });

  it("treats context facts as additional compatibility signals for newer question templates", () => {
    const eligibleQuestions = filterEligibleQuestions(questions, {
      leagueId: "lec",
      personaId: "real-analyst",
      allowedTones: ["professional"],
      contextTags: ["win", "neutral_objectives"],
      contextFacts: {
        mvpPlayerId: "blue-mid",
      },
    });

    expect(eligibleQuestions.map((question) => question.id)).toEqual([
      "objective-win",
      "false-bot-praise",
      "fact-aware-pressure",
    ]);
  });
});
