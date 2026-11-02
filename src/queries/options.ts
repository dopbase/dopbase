import type { PiniaColadaOptions } from "@pinia/colada";

/**
 * How long a read counts as fresh. Inside this window a screen reuses the
 * cached copy without asking the server again.
 */
export const FRESH_FOR = 5_000;

/** Freshness for data that rarely changes, such as roles and sign-in methods. */
export const RARELY_CHANGES = 3 * 60_000;

/** How long data no screen is using stays in memory. */
export const KEEP_UNUSED_FOR = 5 * 60_000;

/**
 * Defaults for every query, shared by `main.ts` and the tests. A cached copy
 * always shows at once; once it is older than {@link FRESH_FOR} it is
 * refreshed in the background on mount, window focus and reconnect.
 *
 * Freshness and lifetime are set here or in a query factory, never by the
 * screen that uses the query: Colada keeps one set of options per entry, so
 * a screen-level override would change it for every other screen too.
 */
export const piniaColadaOptions: PiniaColadaOptions = {
  queryOptions: {
    staleTime: FRESH_FOR,
    gcTime: KEEP_UNUSED_FOR,
    refetchOnMount: true,
    refetchOnWindowFocus: true,
    refetchOnReconnect: true,
  },
};
