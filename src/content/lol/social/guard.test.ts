import { describe, expect, it } from "vitest";
import { SOCIAL_CONTENT_PACK } from "./content";
import { scanFootballEraTerms } from "./guard";

describe("scanFootballEraTerms", () => {
  it("finds no football-era terms in the scoped social/media/news pack", () => {
    expect(scanFootballEraTerms(SOCIAL_CONTENT_PACK)).toEqual([]);
  });

  it("ignores compatibility and domain fields documented as legacy allowlist", () => {
    const result = scanFootballEraTerms({
      compatibility: "Legacy footballHerald alias kept for migration compatibility",
      domain: {
        name: "football-era historical tag",
      },
      visible: "League beat and split coverage remain esports-focused.",
    });

    expect(result).toEqual([]);
  });

  it("reports football-era terms in scoped social/news records", () => {
    expect(
      scanFootballEraTerms({
        visible: "The coach and goalkeeper meet at the stadium pitch.",
      }),
    ).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ path: "root.visible" }),
      ]),
    );
  });
});
