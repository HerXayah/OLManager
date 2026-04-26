import { describe, expect, it } from "vitest";
import {
  calculateStaffRevealBudget,
  selectRivalMasteryKnowledgeForPlayer,
  selectStaffRevealEntries,
} from "./ChampionDraft";

function champion(id: string, name: string) {
  return {
    id,
    key: 1,
    name,
    image: `/${id}.png`,
    tags: [],
    roleHints: [],
  };
}

describe("ChampionDraft rival mastery knowledge", () => {
  it("caps staff reveal budget from 1 to 5 picks based only on meta discovery", () => {
    expect(calculateStaffRevealBudget(0.9)).toBe(1);
    expect(calculateStaffRevealBudget(0.975)).toBe(2);
    expect(calculateStaffRevealBudget(1.05)).toBe(3);
    expect(calculateStaffRevealBudget(1.125)).toBe(4);
    expect(calculateStaffRevealBudget(1.2)).toBe(5);
    expect(calculateStaffRevealBudget(2)).toBe(5);
  });

  it("does not backfill staff reveals when a revealed champion is banned or picked", () => {
    const reveals = selectStaffRevealEntries(
      [
        {
          champion: champion("kaisa", "Kai'Sa"),
          mastery: 95,
          playerName: "Noah",
          playerRole: "ADC",
          source: "staff",
        },
        {
          champion: champion("xayah", "Xayah"),
          mastery: 92,
          playerName: "Noah",
          playerRole: "ADC",
          source: "staff",
        },
        {
          champion: champion("zeri", "Zeri"),
          mastery: 90,
          playerName: "Noah",
          playerRole: "ADC",
          source: "staff",
        },
      ],
      2,
      new Set(["kaisa"]),
    );

    expect(reveals.map((entry) => entry.champion.id)).toEqual(["xayah"]);
  });

  it("does not promote another champion to insignia when the true signature is banned", () => {
    const result = selectRivalMasteryKnowledgeForPlayer(
      [
        {
          champion: champion("ezreal", "Ezreal"),
          mastery: 100,
          playerName: "Noah",
          playerRole: "ADC",
        },
        {
          champion: champion("kaisa", "Kai'Sa"),
          mastery: 92,
          playerName: "Noah",
          playerRole: "ADC",
        },
      ],
      new Set(["ezreal"]),
      new Set(),
      false,
    );

    expect(result.knownEntries).toEqual([]);
    expect(result.staffCandidates).toHaveLength(1);
    expect(result.staffCandidates[0]).toMatchObject({
      champion: expect.objectContaining({ id: "kaisa" }),
      source: "staff",
    });
  });

  it("marks non-signature revealed champions as scouting, not insignia", () => {
    const result = selectRivalMasteryKnowledgeForPlayer(
      [
        {
          champion: champion("ezreal", "Ezreal"),
          mastery: 100,
          playerName: "Noah",
          playerRole: "ADC",
        },
        {
          champion: champion("kaisa", "Kai'Sa"),
          mastery: 92,
          playerName: "Noah",
          playerRole: "ADC",
        },
      ],
      new Set(["ezreal"]),
      new Set(),
      true,
    );

    expect(result.knownEntries).toHaveLength(1);
    expect(result.knownEntries[0]).toMatchObject({
      champion: expect.objectContaining({ id: "kaisa" }),
      source: "scouting",
    });
  });
});
