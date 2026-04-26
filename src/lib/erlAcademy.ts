export type ErlLeagueLevel = "Tier 2" | "Tier 3";
export type ErlSource = "local-seed" | "leaguepedia-import";
export type ErlDataStatus = "seed" | "import-pending" | "imported";
export type ErlDataConfidence = "known" | "configurable" | "import-pending";

export type ErlRole = "Top" | "Jungle" | "Mid" | "ADC" | "Support";

const ERL_SEED_LAST_CHECKED = "2026-04-25";

export interface ErlLeague {
  id: string;
  name: string;
  shortName: string;
  region: string;
  country: string;
  level: ErlLeagueLevel;
  source: ErlSource;
  sourceTournament?: string;
  dataStatus: ErlDataStatus;
  confidence: ErlDataConfidence;
  lastChecked?: string;
}

export interface ErlTeam {
  id: string;
  name: string;
  shortName: string;
  leagueId: string;
  region: string;
  country: string;
  reputation: number;
  developmentLevel: number;
  logoUrl?: string;
  sourcePage?: string;
  affiliatedLecTeamId?: string;
  dataStatus: ErlDataStatus;
  confidence: ErlDataConfidence;
  lastChecked?: string;
}

export interface ErlProspect {
  id: string;
  nickname: string;
  fullName?: string;
  role: ErlRole;
  country: string;
  age: number;
  ovr: number;
  potential: number;
  teamId: string;
  imageUrl?: string;
  sourcePage?: string;
}

export interface LecAcademyAffiliation {
  lecTeamId: string;
  lecTeamNames: string[];
  academyTeamId: string;
  confidence: "known" | "configurable";
  note?: string;
}

export interface AcademySummary {
  prospectCount: number;
  averageOvr: number;
  averagePotential: number;
  eliteProspects: number;
  averageAge: number;
}

export interface AcademyLookupResult {
  team: ErlTeam | null;
  league: ErlLeague | null;
  summary: AcademySummary;
  prospects: ErlProspect[];
  opportunity: "affiliated" | "funding-opportunity";
  note: string;
}

const emptySummary: AcademySummary = {
  prospectCount: 0,
  averageOvr: 0,
  averagePotential: 0,
  eliteProspects: 0,
  averageAge: 0,
};

