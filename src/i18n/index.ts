import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import es from "./locales/es.json";
import pt from "./locales/pt.json";
import fr from "./locales/fr.json";
import de from "./locales/de.json";
import ptBR from "./locales/pt-BR.json";
import it from "./locales/it.json";

export const SUPPORTED_LANGUAGES = [
  { code: "en", label: "English" },
  { code: "es", label: "Español" },
  { code: "fr", label: "Français" },
  { code: "pt-BR", label: "Português (Brasil)" },
] as const;

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    es: { translation: es },
    pt: { translation: pt },
    fr: { translation: fr },
    de: { translation: de },
    it: { translation: it },
    "pt-BR": { translation: ptBR },
  },
  lng: "es",
  fallbackLng: "es",
  interpolation: {
    escapeValue: false, // React already escapes
  },
});

export default i18n;
