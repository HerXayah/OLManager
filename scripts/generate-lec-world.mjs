import { readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const ROOT = resolve(process.cwd());
const draftTeamsPath = resolve(ROOT, "data/lec/draft/teams.json");
const draftPlayersPath = resolve(ROOT, "data/lec/draft/players.json");
const overridesPath = resolve(ROOT, "data/lec/player-overrides.json");
const outPath = resolve(ROOT, "src-tauri/databases/lec_world.json");

const TEAM_ID_TO_WORLD_SLUG = {
  fnc: "fnatic",
  g2: "g2-esports",
  gx: "giantx-lec",
  kc: "karmine-corp",
  mkoi: "mad-lions",
  navi: "natus-vincere",
  sft: "shifters",
  sk: "sk-gaming",
  th: "team-heretics-lec",
  vit: "team-vitality",
};

const TEAM_OVERRIDES = {
  "fnatic": {
    country: "GB",
    footballNation: "GB",
    city: "London",
    reputation: 500,
    finance: 3_500_000,
  },
  "g2-esports": {
    country: "DE",
    footballNation: "DE",
    city: "Berlin",
    reputation: 650,
    finance: 4_500_000,
  },
  "giantx-lec": {
    country: "ES",
    footballNation: "ES",
    city: "Málaga",
    reputation: 500,
    finance: 3_500_000,
  },
  "karmine-corp": {
    country: "FR",
    footballNation: "FR",
    city: "Paris",
    reputation: 650,
    finance: 4_500_000,
  },
  "mad-lions": {
    country: "ES",
    footballNation: "ES",
    city: "Madrid",
    reputation: 650,
    finance: 4_500_000,
  },
  "natus-vincere": {
    country: "UA",
    footballNation: "UA",
    city: "Kyiv",
    reputation: 500,
    finance: 3_500_000,
  },
  "sk-gaming": {
    country: "DE",
    footballNation: "DE",
    city: "Berlin",
    reputation: 320,
    finance: 3_000_000,
  },
  "team-bds": {
    country: "TR",
    footballNation: "TR",
    city: "Istanbul",
    reputation: 320,
    finance: 3_000_000,
  },
  "team-heretics-lec": {
    country: "ES",
    footballNation: "ES",
    city: "Madrid",
    reputation: 500,
    finance: 3_500_000,
  },
  "team-vitality": {
    country: "FR",
    footballNation: "FR",
    city: "Paris",
    reputation: 650,
    finance: 4_500_000,
  },
};

function roleToPosition(role) {
  switch (String(role || "").toLowerCase()) {
    case "top":
      return "Defender";
    case "jungle":
      return "Midfielder";
    case "mid":
      return "AttackingMidfielder";
    case "bot":
    case "bottom":
    case "adc":
      return "Forward";
    case "sup":
    case "support":
      return "DefensiveMidfielder";
    default:
      return "Midfielder";
  }
}

function sanitizeText(value) {
  if (!value) return "";
  return String(value)
    .normalize("NFKC")
    // Common mojibake/replacement artifacts from upstream encoding issues
    .replace(/�/g, "")
    .replace(/ï¿½\??/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

function attrsFor(position) {
  const base = {
    pace: 68,
    stamina: 72,
    strength: 66,
    agility: 67,
    passing: 67,
    shooting: 64,
    tackling: 64,
    dribbling: 66,
    defending: 64,
    positioning: 67,
    vision: 67,
    decisions: 68,
    composure: 67,
    aggression: 58,
    teamwork: 74,
    leadership: 62,
    handling: 20,
    reflexes: 20,
    aerial: 55,
  };

  if (position === "Goalkeeper") {
    return {
      ...base,
      pace: 45,
      agility: 58,
      passing: 55,
      shooting: 30,
      tackling: 35,
      dribbling: 44,
      defending: 58,
      handling: 74,
      reflexes: 76,
      aerial: 74,
    };
  }
  if (position === "Defender") {
    return {
      ...base,
      strength: 72,
      tackling: 73,
      defending: 74,
      aerial: 68,
      shooting: 45,
    };
  }
  if (
    position === "DefensiveMidfielder" ||
    position === "Midfielder" ||
    position === "AttackingMidfielder"
  ) {
    return {
      ...base,
      passing: 74,
      vision: 73,
      decisions: 72,
      dribbling: 70,
      shooting: position === "AttackingMidfielder" ? 70 : 62,
      tackling: position === "DefensiveMidfielder" ? 70 : 60,
    };
  }
  return {
    ...base,
    pace: 73,
    shooting: 75,
    dribbling: 72,
    positioning: 71,
    tackling: 48,
    defending: 45,
  };
}

function applyRatingToAttrs(attrs, rating) {
  const normalizedRating = Number.isFinite(Number(rating)) ? Number(rating) : 75;
  const delta = Math.round((normalizedRating - 75) * 0.55);
  const output = { ...attrs };
  const keys = Object.keys(output);

  for (const key of keys) {
    if (key === "handling" || key === "reflexes") continue;
    output[key] = Math.max(25, Math.min(95, output[key] + delta));
  }

  return output;
}

function makePlayer({
  id,
  matchName,
  fullName,
  teamId,
  position,
  nationality,
  dateOfBirth,
  wage,
  marketValue,
  rating,
}) {
  const mappedAttrs = applyRatingToAttrs(attrsFor(position), rating);

  return {
    id,
    match_name: matchName,
    full_name: fullName,
    date_of_birth: dateOfBirth || "2000-01-01",
    nationality,
    football_nation: "EUN",
    birth_country: null,
    position,
    natural_position: position,
    alternate_positions: [],
    footedness: "Right",
    weak_foot: 2,
    attributes: mappedAttrs,
    condition: 100,
    morale: 100,
    fitness: 75,
    injury: null,
    team_id: teamId,
    traits: [],
    contract_end: null,
    wage: Number.isFinite(Number(wage)) ? Number(wage) : 25000,
    market_value: Number.isFinite(Number(marketValue)) ? Number(marketValue) : 1200000,
    stats: {
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
      shots: 0,
      shots_on_target: 0,
      passes_completed: 0,
      passes_attempted: 0,
      tackles_won: 0,
      interceptions: 0,
      fouls_committed: 0,
      fouls_suffered: 0,
      dribbles_completed: 0,
      dribbles_attempted: 0,
      key_passes: 0,
      xg: 0,
      xa: 0,
    },
    career: [],
    training_focus: null,
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    morale_core: {
      manager_trust: 50,
      unresolved_issue: null,
      recent_treatment: null,
      pending_promise: null,
      talk_cooldown_until: null,
      renewal_state: null,
    },
  };
}

function makeStaff(teamId, teamCode, idx, role) {
  return {
    id: `${teamId}-staff-${idx}`,
    first_name: teamCode,
    last_name: `Staff ${idx + 1}`,
    date_of_birth: "1988-01-01",
    nationality: "EUN",
    football_nation: "EUN",
    birth_country: null,
    role,
    attributes: {
      coaching: role === "Coach" ? 78 : 66,
      judging_ability: role === "Scout" ? 77 : 62,
      judging_potential: role === "Scout" ? 76 : 62,
      physiotherapy: role === "Physio" ? 79 : 52,
    },
    team_id: teamId,
    specialization: role === "Coach" ? "Tactics" : null,
    wage: 18000,
    contract_end: null,
  };
}

const rawTeams = await readFile(draftTeamsPath, "utf8");
const draftTeams = JSON.parse(rawTeams.replace(/^\uFEFF/, ""));

const rawPlayers = await readFile(draftPlayersPath, "utf8");
const draftPlayers = JSON.parse(rawPlayers.replace(/^\uFEFF/, ""));

let overrides = { players: {} };
try {
  const rawOverrides = await readFile(overridesPath, "utf8");
  overrides = JSON.parse(rawOverrides.replace(/^\uFEFF/, ""));
} catch {
  // Optional file. If missing, generator falls back to default DOB.
}

const teams = [];
const players = [];
const staff = [];

let existingWorld = { players: [] };
try {
  const rawExisting = await readFile(outPath, "utf8");
  existingWorld = JSON.parse(rawExisting.replace(/^\uFEFF/, ""));
} catch {
  // If world file doesn't exist yet, IDs will be generated.
}

const existingIdsByName = new Map(
  (existingWorld.players ?? []).map((player) => [sanitizeText(player.match_name).toLowerCase(), player.id]),
);

function resolvePlayerId(ign) {
  const key = sanitizeText(ign).toLowerCase();
  const existing = existingIdsByName.get(key);
  if (existing) return existing;
  return `lec-player-${key.replace(/[^a-z0-9]+/g, "-")}`;
}

const teamSeeds = (draftTeams.data?.teams ?? []).map((team) => {
  const slug = TEAM_ID_TO_WORLD_SLUG[team.id] ?? sanitizeText(team.name).toLowerCase().replace(/[^a-z0-9]+/g, "-");
  return {
    id: team.id,
    slug,
    name: team.name,
    code: team.shortName,
    tier: team.tier,
    transferBudget: team.transferBudget,
    salaryBudget: team.salaryBudget,
    startingBudget: team.startingBudget,
  };
});

const playerSeeds = draftPlayers.data?.rostered_seeds ?? [];

for (const teamSeed of teamSeeds) {
  const teamId = `lec-${teamSeed.slug}`;
  const override = TEAM_OVERRIDES[teamSeed.slug] ?? {
    country: "EUN",
    footballNation: "EUN",
    city: "Berlin",
    reputation: 500,
    finance: 3_500_000,
  };

  const finance = Number.isFinite(Number(teamSeed.startingBudget))
    ? Number(teamSeed.startingBudget)
    : override.finance;
  const wageBudget = Number.isFinite(Number(teamSeed.salaryBudget))
    ? Number(teamSeed.salaryBudget)
    : Math.round(finance * 0.22);
  const transferBudget = Number.isFinite(Number(teamSeed.transferBudget))
    ? Number(teamSeed.transferBudget)
    : Math.round(finance * 0.35);

  teams.push({
    id: teamId,
    name: teamSeed.name,
    short_name: teamSeed.code,
    country: override.country,
    football_nation: override.footballNation,
    city: override.city,
    stadium_name: `${teamSeed.name} Arena`,
    stadium_capacity: 28000,
    finance,
    manager_id: null,
      reputation: Math.max(300, Math.min(800, Number(teamSeed.tier ?? 2) * 150 + 200)),
    wage_budget: wageBudget,
    transfer_budget: transferBudget,
    season_income: 0,
    season_expenses: 0,
    financial_ledger: [],
    sponsorship: null,
    facilities: { training: 2, medical: 2, scouting: 2 },
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "Tactical",
    training_intensity: "Medium",
    training_schedule: "Balanced",
    founded_year: 2015,
    colors: { primary: "#1f2937", secondary: "#f3f4f6" },
    training_groups: [],
    starting_xi_ids: [],
    match_roles: {
      captain: null,
      vice_captain: null,
      penalty_taker: null,
      free_kick_taker: null,
      corner_taker: null,
    },
    form: [],
    history: [],
  });

  const mapped = playerSeeds
    .filter((p) => p.teamId === teamSeed.id)
    .map((p) => {
      const ign = sanitizeText(p.ign);
      const firstName = sanitizeText(p.firstName);
      const lastName = sanitizeText(p.lastName);
      const fullName = sanitizeText(`${firstName} ${lastName}`) || ign;

      return {
        id: resolvePlayerId(ign),
        matchName: ign,
        fullName,
        teamId,
        position: roleToPosition(p.role),
        nationality: sanitizeText(p.nationality || "EUN") || "EUN",
        dateOfBirth: overrides.players?.[ign]?.date_of_birth || p.dob || null,
        wage: p.salary,
        marketValue: p.marketValue,
        rating: p.rating,
      };
    });

  for (const p of mapped) {
    players.push(makePlayer(p));
  }

  staff.push(makeStaff(teamId, teamSeed.code, 0, "AssistantManager"));
  staff.push(makeStaff(teamId, teamSeed.code, 1, "Coach"));
  staff.push(makeStaff(teamId, teamSeed.code, 2, "Scout"));
  staff.push(makeStaff(teamId, teamSeed.code, 3, "Physio"));
}

const world = {
  name: "LEC 2026",
  description: "Mundo predefinido de League of Legends (LEC) para OpenFootManager adaptado.",
  teams,
  players,
  staff,
};

await mkdir(dirname(outPath), { recursive: true });
await writeFile(outPath, JSON.stringify(world, null, 2), "utf8");

console.log(`Generated ${outPath}`);
console.log(`Teams: ${teams.length}, Players: ${players.length}, Staff: ${staff.length}`);
