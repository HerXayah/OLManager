import type { TFunction } from "i18next";

import { SOCIAL_CONTENT_PACK } from "../../content/lol/social/content";
import { extractMatchContext, type CompatibleMatchSummary } from "../../content/lol/social/matchContext";
import { DEFAULT_LEAGUE_ID, registrySide, type UserSide } from "../../content/lol/social/shared";
import {
  filterEligibleOutlets,
  filterEligiblePersonas,
  filterEligibleQuestions,
  filterEligibleResponses,
  selectWeighted,
} from "../../content/lol/social/selectors";
import type { SocialQuestion } from "../../content/lol/social/schema";
import type { GameStateData } from "../../store/gameStore";
import type { MatchEvent, MatchSnapshot } from "./types";

export interface PressResponse {
  id: string;
  tone: string;
  text: string;
  effectId: string;
  target: "squad" | "player" | "none";
}

export interface PressQuestion {
  id: string;
  journalist: string;
  outlet: string;
  question: string;
  responses: PressResponse[];
  playerId?: string;
}

interface Candidate {
  question: SocialQuestion;
  personaId: string;
  personaName: string;
  outletName: string;
  weight: number;
}

interface BuildPressConferenceQuestionsParams {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  userSide: UserSide;
  t: TFunction | ((key: string) => string);
  random?: () => number;
}

function countEvents(events: MatchEvent[], side: UserSide, names: string[]): number {
  const normalized = new Set(names.map((name) => name.toLowerCase()));
  return events.filter(
    (event) => event.side === side && normalized.has(event.event_type.toLowerCase()),
  ).length;
}

function deathsFor(events: MatchEvent[], playerId: string): number {
  return events.filter(
    (event) =>
      event.player_id === playerId &&
      ["death", "died", "killed"].includes(event.event_type.toLowerCase()),
  ).length;
}

function snapshotToSummary(snapshot: MatchSnapshot, userSide: UserSide): CompatibleMatchSummary {
  const enemySide = userSide === "Home" ? "Away" : "Home";
  const userTeam = userSide === "Home" ? snapshot.home_team : snapshot.away_team;
  const enemyTeam = userSide === "Home" ? snapshot.away_team : snapshot.home_team;
  const userRegistrySide = registrySide(userSide);
  const enemyRegistrySide = registrySide(enemySide);
  const userKills = countEvents(snapshot.events, userSide, ["Kill", "Goal", "PenaltyGoal"]);
  const enemyKills = countEvents(snapshot.events, enemySide, ["Kill", "Goal", "PenaltyGoal"]);

  return {
    winnerSide:
      (userSide === "Home" ? snapshot.home_score > snapshot.away_score : snapshot.away_score > snapshot.home_score)
        ? userRegistrySide
        : enemyRegistrySide,
    blueKills: userRegistrySide === "blue" ? userKills : enemyKills,
    redKills: userRegistrySide === "red" ? userKills : enemyKills,
    objectives: {
      [userRegistrySide]: {
        voidgrubs: countEvents(snapshot.events, userSide, ["VoidGrub", "VoidGrubs"]),
        dragons: countEvents(snapshot.events, userSide, ["Dragon"]),
        dragonSoul: countEvents(snapshot.events, userSide, ["DragonSoul"]) > 0,
        elderDragons: countEvents(snapshot.events, userSide, ["ElderDragon"]),
        heralds: countEvents(snapshot.events, userSide, ["Herald"]),
        barons: countEvents(snapshot.events, userSide, ["Baron"]),
        towers: countEvents(snapshot.events, userSide, ["Tower"]),
        inhibitors: countEvents(snapshot.events, userSide, ["Inhibitor"]),
      },
      [enemyRegistrySide]: {
        voidgrubs: countEvents(snapshot.events, enemySide, ["VoidGrub", "VoidGrubs"]),
        dragons: countEvents(snapshot.events, enemySide, ["Dragon"]),
        dragonSoul: countEvents(snapshot.events, enemySide, ["DragonSoul"]) > 0,
        elderDragons: countEvents(snapshot.events, enemySide, ["ElderDragon"]),
        heralds: countEvents(snapshot.events, enemySide, ["Herald"]),
        barons: countEvents(snapshot.events, enemySide, ["Baron"]),
        towers: countEvents(snapshot.events, enemySide, ["Tower"]),
        inhibitors: countEvents(snapshot.events, enemySide, ["Inhibitor"]),
      },
    },
    playerResults: [
      ...userTeam.players.map((player) => {
        const deaths = deathsFor(snapshot.events, player.id);
        return {
          side: userRegistrySide,
          playerId: player.id,
          playerName: player.name,
          role: player.position,
          deaths,
          rating: deaths > 0 ? 4 : 6,
        };
      }),
      ...enemyTeam.players.map((player) => ({
        side: enemyRegistrySide,
        playerId: player.id,
        playerName: player.name,
        role: player.position,
        deaths: deathsFor(snapshot.events, player.id),
        rating: 6,
      })),
    ],
  };
}

