import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const DDRAGON_VERSION = "14.24.1";
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, "..");
const outDir = path.join(projectRoot, "public", "lol-item-icons");

const ITEM_NAME_BY_KEY = {
  sunfire_aegis: "Sunfire Aegis",
  warmogs_armor: "Warmog's Armor",
  iceborn_gauntlet: "Iceborn Gauntlet",
  randuins_omen: "Randuin's Omen",
  spirit_visage: "Spirit Visage",
  plated_steelcaps: "Plated Steelcaps",
  sundered_sky: "Sundered Sky",
  deaths_dance: "Death's Dance",
  steraks_gage: "Sterak's Gage",
  titanic_hydra: "Titanic Hydra",
  maw_of_malmortius: "Maw of Malmortius",
  mercurys_treads: "Mercury's Treads",
  black_cleaver: "Black Cleaver",
  hullbreaker: "Hullbreaker",
  dead_mans_plate: "Dead Man's Plate",
  voltaic_cyclosword: "Voltaic Cyclosword",
  opportunity: "Opportunity",
  immortal_shieldbow: "Immortal Shieldbow",
  seryldas_grudge: "Serylda's Grudge",
  profane_hydra: "Profane Hydra",
  boots_of_swiftness: "Boots of Swiftness",
  stormsurge: "Stormsurge",
  lich_bane: "Lich Bane",
  shadowflame: "Shadowflame",
  zhonyas_hourglass: "Zhonya's Hourglass",
  rabadons_deathcap: "Rabadon's Deathcap",
  sorcerers_shoes: "Sorcerer's Shoes",
  ludens_companion: "Luden's Companion",
  void_staff: "Void Staff",
  seraphs_embrace: "Seraph's Embrace",
  liandrys_torment: "Liandry's Torment",
  rylais_crystal_scepter: "Rylai's Crystal Scepter",
  cosmic_drive: "Cosmic Drive",
  bloodthirster: "Bloodthirster",
  infinity_edge: "Infinity Edge",
  mortal_reminder: "Mortal Reminder",
  rapid_firecannon: "Rapid Firecannon",
  phantom_dancer: "Phantom Dancer",
  berserkers_greaves: "Berserker's Greaves",
  blade_of_the_ruined_king: "Blade of The Ruined King",
  wits_end: "Wit's End",
  runaans_hurricane: "Runaan's Hurricane",
  guinsoos_rageblade: "Guinsoo's Rageblade",
  terminus: "Terminus",
  the_collector: "The Collector",
  edge_of_night: "Edge of Night",
  ionian_boots_of_lucidity: "Ionian Boots of Lucidity",
  trailblazer: "Trailblazer",
  zekes_convergence: "Zeke's Convergence",
  knights_vow: "Knight's Vow",
  locket_of_the_iron_solari: "Locket of the Iron Solari",
  thornmail: "Thornmail",
  mobility_boots: "Mobility Boots",
  shurelyas_battlesong: "Shurelya's Battlesong",
  ardent_censer: "Ardent Censer",
  moonstone_renewer: "Moonstone Renewer",
  redemption: "Redemption",
  staff_of_flowing_water: "Staff of Flowing Water",
  morellonomicon: "Morellonomicon",
  cryptbloom: "Cryptbloom",
};

function normalizeName(value) {
  return value
    .normalize("NFD")
    .replace(/[^\p{L}\p{N}]+/gu, "")
    .toLowerCase();
}

async function main() {
  await mkdir(outDir, { recursive: true });

  const itemDataUrl = `https://ddragon.leagueoflegends.com/cdn/${DDRAGON_VERSION}/data/en_US/item.json`;
  const itemDataResponse = await fetch(itemDataUrl);
  if (!itemDataResponse.ok) {
    throw new Error(`Failed to fetch item.json (${itemDataResponse.status})`);
  }

  const itemDataPayload = await itemDataResponse.json();
  const itemMap = itemDataPayload?.data ?? {};
  const normalizedNameToImage = new Map();
  for (const item of Object.values(itemMap)) {
    if (!item || typeof item !== "object") continue;
    const name = item?.name;
    const imageFull = item?.image?.full;
    if (typeof name !== "string" || typeof imageFull !== "string") continue;
    normalizedNameToImage.set(normalizeName(name), imageFull);
  }

  const missing = [];

  for (const [key, itemName] of Object.entries(ITEM_NAME_BY_KEY)) {
    const imageFull = normalizedNameToImage.get(normalizeName(itemName));
    if (!imageFull) {
      missing.push(`${key} => ${itemName}`);
      continue;
    }

    const iconUrl = `https://ddragon.leagueoflegends.com/cdn/${DDRAGON_VERSION}/img/item/${imageFull}`;
    const iconResponse = await fetch(iconUrl);
    if (!iconResponse.ok) {
      missing.push(`${key} => ${itemName} (download ${iconResponse.status})`);
      continue;
    }

    const buffer = Buffer.from(await iconResponse.arrayBuffer());
    const outPath = path.join(outDir, `${key}.png`);
    await writeFile(outPath, buffer);
  }

  if (missing.length > 0) {
    console.warn("Some items could not be resolved/downloaded:");
    for (const line of missing) console.warn(` - ${line}`);
  }

  console.log(`Done. Icons stored in: ${outDir}`);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
