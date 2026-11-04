import { createApp, type App } from "vue";
import { createPinia, getActivePinia, setActivePinia, type Pinia } from "pinia";
import { PiniaColada, useQueryCache } from "@pinia/colada";
import { piniaColadaOptions } from "~/queries/options";
import { clearQueryCache } from "~/queries/session-cache";

/**
 * Makes a new active Pinia with the query layer installed, as `main.ts` does.
 * `src/tests/setup.ts` calls it before every test, so each one starts with an
 * empty query cache and empty stores.
 */
export function freshPinia(): Pinia {
  const pinia = createPinia();
  // The query cache store is created inside this app, so it reads the same
  // defaults as the real app.
  createApp({}).use(pinia).use(PiniaColada, piniaColadaOptions);
  setActivePinia(pinia);
  return pinia;
}

/** Installs the active Pinia and the query layer on a test app. */
export function installQueryLayer(app: App): App {
  const pinia = getActivePinia() ?? freshPinia();
  return app.use(pinia).use(PiniaColada, piniaColadaOptions);
}

/**
 * Forgets every cached read but keeps the stores, like opening the screen in
 * a new tab with the same sign-in. For tests that mount a screen again with
 * different fixtures.
 */
export function forgetQueries(): void {
  clearQueryCache(useQueryCache());
}

/** Everything the query cache holds, serialized, for "never cached" checks. */
export function cacheContents(): string {
  return JSON.stringify(
    useQueryCache()
      .getEntries()
      .map((entry) => [entry.key, entry.state.value, entry.placeholderData]),
  );
}
