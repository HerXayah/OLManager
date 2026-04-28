import { describe, expect, it } from "vitest";

import { SUPPORTED_LANGUAGES } from "../../../i18n";
import de from "../../../i18n/locales/de.json";
import en from "../../../i18n/locales/en.json";
import es from "../../../i18n/locales/es.json";
import fr from "../../../i18n/locales/fr.json";
import ptBR from "../../../i18n/locales/pt-BR.json";
import pt from "../../../i18n/locales/pt.json";
import { SOCIAL_CONTENT_PACK } from "./content";

type LocaleTree = Record<string, unknown>;

const LOCALE_RESOURCES: Record<string, LocaleTree> = {
  de,
  en,
  es,
  fr,
  pt,
  "pt-BR": ptBR,
};

const LOCALES = Object.fromEntries(
  SUPPORTED_LANGUAGES.map(({ code }) => [code, LOCALE_RESOURCES[code]]),
) as Record<string, LocaleTree | undefined>;

function getNestedValue(tree: LocaleTree, keyPath: string): unknown {
  return keyPath.split(".").reduce<unknown>((value, segment) => {
    if (value === null || typeof value !== "object") {
      return undefined;
    }

    return (value as Record<string, unknown>)[segment];
  }, tree);
}

describe("LoL social i18n coverage", () => {
  it("keeps active question and response translations available in every supported locale", () => {
    const activeResponseIds = new Set(
      SOCIAL_CONTENT_PACK.questions.flatMap((question) => question.responseIds),
    );
    const requiredKeys = [
      ...SOCIAL_CONTENT_PACK.questions.map((question) => question.textKey),
      ...SOCIAL_CONTENT_PACK.responses
        .filter((response) => activeResponseIds.has(response.id))
        .flatMap((response) => [response.labelKey, response.textKey]),
    ];

    const missingKeysByLocale = Object.entries(LOCALES).reduce<Record<string, string[]>>(
      (accumulator, [localeCode, translations]) => {
        if (translations === undefined) {
          accumulator[localeCode] = ["<missing locale resource>"];
          return accumulator;
        }

        const missingKeys = requiredKeys.filter((keyPath) => {
          return getNestedValue(translations, keyPath) === undefined;
        });

        if (missingKeys.length > 0) {
          accumulator[localeCode] = missingKeys;
        }

        return accumulator;
      },
      {},
    );

    expect(missingKeysByLocale).toEqual({});
  });
});
