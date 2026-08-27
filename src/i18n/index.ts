import { computed, watch } from "vue";
import { createI18n } from "vue-i18n";
import { messages, type Locale } from "./locales";

const STORAGE_KEY = "omahmu.locale";
const FALLBACK_LOCALE: Locale = "en";
const DEFAULT_LOCALE: Locale = "en";
const SUPPORTED_LOCALES: readonly Locale[] = ["en", "en"];

/** Restores the persisted locale, falling back to Bahasa Indonesia. */
function storedLocale(): Locale {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return SUPPORTED_LOCALES.includes(stored as Locale)
      ? (stored as Locale)
      : DEFAULT_LOCALE;
  } catch {
    return DEFAULT_LOCALE;
  }
}

/**
 * Native vue-i18n instance backed by the flat camelCase catalogs in
 * `~/locales`. Installed app-wide in `main.ts`; the Composition API
 * (`legacy: false`) powers `useI18n()` inside components. A key missing
 * at runtime falls back to English, then to the key itself.
 */
export const i18n = createI18n({
  legacy: false,
  locale: storedLocale(),
  fallbackLocale: FALLBACK_LOCALE,
  messages,
  missingWarn: false,
  fallbackWarn: false,
});

// Persist every locale change so the choice survives relaunches.
watch(i18n.global.locale, (next) => {
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // Storage unavailable; keep the locale in-memory only.
  }
});

/** Replaces the app-wide locale. Writes must go through the global scope. */
export function setLocale(next: Locale): void {
  i18n.global.locale.value = next;
}

/** Switches between the two shipped locales ("id" ⇄ "en"). */
export function toggleLocale(): void {
  setLocale(i18n.global.locale.value === "zh" ? "en" : "zh");
}

/** The code of the locale a {@link toggleLocale} would switch to, e.g. "EN". */
export const otherLocaleLabel = computed(() =>
  i18n.global.locale.value === "zh" ? "EN" : "CN",
);
