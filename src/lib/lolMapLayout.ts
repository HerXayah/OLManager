export const LOL_MAP_NATIVE_SIZE = 1024;
export const LOL_MAP_STRUCTURE_ICON_SIZE = 40;
export const LOL_MAP_OBJECTIVE_ICON_SIZE = 48;
export const LOL_MAP_JUNGLE_ICON_SIZE = 36;

export type JungleCampIcon = "blue" | "red" | "camp";
export type StructureIcon = "tower_blue" | "tower_red" | "inhibitor_blue" | "inhibitor_red" | "nexus_blue" | "nexus_red";
export type NeutralObjectiveIcon =
  | "dragon"
  | "dragon_infernal"
  | "dragon_ocean"
  | "dragon_mountain"
  | "dragon_cloud"
  | "dragon_hextech"
  | "dragon_chemtech"
  | "dragon_elder"
  | "baron"
  | "grub"
  | "riftherald";

export interface JungleCampPoint {
  id: string;
  side: "blue" | "red" | "neutral";
  label: string;
  icon: JungleCampIcon;
  x: number;
  y: number;
}

export interface StructurePoint {
  id: string;
  side: "blue" | "red";
  lane: "top" | "mid" | "bot" | "base";
  label: string;
  icon: StructureIcon;
  x: number;
  y: number;
}

export interface NeutralObjectivePoint {
  id: string;
  label: string;
  icon: NeutralObjectiveIcon;
  x: number;
  y: number;
}

export interface WallVertex {
  x: number;
  y: number;
}

export interface WallPolygon {
  id: string;
  label: string;
  closed: boolean;
  points: WallVertex[];
}

// Coordinates normalized 0..1 for a classic Summoner's Rift orientation:
// blue base bottom-left, red base top-right.
export const JUNGLE_CAMPS_LAYOUT: JungleCampPoint[] = [
  { id: "blue-blue-buff", side: "blue", label: "Blue Buff", icon: "blue", x: 0.24934895833333334, y: 0.4622395833333333 },
  { id: "blue-gromp", side: "blue", label: "Gromp", icon: "camp", x: 0.14908854166666666, y: 0.43359375 },
  { id: "blue-wolves", side: "blue", label: "Wolves", icon: "camp", x: 0.2584635416666667, y: 0.56640625 },
  { id: "blue-raptors", side: "blue", label: "Raptors", icon: "camp", x: 0.4759114583333333, y: 0.6432291666666666 },
  { id: "blue-red-buff", side: "blue", label: "Red Buff", icon: "red", x: 0.5266927083333334, y: 0.7421875 },
  { id: "blue-krugs", side: "blue", label: "Krugs", icon: "camp", x: 0.568359375, y: 0.828125 },

  { id: "red-blue-buff", side: "red", label: "Blue Buff", icon: "blue", x: 0.478515625, y: 0.26171875 },
  { id: "red-gromp", side: "red", label: "Gromp", icon: "camp", x: 0.4381510416666667, y: 0.16536458333333334 },
  { id: "red-wolves", side: "red", label: "Wolves", icon: "camp", x: 0.525390625, y: 0.3528645833333333 },
  { id: "red-raptors", side: "red", label: "Raptors", icon: "camp", x: 0.748046875, y: 0.4361979166666667 },
  { id: "red-red-buff", side: "red", label: "Red Buff", icon: "red", x: 0.7545572916666666, y: 0.5403645833333334 },
  { id: "red-krugs", side: "red", label: "Krugs", icon: "camp", x: 0.8483072916666666, y: 0.56640625 },

  // River scuttles (neutral)
  { id: "river-scuttle-top", side: "neutral", label: "Scuttle Top", icon: "camp", x: 0.2845052083333333, y: 0.34765625 },
  { id: "river-scuttle-bot", side: "neutral", label: "Scuttle Bot", icon: "camp", x: 0.6998697916666666, y: 0.6419270833333334 },
];

export const JUNGLE_CAMP_ICON_PATH: Record<JungleCampIcon, string> = {
  blue: "/lol-map-icons/blue.png",
  red: "/lol-map-icons/red.png",
  camp: "/lol-map-icons/camp.png",
};

