import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { en } from "./en";
import { it } from "./it";
import { features } from "./features";
import { screens } from "./screens";
import { modules } from "./modules";
import { tuning } from "./tuning";
import { sources } from "./sources";
import { entries } from "./entries";

export type Language = "it" | "en";

void i18n.use(initReactI18next).init({
  resources: {
    it: {
      translation: {
        ...it,
        ...features.it,
        ...screens.it,
        ...modules.it,
        ...tuning.it,
        ...sources.it,
        ...entries.it,
      },
    },
    en: {
      translation: {
        ...en,
        ...features.en,
        ...screens.en,
        ...modules.en,
        ...tuning.en,
        ...sources.en,
        ...entries.en,
      },
    },
  },
  lng: "it",
  fallbackLng: "it",
  interpolation: { escapeValue: false },
  returnNull: false,
});

export function setLanguage(lang: Language) {
  void i18n.changeLanguage(lang);
  document.documentElement.lang = lang;
}

export default i18n;
