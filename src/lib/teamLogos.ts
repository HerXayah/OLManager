import lesExampleRaw from "../../data/erls/les.txt?raw";
import lflExampleRaw from "../../data/erls/lfl.txt?raw";
import primeLeagueExampleRaw from "../../data/erls/Prime League.txt?raw";

const FALLBACK_TEAM_LOGOS: Record<string, string> = {
  falkeesports:
    "https://static.wikia.nocookie.net/lolesports_gamepedia_en/images/b/b0/Falke_Esportslogo_square.png/revision/latest/scale-to-width-down/220?cb=20250917172449",
};

function normalizeKey(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9]/g, "");
}

function parseExampleTeamLogoMap(content: string): Map<string, string> {
  const map = new Map<string, string>();
  let currentTeam = "";

  content.split(/\r?\n/).forEach((rawLine) => {
    const line = rawLine.trim();
    if (!line) return;

    if (line.startsWith("Team:")) {
      currentTeam = line.slice("Team:".length).trim();
      return;
    }

    if (line.startsWith("Team Logo:")) {
      const rawUrl = line.slice("Team Logo:".length).trim();
      if (!currentTeam) return;
      if (!rawUrl || rawUrl.includes("??") || !rawUrl.startsWith("http")) return;
      map.set(normalizeKey(currentTeam), rawUrl);
    }
  });

  return map;
}

const EXAMPLE_TEAM_LOGO_MAP = new Map<string, string>([
  ...parseExampleTeamLogoMap(lesExampleRaw).entries(),
  ...parseExampleTeamLogoMap(lflExampleRaw).entries(),
  ...parseExampleTeamLogoMap(primeLeagueExampleRaw).entries(),
  ...Object.entries(FALLBACK_TEAM_LOGOS),
]);

export function resolveExampleTeamLogo(teamName?: string | null): string | null {
  const key = normalizeKey(teamName ?? "");
  if (!key) return null;
  return EXAMPLE_TEAM_LOGO_MAP.get(key) ?? null;
}
