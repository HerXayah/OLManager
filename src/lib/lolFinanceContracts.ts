import type { FacilitiesData, SponsorshipData, TeamData } from "../store/types";

export type FacilityUpgradeId = "Training" | "Medical" | "Scouting";

type InstallationContractKey =
  | "scrimsRoom"
  | "analysisRoom"
  | "bootcampArea"
  | "recoverySuite"
  | "contentStudio"
  | "scoutingLab";

export interface InstallationContractView {
  key: InstallationContractKey;
  label: string;
  labelKey: string;
  effectKey: string;
  level: number;
  monthlyUpkeep: number;
  upgradeFacility: FacilityUpgradeId | null;
}

export interface SponsorshipContractView {
  sponsorName: string;
  baseValue: number;
  remainingWeeks: number;
  bonusCount: number;
  theme: "standard" | "esports";
  themeLabel: string;
}

const DEFAULT_FACILITIES: FacilitiesData = {
  training: 1,
  medical: 1,
  scouting: 1,
};

export const FACILITY_MODULE_DEFINITIONS: Array<{
  key: InstallationContractKey;
  label: string;
  labelKey: string;
  effectKey: string;
  levelKey: keyof FacilitiesData | "hub";
  upkeepPerExtraLevel: number;
  upgradeFacility: FacilityUpgradeId | null;
}> = [
  {
    key: "scrimsRoom",
    label: "Scrims Room",
    labelKey: "finances.facilityScrimsRoom",
    effectKey: "finances.facilityScrimsRoomEffect",
    levelKey: "training",
    upkeepPerExtraLevel: 20_000,
    upgradeFacility: "Training",
  },
  {
    key: "analysisRoom",
    label: "Analysis Room",
    labelKey: "finances.facilityAnalysisRoom",
    effectKey: "finances.facilityAnalysisRoomEffect",
    levelKey: "training",
    upkeepPerExtraLevel: 15_000,
    upgradeFacility: "Training",
  },
  {
    key: "bootcampArea",
    label: "Bootcamp Area",
    labelKey: "finances.facilityBootcampArea",
    effectKey: "finances.facilityBootcampAreaEffect",
    levelKey: "medical",
    upkeepPerExtraLevel: 15_000,
    upgradeFacility: "Medical",
  },
  {
    key: "recoverySuite",
    label: "Recovery Suite",
    labelKey: "finances.facilityRecoverySuite",
    effectKey: "finances.facilityRecoverySuiteEffect",
    levelKey: "medical",
    upkeepPerExtraLevel: 10_000,
    upgradeFacility: "Medical",
  },
  {
    key: "contentStudio",
    label: "Content Studio",
    labelKey: "finances.facilityContentStudio",
    effectKey: "finances.facilityContentStudioEffect",
    levelKey: "hub",
    upkeepPerExtraLevel: 0,
    upgradeFacility: null,
  },
  {
    key: "scoutingLab",
    label: "Scouting Lab",
    labelKey: "finances.facilityScoutingLab",
    effectKey: "finances.facilityScoutingLabEffect",
    levelKey: "scouting",
    upkeepPerExtraLevel: 10_000,
    upgradeFacility: "Scouting",
  },
];

function getInstallationMonthlyUpkeep(level: number, upkeepPerExtraLevel: number): number {
  return Math.max(0, level - 1) * upkeepPerExtraLevel;
}

function getSponsorTheme(sponsorName: string): SponsorshipContractView["theme"] {
  const normalized = sponsorName.toLowerCase();

  if (
    normalized.includes("esport") ||
    normalized.includes("gaming") ||
    normalized.includes("pc") ||
    normalized.includes("hardware") ||
    normalized.includes("tech")
  ) {
    return "esports";
  }

  return "standard";
}

function resolveInstallationLevels(team: TeamData): FacilitiesData {
  const source = team.installations ?? team.facilities ?? DEFAULT_FACILITIES;

  return {
    main_hub_level: source.main_hub_level,
    training: source.training ?? DEFAULT_FACILITIES.training,
    medical: source.medical ?? DEFAULT_FACILITIES.medical,
    scouting: source.scouting ?? DEFAULT_FACILITIES.scouting,
  };
}

function getMainHubLevel(levels: FacilitiesData): number {
  return Math.max(
    DEFAULT_FACILITIES.training,
    levels.main_hub_level ?? DEFAULT_FACILITIES.training,
    levels.training,
    levels.medical,
    levels.scouting,
  );
}

export function getClubInstallationContract(team: TeamData): InstallationContractView[] {
  const levels = resolveInstallationLevels(team);
  const hubLevel = getMainHubLevel(levels);

  return FACILITY_MODULE_DEFINITIONS.map((definition) => {
    const level = definition.levelKey === "hub" ? hubLevel : levels[definition.levelKey];

    return {
      key: definition.key,
      label: definition.label,
      labelKey: definition.labelKey,
      effectKey: definition.effectKey,
      level,
      monthlyUpkeep: getInstallationMonthlyUpkeep(level, definition.upkeepPerExtraLevel),
      upgradeFacility: definition.upgradeFacility,
    };
  });
}

export function getSponsorshipContractView(
  sponsorship: SponsorshipData | null | undefined,
): SponsorshipContractView | null {
  if (!sponsorship) {
    return null;
  }

  return {
    sponsorName: sponsorship.sponsor_name,
    baseValue: sponsorship.base_value,
    remainingWeeks: sponsorship.remaining_weeks,
    bonusCount: sponsorship.bonus_criteria.length,
    theme: getSponsorTheme(sponsorship.sponsor_name),
    themeLabel:
      getSponsorTheme(sponsorship.sponsor_name) === "esports"
        ? "Esports sponsor"
        : "Standard sponsor",
  };
}
