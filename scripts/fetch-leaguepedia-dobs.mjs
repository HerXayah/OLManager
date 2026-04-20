import { readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const ROOT = resolve(process.cwd());
const seedPath = resolve(ROOT, "data/lec/seed.teams-players.local.json");
const outPath = resolve(ROOT, "data/lec/player-overrides.json");

const MONTHS = {
  january: 1,
  february: 2,
  march: 3,
  april: 4,
  may: 5,
  june: 6,
  july: 7,
  august: 8,
  september: 9,
  october: 10,
  november: 11,
  december: 12,
};

function sleep(ms) {
  return new Promise((resolveSleep) => setTimeout(resolveSleep, ms));
}

function toIsoDate(year, month, day) {
  const y = String(year).padStart(4, "0");
  const m = String(month).padStart(2, "0");
  const d = String(day).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

function parseBirthDateFromWikitext(wikitext) {
  const yearMatch = wikitext.match(/\|\s*birth_date_year\s*=\s*([^\n\r|]+)/i);
  const monthMatch = wikitext.match(/\|\s*birth_date_month\s*=\s*([^\n\r|]+)/i);
  const dayMatch = wikitext.match(/\|\s*birth_date_day\s*=\s*([^\n\r|]+)/i);

  if (!yearMatch || !monthMatch || !dayMatch) {
    return null;
  }

  const year = Number(String(yearMatch[1]).trim());
  const day = Number(String(dayMatch[1]).trim());
  const rawMonth = String(monthMatch[1]).trim();

  let month = Number(rawMonth);
  if (!Number.isFinite(month) || month < 1 || month > 12) {
    month = MONTHS[rawMonth.toLowerCase()] ?? NaN;
  }

  if (!Number.isFinite(year) || !Number.isFinite(month) || !Number.isFinite(day)) {
    return null;
  }

  if (year < 1970 || year > 2015 || month < 1 || month > 12 || day < 1 || day > 31) {
    return null;
  }

  return toIsoDate(year, month, day);
}

async function fetchPlayerWikitext(page) {
  const params = new URLSearchParams({
    action: "parse",
    page,
    prop: "wikitext",
    format: "json",
    redirects: "true",
  });

  const url = `https://lol.fandom.com/api.php?${params.toString()}`;
  const response = await fetch(url, {
    headers: {
      "User-Agent": "LeagueManager/0.1 (local data bootstrap)",
    },
  });

  if (!response.ok) {
    return null;
  }

  const data = await response.json();
  const wikitext = data?.parse?.wikitext?.["*"];
  if (!wikitext || typeof wikitext !== "string") {
    return null;
  }

  const title = data?.parse?.title ?? page;
  return { title, wikitext };
}

async function findCandidateTitles(query) {
  const params = new URLSearchParams({
    action: "query",
    list: "search",
    srsearch: query,
    srlimit: "5",
    format: "json",
  });

  const url = `https://lol.fandom.com/api.php?${params.toString()}`;
  const response = await fetch(url, {
    headers: {
      "User-Agent": "LeagueManager/0.1 (local data bootstrap)",
    },
  });

  if (!response.ok) return [];
  const data = await response.json();
  return (data?.query?.search ?? []).map((entry) => entry.title).filter(Boolean);
}

async function resolveDateOfBirthByNick(nick) {
  const direct = await fetchPlayerWikitext(nick);
  if (direct) {
    const dob = parseBirthDateFromWikitext(direct.wikitext);
    if (dob) return { dob, page: direct.title, strategy: "direct" };
  }

  const candidates = await findCandidateTitles(`${nick} Leaguepedia player`);
  for (const title of candidates) {
    const candidate = await fetchPlayerWikitext(title);
    if (!candidate) continue;
    const dob = parseBirthDateFromWikitext(candidate.wikitext);
    if (dob) return { dob, page: candidate.title, strategy: "search" };
  }

  return null;
}

const rawSeed = await readFile(seedPath, "utf8");
const seed = JSON.parse(rawSeed.replace(/^\uFEFF/, ""));

const nicks = Array.from(
  new Set(
    seed.teams
      .flatMap((team) => team.players ?? [])
      .map((player) => String(player.summonerName || "").trim())
      .filter(Boolean),
  ),
).sort((a, b) => a.localeCompare(b));

const players = {};
const missing = [];

for (const nick of nicks) {
  const result = await resolveDateOfBirthByNick(nick);
  if (result) {
    players[nick] = {
      date_of_birth: result.dob,
      leaguepedia_page: result.page,
      strategy: result.strategy,
    };
    console.log(`✓ ${nick} -> ${result.dob} (${result.page})`);
  } else {
    missing.push(nick);
    console.log(`- ${nick} -> not found`);
  }
  // polite pacing to avoid hammering
  await sleep(120);
}

const output = {
  generatedAt: new Date().toISOString(),
  source: "leaguepedia",
  players,
  missing,
};

await mkdir(dirname(outPath), { recursive: true });
await writeFile(outPath, JSON.stringify(output, null, 2), "utf8");

console.log(`\nSaved overrides: ${outPath}`);
console.log(`Resolved: ${Object.keys(players).length} / ${nicks.length}`);
console.log(`Missing: ${missing.length}`);