const normalize = (value?: string | null) =>
  (value ?? "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9]/g, "");

const localErlLeagues: ErlLeague[] = [
  {
    id: "lfl",
    name: "La Ligue Française",
    shortName: "LFL",
    region: "EMEA",
    country: "FR",
    level: "Tier 2",
    source: "local-seed",
    sourceTournament: "Leaguepedia:LFL",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "liga-espanola",
    name: "Liga Española de League of Legends",
    shortName: "Liga Española",
    region: "EMEA",
    country: "ES",
    level: "Tier 2",
    source: "local-seed",
    sourceTournament: "Leaguepedia:Liga Española de League of Legends",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "prime-league",
    name: "Prime League",
    shortName: "PRM",
    region: "EMEA",
    country: "DE",
    level: "Tier 2",
    source: "local-seed",
    sourceTournament: "Leaguepedia:Prime League",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "nlc",
    name: "Northern League of Legends Championship",
    shortName: "NLC",
    region: "EMEA",
    country: "GB",
    level: "Tier 2",
    source: "local-seed",
    sourceTournament: "Leaguepedia:NLC",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "ultraliga",
    name: "Ultraliga",
    shortName: "UL",
    region: "EMEA",
    country: "PL",
    level: "Tier 2",
    source: "local-seed",
    sourceTournament: "Leaguepedia:Ultraliga",
    dataStatus: "import-pending",
    confidence: "import-pending",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "hitpoint-masters",
    name: "Hitpoint Masters",
    shortName: "HM",
    region: "EMEA",
    country: "CZ",
    level: "Tier 2",
    source: "local-seed",
    sourceTournament: "Leaguepedia:Hitpoint Masters",
    dataStatus: "import-pending",
    confidence: "import-pending",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "elite-series",
    name: "Elite Series",
    shortName: "ELITE",
    region: "EMEA",
    country: "BE",
    level: "Tier 2",
    source: "local-seed",
    sourceTournament: "Leaguepedia:Elite Series",
    dataStatus: "import-pending",
    confidence: "import-pending",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "greek-legends-league",
    name: "Greek Legends League",
    shortName: "GLL",
    region: "EMEA",
    country: "GR",
    level: "Tier 2",
    source: "local-seed",
    sourceTournament: "Leaguepedia:Greek Legends League",
    dataStatus: "import-pending",
    confidence: "import-pending",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "lplol",
    name: "Liga Portuguesa de League of Legends",
    shortName: "LPLOL",
    region: "EMEA",
    country: "PT",
    level: "Tier 2",
    source: "local-seed",
    sourceTournament: "Leaguepedia:LPLOL",
    dataStatus: "import-pending",
    confidence: "import-pending",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "tcl",
    name: "Turkish Championship League",
    shortName: "TCL",
    region: "EMEA",
    country: "TR",
    level: "Tier 2",
    source: "local-seed",
    sourceTournament: "Leaguepedia:TCL",
    dataStatus: "import-pending",
    confidence: "import-pending",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "lit",
    name: "Italian ERL / LIT",
    shortName: "LIT",
    region: "EMEA",
    country: "IT",
    level: "Tier 2",
    source: "local-seed",
    sourceTournament: "Leaguepedia:LIT",
    dataStatus: "import-pending",
    confidence: "configurable",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
];

export const erlLeagues: ErlLeague[] = localErlLeagues;

export const erlLeagueCoverage = {
  trackedLeagues: erlLeagues.length,
  importerPending: erlLeagues.filter((league) => league.dataStatus !== "imported").length,
  generatedAt: ERL_SEED_LAST_CHECKED,
  generatedSource: "manual-seed",
};

const localErlTeams: ErlTeam[] = [
  {
    id: "fnatic-tq",
    name: "Fnatic TQ",
    shortName: "FNTQ",
    leagueId: "liga-espanola",
    region: "EMEA",
    country: "ES",
    reputation: 78,
    developmentLevel: 82,
    sourcePage: "Leaguepedia:Fnatic TQ",
    affiliatedLecTeamId: "fnatic",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "karmine-corp-blue",
    name: "Karmine Corp Blue",
    shortName: "KCB",
    leagueId: "lfl",
    region: "EMEA",
    country: "FR",
    reputation: 84,
    developmentLevel: 80,
    sourcePage: "Leaguepedia:Karmine Corp Blue",
    affiliatedLecTeamId: "karmine-corp",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "vitality-bee",
    name: "Vitality.Bee",
    shortName: "VITB",
    leagueId: "lfl",
    region: "EMEA",
    country: "FR",
    reputation: 74,
    developmentLevel: 76,
    sourcePage: "Leaguepedia:Vitality.Bee",
    affiliatedLecTeamId: "team-vitality",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "sk-gaming-prime",
    name: "SK Gaming Prime",
    shortName: "SKP",
    leagueId: "prime-league",
    region: "EMEA",
    country: "DE",
    reputation: 72,
    developmentLevel: 73,
    sourcePage: "Leaguepedia:SK Gaming Prime",
    affiliatedLecTeamId: "sk-gaming",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "bds-academy",
    name: "BDS Academy",
    shortName: "BDSA",
    leagueId: "lfl",
    region: "EMEA",
    country: "FR",
    reputation: 79,
    developmentLevel: 81,
    sourcePage: "Leaguepedia:BDS Academy",
    affiliatedLecTeamId: "team-bds",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "heretics-academy",
    name: "Team Heretics Academy",
    shortName: "THA",
    leagueId: "liga-espanola",
    region: "EMEA",
    country: "ES",
    reputation: 70,
    developmentLevel: 74,
    sourcePage: "Leaguepedia:Team Heretics Academy",
    affiliatedLecTeamId: "team-heretics",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "giantx-pride",
    name: "GIANTX PRIDE",
    shortName: "GXPR",
    leagueId: "liga-espanola",
    region: "EMEA",
    country: "ES",
    reputation: 68,
    developmentLevel: 71,
    sourcePage: "Leaguepedia:GIANTX PRIDE",
    affiliatedLecTeamId: "giantx",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "mkoi-academy",
    name: "MKOI Academy",
    shortName: "MKOIA",
    leagueId: "liga-espanola",
    region: "EMEA",
    country: "ES",
    reputation: 73,
    developmentLevel: 78,
    sourcePage: "Leaguepedia:configurable-mkoi-academy",
    affiliatedLecTeamId: "mkoi",
    dataStatus: "seed",
    confidence: "configurable",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "solary",
    name: "Solary",
    shortName: "SLY",
    leagueId: "lfl",
    region: "EMEA",
    country: "FR",
    reputation: 66,
    developmentLevel: 68,
    sourcePage: "Leaguepedia:Solary",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "ucam-tokiers",
    name: "UCAM Tokiers",
    shortName: "UCAM",
    leagueId: "liga-espanola",
    region: "EMEA",
    country: "ES",
    reputation: 65,
    developmentLevel: 69,
    sourcePage: "Leaguepedia:UCAM Tokiers",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
  {
    id: "g2-nord",
    name: "G2 Nord",
    shortName: "G2N",
    leagueId: "nlc",
    region: "EMEA",
    country: "GB",
    reputation: 61,
    developmentLevel: 65,
    sourcePage: "Leaguepedia:G2 Nord",
    affiliatedLecTeamId: "g2",
    dataStatus: "seed",
    confidence: "known",
    lastChecked: ERL_SEED_LAST_CHECKED,
  },
];

export const erlTeams: ErlTeam[] = localErlTeams;

const localErlProspects: ErlProspect[] = [
  { id: "fnatic-tq-top", nickname: "FNTQ Top", fullName: "Prospecto configurable", role: "Top", country: "ES", age: 18, ovr: 61, potential: 82, teamId: "fnatic-tq" },
  { id: "fnatic-tq-jungle", nickname: "FNTQ Jungle", fullName: "Prospecto configurable", role: "Jungle", country: "ES", age: 19, ovr: 63, potential: 80, teamId: "fnatic-tq" },
  { id: "fnatic-tq-mid", nickname: "FNTQ Mid", fullName: "Prospecto configurable", role: "Mid", country: "PT", age: 18, ovr: 65, potential: 85, teamId: "fnatic-tq" },
  { id: "kcb-top", nickname: "KCB Top", fullName: "Prospecto configurable", role: "Top", country: "FR", age: 19, ovr: 65, potential: 83, teamId: "karmine-corp-blue" },
  { id: "kcb-adc", nickname: "KCB ADC", fullName: "Prospecto configurable", role: "ADC", country: "FR", age: 18, ovr: 66, potential: 86, teamId: "karmine-corp-blue" },
  { id: "kcb-support", nickname: "KCB Support", fullName: "Prospecto configurable", role: "Support", country: "BE", age: 20, ovr: 62, potential: 78, teamId: "karmine-corp-blue" },
  { id: "vitb-jungle", nickname: "VITB Jungle", fullName: "Prospecto configurable", role: "Jungle", country: "FR", age: 19, ovr: 61, potential: 77, teamId: "vitality-bee" },
  { id: "vitb-mid", nickname: "VITB Mid", fullName: "Prospecto configurable", role: "Mid", country: "FR", age: 18, ovr: 62, potential: 81, teamId: "vitality-bee" },
  { id: "skp-top", nickname: "SKP Top", fullName: "Prospecto configurable", role: "Top", country: "DE", age: 20, ovr: 60, potential: 74, teamId: "sk-gaming-prime" },
  { id: "skp-support", nickname: "SKP Support", fullName: "Prospecto configurable", role: "Support", country: "DE", age: 19, ovr: 59, potential: 76, teamId: "sk-gaming-prime" },
  { id: "bdsa-adc", nickname: "BDSA ADC", fullName: "Prospecto configurable", role: "ADC", country: "CH", age: 18, ovr: 64, potential: 84, teamId: "bds-academy" },
  { id: "tha-mid", nickname: "THA Mid", fullName: "Prospecto configurable", role: "Mid", country: "ES", age: 19, ovr: 60, potential: 79, teamId: "heretics-academy" },
  { id: "gxpr-jungle", nickname: "GXPR Jungle", fullName: "Prospecto configurable", role: "Jungle", country: "ES", age: 18, ovr: 59, potential: 78, teamId: "giantx-pride" },
  { id: "mkoi-academy-top", nickname: "MKOIA Top", fullName: "Prospecto configurable", role: "Top", country: "ES", age: 18, ovr: 61, potential: 81, teamId: "mkoi-academy" },
  { id: "mkoi-academy-jungle", nickname: "MKOIA Jungle", fullName: "Prospecto configurable", role: "Jungle", country: "ES", age: 19, ovr: 62, potential: 80, teamId: "mkoi-academy" },
  { id: "mkoi-academy-mid", nickname: "MKOIA Mid", fullName: "Prospecto configurable", role: "Mid", country: "ES", age: 18, ovr: 63, potential: 83, teamId: "mkoi-academy" },
  { id: "sly-adc", nickname: "SLY ADC", fullName: "Prospecto configurable", role: "ADC", country: "FR", age: 20, ovr: 58, potential: 72, teamId: "solary" },
  { id: "ucam-mid", nickname: "UCAM Mid", fullName: "Prospecto configurable", role: "Mid", country: "ES", age: 18, ovr: 57, potential: 75, teamId: "ucam-tokiers" },
  { id: "g2n-support", nickname: "G2N Support", fullName: "Prospecto configurable", role: "Support", country: "GB", age: 19, ovr: 56, potential: 73, teamId: "g2-nord" },
];

export const erlProspects: ErlProspect[] = localErlProspects;

export const lecAcademyAffiliations: LecAcademyAffiliation[] = [
  { lecTeamId: "fnatic", lecTeamNames: ["Fnatic", "FNC"], academyTeamId: "fnatic-tq", confidence: "known" },
  { lecTeamId: "karmine-corp", lecTeamNames: ["Karmine Corp", "KC", "KCorp"], academyTeamId: "karmine-corp-blue", confidence: "known" },
  { lecTeamId: "team-vitality", lecTeamNames: ["Team Vitality", "Vitality", "VIT"], academyTeamId: "vitality-bee", confidence: "known" },
  { lecTeamId: "sk-gaming", lecTeamNames: ["SK Gaming", "SK"], academyTeamId: "sk-gaming-prime", confidence: "known" },
  { lecTeamId: "team-bds", lecTeamNames: ["Team BDS", "BDS"], academyTeamId: "bds-academy", confidence: "known" },
  { lecTeamId: "team-heretics", lecTeamNames: ["Team Heretics", "Heretics", "TH"], academyTeamId: "heretics-academy", confidence: "known" },
  { lecTeamId: "giantx", lecTeamNames: ["GIANTX", "GX"], academyTeamId: "giantx-pride", confidence: "known" },
  { lecTeamId: "g2", lecTeamNames: ["G2 Esports", "G2"], academyTeamId: "g2-nord", confidence: "known", note: "G2 está afiliado a G2 Nord; seed local pendiente de importador Leaguepedia para roster/fotos." },
  {
    lecTeamId: "mkoi",
    lecTeamNames: ["Movistar KOI", "MKOI", "KOI", "MAD Lions KOI"],
    academyTeamId: "mkoi-academy",
    confidence: "configurable",
    note: "Afiliación configurable para MVP: reemplazar por el equipo ERL real desde el importador offline de Leaguepedia.",
  },
];

export function getErlProspectsForTeam(academyTeamId: string): ErlProspect[] {
  return erlProspects.filter((prospect) => prospect.teamId === academyTeamId);
}

export function getAvailableErlTeamsForAffiliation(excluding: string[] = []): ErlTeam[] {
  const unavailable = new Set([
    ...excluding,
    ...lecAcademyAffiliations.map((affiliation) => affiliation.academyTeamId),
  ]);

  return erlTeams.filter((team) => !unavailable.has(team.id));
}

export function calculateAcademySummary(prospects: ErlProspect[]): AcademySummary {
  if (prospects.length === 0) return emptySummary;

  const average = (selector: (prospect: ErlProspect) => number) =>
    Math.round(prospects.reduce((sum, prospect) => sum + selector(prospect), 0) / prospects.length);

  return {
    prospectCount: prospects.length,
    averageOvr: average((prospect) => prospect.ovr),
    averagePotential: average((prospect) => prospect.potential),
    eliteProspects: prospects.filter((prospect) => prospect.potential >= 82).length,
    averageAge: average((prospect) => prospect.age),
  };
}

export function getAcademyForLecTeam(
  teamId?: string | null,
  teamName?: string | null,
  teamShortName?: string | null,
): AcademyLookupResult {
  const candidates = new Set([normalize(teamId), normalize(teamName), normalize(teamShortName)]);
  const affiliation = lecAcademyAffiliations.find((item) => {
    const mappedNames = [item.lecTeamId, ...item.lecTeamNames].map(normalize);
    return mappedNames.some((name) => candidates.has(name));
  });

  if (!affiliation) {
    return {
      team: null,
      league: null,
      prospects: [],
      summary: emptySummary,
      opportunity: "funding-opportunity",
      note: "Sin academia ERL afiliada en el seed local; puede financiarse o vincularse una organización libre.",
    };
  }

  const team = erlTeams.find((candidate) => candidate.id === affiliation.academyTeamId) ?? null;
  const league = team ? erlLeagues.find((candidate) => candidate.id === team.leagueId) ?? null : null;
  const prospects = team ? getErlProspectsForTeam(team.id) : [];

  return {
    team,
    league,
    prospects,
    summary: calculateAcademySummary(prospects),
    opportunity: team ? "affiliated" : "funding-opportunity",
    note:
      affiliation.note ??
      "Seed local preparado para importador Leaguepedia; datos editables hasta conectar el pipeline offline.",
  };
}