// Coordinates normalized from map_references.png (1200x1200).
export const STRUCTURES_LAYOUT: StructurePoint[] = [
  // Blue side (bottom-left)
  { id: "blue-top-outer", side: "blue", lane: "top", label: "Blue Top Outer", icon: "tower_blue", x: 0.072265625, y: 0.2838541666666667 },
  { id: "blue-top-inner", side: "blue", lane: "top", label: "Blue Top Inner", icon: "tower_blue", x: 0.099609375, y: 0.5533854166666666 },
  { id: "blue-top-inhib-tower", side: "blue", lane: "top", label: "Blue Top Inhib Tower", icon: "tower_blue", x: 0.09049479166666667, y: 0.69921875 },
  { id: "blue-mid-outer", side: "blue", lane: "mid", label: "Blue Mid Outer", icon: "tower_blue", x: 0.4016927083333333, y: 0.5755208333333334 },
  { id: "blue-mid-inner", side: "blue", lane: "mid", label: "Blue Mid Inner", icon: "tower_blue", x: 0.3470052083333333, y: 0.6705729166666666 },
  { id: "blue-mid-inhib-tower", side: "blue", lane: "mid", label: "Blue Mid Inhib Tower", icon: "tower_blue", x: 0.2623697916666667, y: 0.7408854166666666 },
  { id: "blue-bot-inner", side: "blue", lane: "bot", label: "Blue Bot Inner", icon: "tower_blue", x: 0.4720052083333333, y: 0.8958333333333334 },
  { id: "blue-bot-outer", side: "blue", lane: "bot", label: "Blue Bot Outer", icon: "tower_blue", x: 0.720703125, y: 0.9231770833333334 },
  { id: "blue-bot-inhib-tower", side: "blue", lane: "bot", label: "Blue Bot Inhib Tower", icon: "tower_blue", x: 0.298828125, y: 0.9127604166666666 },
  { id: "blue-inhib-top", side: "blue", lane: "base", label: "Blue Top Inhib", icon: "inhibitor_blue", x: 0.08658854166666667, y: 0.7591145833333334 },
  { id: "blue-inhib-mid", side: "blue", lane: "base", label: "Blue Mid Inhib", icon: "inhibitor_blue", x: 0.224609375, y: 0.7864583333333334 },
  { id: "blue-inhib-bot", side: "blue", lane: "base", label: "Blue Bot Inhib", icon: "inhibitor_blue", x: 0.24544270833333334, y: 0.9114583333333334 },
  { id: "blue-nexus-top-tower", side: "blue", lane: "base", label: "Blue Nexus Top Tower", icon: "tower_blue", x: 0.126953125, y: 0.8372395833333334 },
  { id: "blue-nexus-bot-tower", side: "blue", lane: "base", label: "Blue Nexus Bot Tower", icon: "tower_blue", x: 0.15950520833333334, y: 0.875 },
  { id: "blue-nexus", side: "blue", lane: "base", label: "Blue Nexus", icon: "nexus_blue", x: 0.115234375, y: 0.8815104166666666 },

  // Red side (top-right)
  { id: "red-top-outer", side: "red", lane: "top", label: "Red Top Outer", icon: "tower_red", x: 0.275390625, y: 0.07161458333333333 },
  { id: "red-top-inner", side: "red", lane: "top", label: "Red Top Inner", icon: "tower_red", x: 0.533203125, y: 0.08203125 },
  { id: "red-top-inhib-tower", side: "red", lane: "top", label: "Red Top Inhib Tower", icon: "tower_red", x: 0.7024739583333334, y: 0.09375 },
  { id: "red-mid-outer", side: "red", lane: "mid", label: "Red Mid Outer", icon: "tower_red", x: 0.595703125, y: 0.44140625 },
  { id: "red-mid-inner", side: "red", lane: "mid", label: "Red Mid Inner", icon: "tower_red", x: 0.6569010416666666, y: 0.33203125 },
  { id: "red-mid-inhib-tower", side: "red", lane: "mid", label: "Red Mid Inhib Tower", icon: "tower_red", x: 0.740234375, y: 0.26171875 },
  { id: "red-bot-inner", side: "red", lane: "bot", label: "Red Bot Inner", icon: "tower_red", x: 0.9016927083333334, y: 0.44921875 },
  { id: "red-bot-outer", side: "red", lane: "bot", label: "Red Bot Outer", icon: "tower_red", x: 0.9303385416666666, y: 0.7057291666666666 },
  { id: "red-bot-inhib-tower", side: "red", lane: "bot", label: "Red Bot Inhib Tower", icon: "tower_red", x: 0.912109375, y: 0.3125 },
  { id: "red-inhib-top", side: "red", lane: "base", label: "Red Top Inhib", icon: "inhibitor_red", x: 0.7545572916666666, y: 0.09114583333333333 },
  { id: "red-inhib-mid", side: "red", lane: "base", label: "Red Mid Inhib", icon: "inhibitor_red", x: 0.783203125, y: 0.22395833333333334 },
  { id: "red-inhib-bot", side: "red", lane: "base", label: "Red Bot Inhib", icon: "inhibitor_red", x: 0.9108072916666666, y: 0.24869791666666666 },
  { id: "red-nexus-top-tower", side: "red", lane: "base", label: "Red Nexus Top Tower", icon: "tower_red", x: 0.845703125, y: 0.1328125 },
  { id: "red-nexus-bot-tower", side: "red", lane: "base", label: "Red Nexus Bot Tower", icon: "tower_red", x: 0.8717447916666666, y: 0.1640625 },
  { id: "red-nexus", side: "red", lane: "base", label: "Red Nexus", icon: "nexus_red", x: 0.8912760416666666, y: 0.1171875 },
];

