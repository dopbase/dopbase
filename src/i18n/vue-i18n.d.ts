import type en from "~/locales/en.json";

/**
 * Teaches vue-i18n the app's message schema so `t()` accepts exactly the
 * flat camelCase keys defined in `src/locales/en.json` — compile-time
 * parity with the previous custom composable's `MessageKey` type.
 */
declare module "vue-i18n" {
  // Interface merging is the only way to augment vue-i18n's schema, so the
  // "empty interface" here is intentional (it adopts every key of en.json).
  // eslint-disable-next-line @typescript-eslint/no-empty-object-type
  interface DefineLocaleMessage extends en {}
}
