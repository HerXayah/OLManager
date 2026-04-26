import { describe, expect, it } from "vitest";

import type {
  AcademyAcquisitionOptionData,
  AcademyAcquisitionOptionsResponseData,
  AcquireAcademyTeamRequestData,
  TeamData,
} from "./types";

describe("academy acquisition contracts", () => {
  it("models backend acquisition options with source identity and rebrand metadata", () => {
    const option: AcademyAcquisitionOptionData = {
      source_team_id: "mkoi-fenix",
      source_team_name: "Movistar KOI Fénix",
      source_team_short_name: "MKOI F",
      source_team_logo_url: "https://cdn.example/logo.png",
      erl_league_id: "nlc",
      league_name: "Northern League of Legends Championship",
      country: "ES",
      region: "Europe",
      reputation: 4,
      development_level: 3,
      acquisition_cost: 260000,
      assignment_rule: "Domestic",
      fallback_reason: null,
      rebrand_allowed: true,
      source_identity: {
        source_team_id: "mkoi-fenix",
        original_name: "Movistar KOI Fénix",
        original_short_name: "MKOI F",
        original_logo_url: "https://cdn.example/logo.png",
      },
    };

    const response: AcademyAcquisitionOptionsResponseData = {
      parent_team_id: "team-1",
      acquisition_allowed: true,
      blocked_reason: null,
      options: [option],
    };

    expect(response.options[0].source_identity.original_name).toBe("Movistar KOI Fénix");
    expect(response.options[0].rebrand_allowed).toBe(true);
  });

  it("models acquire request payloads with optional rebrand fields", () => {
    const request: AcquireAcademyTeamRequestData = {
      parent_team_id: "team-1",
      source_team_id: "mkoi-fenix",
      custom_name: "Movistar KOI Fénix Academy",
      custom_short_name: "MKOI F",
      custom_logo_url: "https://cdn.example/custom-logo.png",
    };

    expect(request.custom_short_name).toBe("MKOI F");
  });

  it("keeps academy metadata available on the team model", () => {
    const team: TeamData = {
      id: "academy-1",
      name: "Movistar KOI Fénix Academy",
      short_name: "MKOI F",
      country: "ES",
      city: "Madrid",
      stadium_name: "KOI Arena",
      stadium_capacity: 5000,
      finance: 250000,
      manager_id: null,
      reputation: 42,
      wage_budget: 30000,
      transfer_budget: 120000,
      season_income: 0,
      season_expenses: 0,
      formation: "4-4-2",
      play_style: "Balanced",
      training_focus: "General",
      training_intensity: "Balanced",
      training_schedule: "Balanced",
      founded_year: 2024,
      colors: { primary: "#111111", secondary: "#eeeeee" },
      starting_xi_ids: [],
      form: [],
      history: [],
      team_kind: "Academy",
      parent_team_id: "team-1",
      academy_team_id: null,
      academy: {
        lifecycle: "Active",
        acquisition: {
          source_team_id: "mkoi-fenix",
          original_name: "Movistar KOI Fénix",
          original_short_name: "MKOI F",
          original_logo_url: "https://cdn.example/logo.png",
          acquisition_cost: 260000,
          acquired_at: "2026-08-10T00:00:00Z",
        },
        branding: {
          current_name: "Movistar KOI Fénix Academy",
          current_short_name: "MKOI F",
          current_logo_url: "https://cdn.example/custom-logo.png",
        },
        source_identity: {
          source_team_id: "mkoi-fenix",
          original_name: "Movistar KOI Fénix",
          original_short_name: "MKOI F",
          original_logo_url: "https://cdn.example/logo.png",
        },
        erl_assignment: {
          erl_league_id: "nlc",
          assignment_rule: "Domestic",
          fallback_reason: null,
          reputation: 4,
          acquisition_cost: 260000,
          acquired_at: "2026-08-10T00:00:00Z",
        },
      },
    };

    expect(team.academy?.acquisition.source_team_id).toBe("mkoi-fenix");
  });
});