function buildCandidates(params: {
  questions: SocialQuestion[];
  leagueId?: string;
  contextTags: string[];
}): Candidate[] {
  const outlets = filterEligibleOutlets(SOCIAL_CONTENT_PACK.outlets, { leagueId: params.leagueId });
  const personas = filterEligiblePersonas(SOCIAL_CONTENT_PACK.personas, {
    leagueId: params.leagueId,
    outletIds: outlets.map((outlet) => outlet.id),
  });

  return personas.flatMap((persona) => {
    const outlet = outlets.find((item) => item.id === persona.outletId);
    if (!outlet) return [];

    const questions = filterEligibleQuestions(params.questions, {
      leagueId: params.leagueId,
      personaId: persona.id,
      allowedTones: persona.allowedTones,
      contextTags: params.contextTags,
    });

    return questions.map((question) => ({
      question,
      personaId: persona.id,
      personaName: persona.displayName,
      outletName: outlet.name,
      weight: persona.weight * outlet.weight * question.weight,
    }));
  });
}

export function buildPressConferenceQuestions({
  snapshot,
  gameState,
  userSide,
  t,
  random = Math.random,
}: BuildPressConferenceQuestionsParams): PressQuestion[] {
  const leagueId = gameState.league?.id ?? DEFAULT_LEAGUE_ID;
  const context = extractMatchContext({
    match: snapshotToSummary(snapshot, userSide),
    userSide: registrySide(userSide),
    leagueId,
  });
  const candidate = selectWeighted(
    buildCandidates({
      questions: SOCIAL_CONTENT_PACK.questions,
      leagueId,
      contextTags: context.tags,
    }),
    random,
  );

  const fallbackResponses: PressResponse[] = [
    {
      id: "credit-preparation",
      tone: t("content.lol.social.responses.creditPreparation.label"),
      text: t("content.lol.social.responses.creditPreparation.text"),
      effectId: "press_squad_morale_small_up",
      target: "squad",
    },
    {
      id: "stay-measured",
      tone: t("content.lol.social.responses.stayMeasured.label"),
      text: t("content.lol.social.responses.stayMeasured.text"),
      effectId: "press_no_effect",
      target: "none",
    },
  ];

  if (!candidate) {
    return [
      {
        id: "fallback-post-match",
        journalist: "Verified Analyst",
        outlet: "Rift Desk",
        question: t("content.lol.social.questions.cleanWinObjectives.text"),
        responses: fallbackResponses,
      },
    ];
  }

  const responses = filterEligibleResponses(SOCIAL_CONTENT_PACK.responses, {
    responseIds: candidate.question.responseIds,
    allowedTones: candidate.question.tones,
  })
    .map((response) => ({
      id: response.id,
      tone: t(response.labelKey),
      text: t(response.textKey),
      effectId: response.effectId,
      target: response.target,
    }))
    .filter((response) => response.text && response.tone);

  const safeResponses = responses.length > 0 ? responses : fallbackResponses;

  const targetPlayerId =
    responses.some((response) => response.target === "player") && typeof context.facts.worstPlayerId === "string"
      ? context.facts.worstPlayerId
      : undefined;

  return [
    {
      id: candidate.question.id,
      journalist: candidate.personaName,
      outlet: candidate.outletName,
      question: t(candidate.question.textKey),
      responses: safeResponses,
      playerId: targetPlayerId,
    },
  ];
}
