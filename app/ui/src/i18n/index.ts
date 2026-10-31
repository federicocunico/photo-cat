import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { en } from "./en";
import { it } from "./it";
import { features } from "./features";
import { screens } from "./screens";

export type Language = "it" | "en";

void i18n.use(initReactI18next).init({
  resources: {
    it: { translation: { ...it, ...features.it, ...screens.it } },
    en: { translation: { ...en, ...features.en, ...screens.en } },
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
