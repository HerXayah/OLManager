import { describe, expect, it } from "vitest";

import type { FacilitiesData, SponsorshipData, TeamData } from "../store/types";
import {
  FACILITY_MODULE_DEFINITIONS,
  getClubInstallationContract,
  getSponsorshipContractView,
} from "./lolFinanceContracts";

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha Squad",
    short_name: "ALP",
    country: "ES",
    city: "Madrid",
    stadium_name: "Alpha Arena",
    stadium_capacity: 25000,
    finance: 150000,
    manager_id: null,
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 100000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-2-3-1",
    play_style: "Balanced",
    training_focus: "Scrims",
    training_intensity: "Medium",
    training_schedule: "Balanced",
    founded_year: 2020,
    colors: {
      primary: "#111111",
      secondary: "#ffffff",
    },
    facilities: {
      training: 2,
      medical: 1,
      scouting: 3,
    },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

describe("lolFinanceContracts", () => {
  it("uses one canonical module catalog to derive installation contracts", () => {
    const team = createTeam({
      facilities: {
        main_hub_level: 4,
        training: 3,
        medical: 2,
        scouting: 1,
      } as FacilitiesData,
    });

    const contract = getClubInstallationContract(team);

    expect(FACILITY_MODULE_DEFINITIONS).toEqual([
      expect.objectContaining({ key: "scrimsRoom", labelKey: "finances.facilityScrimsRoom" }),
      expect.objectContaining({ key: "analysisRoom", labelKey: "finances.facilityAnalysisRoom" }),
      expect.objectContaining({ key: "bootcampArea", labelKey: "finances.facilityBootcampArea" }),
      expect.objectContaining({ key: "recoverySuite", labelKey: "finances.facilityRecoverySuite" }),
      expect.objectContaining({ key: "contentStudio", labelKey: "finances.facilityContentStudio" }),
      expect.objectContaining({ key: "scoutingLab", labelKey: "finances.facilityScoutingLab" }),
    ]);
    expect(contract).toEqual(
      FACILITY_MODULE_DEFINITIONS.map((definition) =>
        expect.objectContaining({
          key: definition.key,
          label: definition.label,
          labelKey: definition.labelKey,
          effectKey: definition.effectKey,
        }),
      ),
    );
  });

  it("maps legacy facilities into the installation contract view", () => {
    const team = createTeam();

    expect(getClubInstallationContract(team)).toEqual([
      {
        key: "scrimsRoom",
        label: "Scrims Room",
        labelKey: "finances.facilityScrimsRoom",
        effectKey: "finances.facilityScrimsRoomEffect",
        level: 2,
        monthlyUpkeep: 20_000,
        upgradeFacility: "ScrimsRoom",
      },
      {
        key: "analysisRoom",
        label: "Analysis Room",
        labelKey: "finances.facilityAnalysisRoom",
        effectKey: "finances.facilityAnalysisRoomEffect",
        level: 1,
        monthlyUpkeep: 0,
        upgradeFacility: "AnalysisRoom",
      },
      {
        key: "bootcampArea",
        label: "Bootcamp Area",
        labelKey: "finances.facilityBootcampArea",
        effectKey: "finances.facilityBootcampAreaEffect",
        level: 1,
        monthlyUpkeep: 0,
        upgradeFacility: "BootcampArea",
      },
      {
        key: "recoverySuite",
        label: "Recovery Suite",
        labelKey: "finances.facilityRecoverySuite",
        effectKey: "finances.facilityRecoverySuiteEffect",
        level: 1,
        monthlyUpkeep: 0,
        upgradeFacility: "RecoverySuite",
      },
      {
        key: "contentStudio",
        label: "Content Studio",
        labelKey: "finances.facilityContentStudio",
        effectKey: "finances.facilityContentStudioEffect",
        level: 3,
        monthlyUpkeep: 0,
        upgradeFacility: "ContentStudio",
      },
      {
        key: "scoutingLab",
        label: "Scouting Lab",
        labelKey: "finances.facilityScoutingLab",
        effectKey: "finances.facilityScoutingLabEffect",
        level: 3,
        monthlyUpkeep: 20_000,
        upgradeFacility: "ScoutingLab",
      },
    ]);
  });

  it("prefers the installations alias when present", () => {
    const installations: FacilitiesData = {
      training: 4,
      medical: 2,
      scouting: 5,
    };

    const team = createTeam({ installations });

    expect(getClubInstallationContract(team)).toEqual([
      expect.objectContaining({ key: "scrimsRoom", level: 4 }),
      expect.objectContaining({ key: "analysisRoom", level: 1 }),
      expect.objectContaining({ key: "bootcampArea", level: 1 }),
      expect.objectContaining({ key: "recoverySuite", level: 2 }),
      expect.objectContaining({ key: "contentStudio", level: 5 }),
      expect.objectContaining({ key: "scoutingLab", level: 5 }),
    ]);
  });

  it("accepts partial legacy facilities data with safe module defaults", () => {
    const team = createTeam({ facilities: { training: 4 } as FacilitiesData });

    expect(getClubInstallationContract(team)).toEqual([
      expect.objectContaining({ key: "scrimsRoom", level: 4 }),
      expect.objectContaining({ key: "analysisRoom", level: 1 }),
      expect.objectContaining({ key: "bootcampArea", level: 1 }),
      expect.objectContaining({ key: "recoverySuite", level: 1 }),
      expect.objectContaining({ key: "contentStudio", level: 4 }),
      expect.objectContaining({ key: "scoutingLab", level: 1 }),
    ]);
  });

  it("uses an explicit main hub level when the backend sends the expanded hub field", () => {
    const team = createTeam({
      facilities: {
        main_hub_level: 4,
        training: 2,
        medical: 1,
        scouting: 1,
      } as FacilitiesData,
    });

    expect(getClubInstallationContract(team)).toEqual([
      expect.objectContaining({ key: "scrimsRoom", level: 2 }),
      expect.objectContaining({ key: "analysisRoom", level: 1 }),
      expect.objectContaining({ key: "bootcampArea", level: 1 }),
      expect.objectContaining({ key: "recoverySuite", level: 1 }),
      expect.objectContaining({ key: "contentStudio", level: 4 }),
      expect.objectContaining({ key: "scoutingLab", level: 1 }),
    ]);
  });

  it("keeps old saved facilities blobs compatible with the hub-facing installation contract", () => {
    const legacySaveFacilities = JSON.parse('{"training":5,"medical":2}') as FacilitiesData;
    const team = createTeam({ facilities: legacySaveFacilities });

    expect(getClubInstallationContract(team)).toEqual([
      expect.objectContaining({ key: "scrimsRoom", level: 5, monthlyUpkeep: 80_000 }),
      expect.objectContaining({ key: "analysisRoom", level: 1, monthlyUpkeep: 0 }),
      expect.objectContaining({ key: "bootcampArea", level: 1, monthlyUpkeep: 0 }),
      expect.objectContaining({ key: "recoverySuite", level: 2, monthlyUpkeep: 10_000 }),
      expect.objectContaining({ key: "contentStudio", level: 5, monthlyUpkeep: 0 }),
      expect.objectContaining({ key: "scoutingLab", level: 1, monthlyUpkeep: 0 }),
    ]);
  });

  it("prefers new saved hub expansion data without inventing upgraded legacy modules", () => {
    const newSaveFacilities = JSON.parse(
      '{"main_hub_level":4,"training":2,"medical":1,"scouting":1}',
    ) as FacilitiesData;
    const team = createTeam({ facilities: newSaveFacilities });

    expect(getClubInstallationContract(team)).toEqual([
      expect.objectContaining({ key: "scrimsRoom", level: 2 }),
      expect.objectContaining({ key: "analysisRoom", level: 1 }),
      expect.objectContaining({ key: "bootcampArea", level: 1 }),
      expect.objectContaining({ key: "recoverySuite", level: 1 }),
      expect.objectContaining({ key: "contentStudio", level: 4 }),
      expect.objectContaining({ key: "scoutingLab", level: 1 }),
    ]);
  });

  it("summarises a sponsorship contract without losing bonus data shape", () => {
    const sponsorship: SponsorshipData = {
      sponsor_name: "HyperX eSports",
      base_value: 120000,
      remaining_weeks: 10,
      bonus_criteria: [{ kind: "unbeaten-run", bonus_amount: 30000 }],
    };

    expect(getSponsorshipContractView(sponsorship)).toEqual({
      sponsorName: "HyperX eSports",
      baseValue: 120000,
      remainingWeeks: 10,
      bonusCount: 1,
      theme: "esports",
      themeLabel: "Esports sponsor",
    });
    expect(getSponsorshipContractView(null)).toBeNull();
  });

  it("marks non-technical sponsors as standard themed contracts", () => {
    const sponsorship: SponsorshipData = {
      sponsor_name: "Local Bank",
      base_value: 50000,
      remaining_weeks: 4,
      bonus_criteria: [],
    };

    expect(getSponsorshipContractView(sponsorship)).toEqual({
      sponsorName: "Local Bank",
      baseValue: 50000,
      remainingWeeks: 4,
      bonusCount: 0,
      theme: "standard",
      themeLabel: "Standard sponsor",
    });
  });
});
