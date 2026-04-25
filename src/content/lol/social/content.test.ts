import { describe, expect, it } from "vitest";
import { SOCIAL_CONTENT_PACK } from "./content";
import { validateSocialContent } from "./schema";

describe("SOCIAL_CONTENT_PACK", () => {
  it("assembles source-controlled JSON packs into a valid schemaVersion 1 registry", () => {
    expect(SOCIAL_CONTENT_PACK.schemaVersion).toBe(1);
    expect(SOCIAL_CONTENT_PACK.outlets.map((outlet) => outlet.id)).toEqual(
      expect.arrayContaining([
        "rift-desk",
        "league-beat",
        "academy-wire",
        "lec-spotlight",
      ]),
    );
    expect(SOCIAL_CONTENT_PACK.personas.map((persona) => ({
      id: persona.id,
      type: persona.type,
      allowedTones: persona.allowedTones,
    }))).toEqual(
      expect.arrayContaining([
        {
          id: "verified-analyst",
          type: "real",
          allowedTones: ["professional", "analytical"],
        },
        {
          id: "academy-voice",
          type: "inspired",
          allowedTones: ["community", "close"],
        },
        {
          id: "lec-instigator",
          type: "fictional",
          allowedTones: ["spicy", "pressure"],
        },
      ]),
    );
    expect(SOCIAL_CONTENT_PACK.news.map((template) => template.id)).toEqual(
      [
        "league-roundup-maps",
        "standings-power-rankings",
        "season-preview-split",
        "weekly-digest-social-buzz",
      ],
    );
    expect(validateSocialContent(SOCIAL_CONTENT_PACK)).toEqual({
      ok: true,
      errors: [],
    });
  });

  it("preserves gameplay effect IDs on every selectable response", () => {
    expect(
      SOCIAL_CONTENT_PACK.responses.map((response) => ({
        id: response.id,
        effectId: response.effectId,
        target: response.target,
      })),
    ).toEqual([
      {
        id: "credit-preparation",
        effectId: "press_squad_morale_small_up",
        target: "squad",
      },
      {
        id: "demand-reset",
        effectId: "press_player_pressure_small_down",
        target: "player",
      },
      { id: "stay-measured", effectId: "press_no_effect", target: "none" },
    ]);
  });

  it("preserves LoL news/social template metadata for backend news migration", () => {
    expect(
      SOCIAL_CONTENT_PACK.news.map((template) => ({
        id: template.id,
        templateKey: template.templateKey,
        tags: template.tags,
      })),
    ).toEqual(
      expect.arrayContaining([
        {
          id: "league-roundup-maps",
          templateKey: "be.news.roundup.body",
          tags: ["roundup", "maps"],
        },
      ]),
    );
  });
});
