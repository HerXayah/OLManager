import { describe, expect, it } from "vitest";

import de from "../i18n/locales/de.json";
import en from "../i18n/locales/en.json";
import es from "../i18n/locales/es.json";
import fr from "../i18n/locales/fr.json";
import itLocale from "../i18n/locales/it.json";
import ptBR from "../i18n/locales/pt-BR.json";
import pt from "../i18n/locales/pt.json";

type LocaleTree = Record<string, unknown>;

const LOCALES: Record<string, LocaleTree> = {
  de,
  en,
  es,
  fr,
  it: itLocale,
  pt,
  "pt-BR": ptBR,
};

const REQUIRED_KEYS = [
  "be.sender.assistantManager",
  "be.role.assistantManager",
  "be.msg.delegatedRenewals.subject",
  "be.msg.delegatedRenewals.body",
  "be.msg.delegatedRenewals.case.successful",
  "be.msg.delegatedRenewals.case.stalled",
  "be.msg.delegatedRenewals.case.failed",
  "be.msg.delegatedRenewals.notes.beyondLimits",
  "be.msg.delegatedRenewals.notes.prefersManager",
  "be.msg.delegatedRenewals.notes.managerBlocked",
  "be.msg.delegatedRenewals.notes.relationshipBlocked",
  "be.msg.playerEvent.respond",
  "be.msg.playerEvent.options.happyPlayer.praiseBack.label",
  "be.msg.playerEvent.options.happyPlayer.praiseBack.description",
  "be.news.weeklyDigest.headline",
  "be.msg.boardWarning.subject",
  "be.msg.boardWarning.body",
  "be.msg.boardFinalWarning.subject",
  "be.msg.boardFinalWarning.body",
  "be.msg.boardFired.subject",
  "be.msg.boardFired.body",
  "be.msg.jobOffer.subject",
  "be.msg.jobOffer.body",
  "be.msg.jobOffer.accept",
  "be.msg.jobOffer.decline",
  "be.msg.jobHired.subject",
  "be.msg.jobHired.body",
  "be.msg.jobRejection.subject",
  "be.msg.jobRejection.body",
  "boardObjectives.objective.LeaguePosition",
  "boardObjectives.objective.Wins",
  "boardObjectives.objective.GoalsScored",
] as const;

function getNestedValue(tree: LocaleTree, keyPath: string): unknown {
  return keyPath
    .split(".")
    .reduce<unknown>((value, segment) => {
      if (value === null || typeof value !== "object") {
        return undefined;
      }

      return (value as Record<string, unknown>)[segment];
    }, tree);
}

describe("backend i18n locale coverage", () => {
  it("keeps required backend-facing translation keys in every supported locale", () => {
    const missingKeysByLocale = Object.entries(LOCALES).reduce<
      Record<string, string[]>
    >((accumulator, [localeCode, translations]) => {
      const missingKeys = REQUIRED_KEYS.filter((keyPath) => {
        return getNestedValue(translations, keyPath) === undefined;
      });

      if (missingKeys.length > 0) {
        accumulator[localeCode] = missingKeys;
      }

      return accumulator;
    }, {});

    expect(missingKeysByLocale).toEqual({});
  });
});
