import { i18n } from "~/i18n";
import type { Locale } from "~/locales";

/**
 * Switches the app-wide locale from tests without a component context,
 * mirroring what writing to the global composer's `locale` ref does at
 * runtime. Pair with the `vue-i18n` mock (see controller tests) whose
 * `useI18n()` exposes this same global scope.
 */
export function setTestLocale(locale: Locale): void {
  i18n.global.locale.value = locale;
}
