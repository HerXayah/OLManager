const FOOTBALL_ERA_TERMS = [
  /\bfootball\b/i,
  /\bsoccer\b/i,
  /\bpitch\b/i,
  /\bstadium\b/i,
  /\bgoal(s)?\b/i,
  /\bmanager\b/i,
  /\bcoach\b/i,
  /\bstriker\b/i,
  /\bgoalkeeper\b/i,
  /\btransfer\b/i,
  /\bleague table\b/i,
  /\bmatchday\b/i,
  /\bdressing room\b/i,
];

const DEFAULT_ALLOWLIST_KEYS = new Set(["compatibility", "domain"]);

export interface FootballEraFinding {
  path: string;
  term: string;
}

function matchesFootballEraTerm(text: string): string | null {
  for (const pattern of FOOTBALL_ERA_TERMS) {
    if (pattern.test(text)) {
      return pattern.source;
    }
  }

  return null;
}

function scanValue(
  value: unknown,
  path: string,
  allowlistKeys: Set<string>,
  findings: FootballEraFinding[],
): void {
  if (typeof value === "string") {
    const term = matchesFootballEraTerm(value);
    if (term) {
      findings.push({ path, term });
    }
    return;
  }

  if (Array.isArray(value)) {
    value.forEach((item, index) => scanValue(item, `${path}[${index}]`, allowlistKeys, findings));
    return;
  }

  if (!value || typeof value !== "object") {
    return;
  }

  Object.entries(value).forEach(([key, nested]) => {
    if (allowlistKeys.has(key)) {
      return;
    }

    scanValue(nested, path ? `${path}.${key}` : key, allowlistKeys, findings);
  });
}

export function scanFootballEraTerms(
  value: unknown,
  allowlistKeys: Iterable<string> = DEFAULT_ALLOWLIST_KEYS,
): FootballEraFinding[] {
  const findings: FootballEraFinding[] = [];
  scanValue(value, "root", new Set(allowlistKeys), findings);
  return findings;
}
