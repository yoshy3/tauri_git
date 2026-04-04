import { addMessages, getLocaleFromNavigator, init, locale } from "svelte-i18n";
import en from "./locales/en.json";
import ja from "./locales/ja.json";

const storageKey = "tauri-git:locale";
const defaultLocale = "en";
const supportedLocales = new Set(["en", "ja"]);

let initialized = false;

export function normalizeLocale(value: string | null | undefined) {
  if (!value) {
    return defaultLocale;
  }

  const baseLocale = value.toLowerCase().split("-")[0];
  return supportedLocales.has(baseLocale) ? baseLocale : defaultLocale;
}

export const availableLocales = ["en", "ja"] as const;

export function setAppLocale(value: string) {
  locale.set(normalizeLocale(value));
}

export function setupI18n() {
  if (initialized) {
    return;
  }

  addMessages("en", en);
  addMessages("ja", ja);

  const storedLocale =
    typeof localStorage === "undefined" ? null : localStorage.getItem(storageKey);
  const initialLocale = normalizeLocale(storedLocale ?? getLocaleFromNavigator());

  init({
    fallbackLocale: defaultLocale,
    initialLocale,
  });

  locale.subscribe((value) => {
    if (!value || typeof localStorage === "undefined") {
      return;
    }

    localStorage.setItem(storageKey, normalizeLocale(value));
  });

  initialized = true;
}