export const NEUTRAL_OBJECTIVES_LAYOUT: NeutralObjectivePoint[] = [
  { id: "baron-pit", label: "Baron", icon: "baron", x: 0.3274739583333333, y: 0.2981770833333333 },
  { id: "dragon-pit", label: "Dragon", icon: "dragon", x: 0.673828125, y: 0.703125 },
  { id: "grub-pit", label: "Void Grubs", icon: "grub", x: 0.3313802083333333, y: 0.2994791666666667 },
  { id: "herald-area", label: "Rift Herald", icon: "riftherald", x: 0.3274739583333333, y: 0.2942708333333333 },
];

export const STRUCTURE_ICON_PATH: Record<StructureIcon, string> = {
  tower_blue: "/lol-map-icons/tower_blue.png",
  tower_red: "/lol-map-icons/tower_red.png",
  inhibitor_blue: "/lol-map-icons/inhibitor_blue.png",
  inhibitor_red: "/lol-map-icons/inhibitor_red.png",
  nexus_blue: "/lol-map-icons/nexus_blue.png",
  nexus_red: "/lol-map-icons/nexus_red.png",
};

export const NEUTRAL_OBJECTIVE_ICON_PATH: Record<NeutralObjectiveIcon, string> = {
  dragon: "/lol-map-icons/dragon.png",
  dragon_infernal: "/lol-map-icons/dragon_infernal.png",
  dragon_ocean: "/lol-map-icons/dragon_ocean.png",
  dragon_mountain: "/lol-map-icons/dragon_mountain.png",
  dragon_cloud: "/lol-map-icons/dragon_cloud.png",
  dragon_hextech: "/lol-map-icons/dragon_hextech.png",
  dragon_chemtech: "/lol-map-icons/dragon_chemtech.png",
  dragon_elder: "/lol-map-icons/dragon_elder.png",
  baron: "/lol-map-icons/baron.png",
  grub: "/lol-map-icons/grub.png",
  riftherald: "/lol-map-icons/riftherald.png",
};

// Initial editable wall guides (normalized 0..1). These are calibration helpers.
export const WALLS_LAYOUT: WallPolygon[] = [
  {
    id: "river-wall-top",
    label: "Top River Wall",
    closed: true,
    points: [
      { x: 0.33, y: 0.22 },
      { x: 0.40, y: 0.24 },
      { x: 0.44, y: 0.30 },
      { x: 0.36, y: 0.31 },
    ],
  },
  {
    id: "river-wall-bot",
    label: "Bot River Wall",
    closed: true,
    points: [
      { x: 0.58, y: 0.61 },
      { x: 0.66, y: 0.63 },
      { x: 0.69, y: 0.70 },
      { x: 0.61, y: 0.71 },
    ],
  },
  {
    id: "baron-pit-wall",
    label: "Baron Pit Wall",
    closed: true,
    points: [
      { x: 0.26, y: 0.24 },
      { x: 0.33, y: 0.25 },
      { x: 0.35, y: 0.32 },
      { x: 0.28, y: 0.34 },
    ],
  },
  {
    id: "dragon-pit-wall",
    label: "Dragon Pit Wall",
    closed: true,
    points: [
      { x: 0.63, y: 0.64 },
      { x: 0.71, y: 0.66 },
      { x: 0.72, y: 0.73 },
      { x: 0.64, y: 0.75 },
    ],
  },
];
