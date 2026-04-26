import playersSeed from "../../data/lec/draft/players.json";

interface PlayerSeedEntry {
  ign?: string;
  photo?: string;
}

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

export function resolvePlayerPhoto(playerId: string, matchName?: string): string | null {
  const legacy = playerId.match(/^lec-player-(.+)$/);
  if (legacy) return `/player-photos/${legacy[1]}.png`;

  const key = normalizeKey(matchName ?? "");
  if (!key) return null;
  return PHOTO_BY_IGN.get(key) ?? null;
}
