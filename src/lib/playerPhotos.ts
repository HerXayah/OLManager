import playersSeed from "../../data/lec/draft/players.json";
import lesExampleRaw from "../../EXAMPLE/les.txt?raw";
import lflExampleRaw from "../../EXAMPLE/lfl.txt?raw";
import primeLeagueExampleRaw from "../../EXAMPLE/Prime League.txt?raw";

interface PlayerSeedEntry {
  ign?: string;
  photo?: string;
}

const FALLBACK_PLAYER_PHOTO = "/player-photos/107455908655055017.png";

function normalizeKey(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]/g, "");
}

function seedPhotoToSrc(photo?: string): string | null {
  if (!photo) return null;
  if (photo.startsWith("/images/")) return `/data/lec${photo}`;
  return photo;
}

const ALL_SEED_PLAYERS: PlayerSeedEntry[] = [
  ...(((playersSeed as { data?: { rostered_seeds?: PlayerSeedEntry[] } }).data?.rostered_seeds ??
    []) as PlayerSeedEntry[]),
  ...(((playersSeed as { data?: { free_agent_seeds?: PlayerSeedEntry[] } }).data?.free_agent_seeds ??
    []) as PlayerSeedEntry[]),
];

const PHOTO_BY_IGN = new Map<string, string>();
ALL_SEED_PLAYERS.forEach((seed) => {
  const ign = String(seed.ign ?? "").trim();
  if (!ign) return;
  const src = seedPhotoToSrc(seed.photo);
  if (!src) return;
  PHOTO_BY_IGN.set(normalizeKey(ign), src);
});

function parseExamplePhotoMap(content: string): Map<string, string> {
  const map = new Map<string, string>();
  let currentIgn = "";

  const rolePrefixes = ["Toplaner:", "Jungle:", "Midlaner:", "ADC:", "Support:"];

  content.split(/\r?\n/).forEach((rawLine) => {
    const line = rawLine.trim();
    if (!line) return;

    const rolePrefix = rolePrefixes.find((prefix) => line.startsWith(prefix));
    if (rolePrefix) {
      currentIgn = line.slice(rolePrefix.length).trim();
      return;
    }

    if (line.startsWith("Image:")) {
      const rawUrl = line.slice("Image:".length).trim();
      if (!currentIgn) return;
      if (!rawUrl || rawUrl.includes("??") || !rawUrl.startsWith("http")) return;
      map.set(normalizeKey(currentIgn), rawUrl);
    }
  });

  return map;
}

const EXAMPLE_PHOTO_MAP = new Map<string, string>([
  ...parseExamplePhotoMap(lesExampleRaw).entries(),
  ...parseExamplePhotoMap(lflExampleRaw).entries(),
  ...parseExamplePhotoMap(primeLeagueExampleRaw).entries(),
]);

export function resolvePlayerPhoto(playerId: string, matchName?: string): string | null {
  const legacy = playerId.match(/^lec-player-(.+)$/);
  if (legacy) return `/player-photos/${legacy[1]}.png`;

  const key = normalizeKey(matchName ?? "");
  if (!key) return FALLBACK_PLAYER_PHOTO;
  return PHOTO_BY_IGN.get(key) ?? EXAMPLE_PHOTO_MAP.get(key) ?? FALLBACK_PLAYER_PHOTO;
}
