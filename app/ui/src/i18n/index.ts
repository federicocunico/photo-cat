import { feedback } from "./feedback";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { exclusions } from "./exclusions";
import { similar } from "./similar";
import { en } from "./en";
import { it } from "./it";
import { features } from "./features";
import { screens } from "./screens";
import { modules } from "./modules";
import { tuning } from "./tuning";
import { sources } from "./sources";
import { entries } from "./entries";
import { common } from "./common";
import { selection } from "./selection";
import { commandStrings } from "./commands";
import { guide } from "./guide";

export type Language = "it" | "en";

void i18n.use(initReactI18next).init({
  resources: {
    it: {
      translation: {
        ...it,
        ...exclusions.it,
        ...feedback.it,
        ...similar.it,
        ...features.it,
        ...screens.it,
        ...modules.it,
        ...tuning.it,
        ...sources.it,
        ...entries.it,
        ...common.it,
        ...selection.it,
        ...commandStrings.it,
        ...guide.it,
      },
    },
    en: {
      translation: {
        ...en,
        ...exclusions.en,
        ...feedback.en,
        ...similar.en,
        ...features.en,
        ...screens.en,
        ...modules.en,
        ...tuning.en,
        ...sources.en,
        ...entries.en,
        ...common.en,
        ...selection.en,
        ...commandStrings.en,
        ...guide.en,
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
