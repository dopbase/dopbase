import { computed, toValue, type ComputedRef } from "vue";
import {
  useQueryCache,
  type EntryKey,
  type UseQueryEntry,
  type UseQueryOptions,
} from "@pinia/colada";
import { ApiError } from "~/services/http.client";

type QueryCache = ReturnType<typeof useQueryCache>;

/** The part of a `useQuery()` return the helpers below read. */
interface QueryView<T> {
  readonly data: { readonly value: T | undefined };
  readonly error: { readonly value: unknown };
  readonly isLoading: { readonly value: boolean };
}

/**
 * Turns a query off until `on` is true.
 *
 * Use this instead of a bare `enabled`. Colada keeps one set of options per
 * cache entry, so a screen that switched the shared entry off would also stop
 * it refreshing for every other screen showing the same data. While off, the
 * query parks on its own key and never reaches the server, even when a
 * screen calls `refresh()` or `refetch()` on it.
 */
export function onlyWhen<T extends { key: EntryKey }>(
  on: boolean,
  options: T,
): T {
  if (on) return options;
  return {
    ...options,
    key: ["off", ...options.key],
    enabled: false,
    query: parked,
  };
}

async function parked(): Promise<undefined> {
  return undefined;
}

/**
 * Reads a query outside a screen's `useQuery()`, for one-off reads in event
 * handlers and for fan-out queries built from other entries. The result lands
 * in the shared entry, so screens showing it see it too. `fresh` always asks
 * the server; otherwise a fresh cached copy is reused. When `signal` aborts,
 * the read stops too, unless a screen is showing that entry. Throws on
 * failure.
 */
export async function readQuery<TData, TError>(
  cache: QueryCache,
  options: UseQueryOptions<TData, TError, undefined>,
  { fresh = false, signal }: { fresh?: boolean; signal?: AbortSignal } = {},
): Promise<TData> {
  const entry = cache.ensure<TData, TError, undefined>(options);
  const stop = () => {
    if (!entry.active) cache.cancel(entry);
  };
  signal?.addEventListener("abort", stop, { once: true });
  try {
    const state = await (fresh ? cache.fetch(entry) : cache.refresh(entry));
    if (state.status === "error") throw state.error;
    return state.data as TData;
  } finally {
    signal?.removeEventListener("abort", stop);
  }
}

/**
 * Empties an entry in place: no data, no error, nothing pending, and stale,
 * so whoever shows it next sees the loading state and fetches it again. A
 * mounted query keeps its entry until its own options change, so emptying
 * keeps it live where removing it would leave a detached copy on screen.
 */
export function emptyEntry(cache: QueryCache, entry: UseQueryEntry): void {
  cache.cancel(entry);
  entry.placeholderData = null;
  cache.setEntryState(entry, {
    status: "pending",
    data: undefined,
    error: null,
  });
  cache.invalidate(entry);
}

/**
 * After a change: refetches the entries `where` matches that are on screen
 * and empties the rest, so data known to be out of date is never shown
 * again. Resolves when the refetches settle and never rejects.
 *
 * `offScreenToo` refetches entries no screen shows instead of emptying them,
 * for small lists the next screen should have ready. `refetch: false`
 * empties everything matched, for a write that navigates away next.
 *
 * Unlike Colada's `invalidateQueries()`, every match is marked stale before
 * anything refetches: a query built from other entries (such as Settings
 * counts from each environment's secrets) must not reuse the old ones.
 * Refetches use `refresh()`, so one already running is shared.
 */
export async function refreshWhere(
  cache: QueryCache,
  where: (entry: UseQueryEntry) => boolean,
  {
    refetch = true,
    offScreenToo = false,
  }: { refetch?: boolean; offScreenToo?: boolean } = {},
): Promise<void> {
  const now: UseQueryEntry[] = [];
  for (const entry of cache.getEntries({ predicate: where })) {
    const enabled = toValue(entry.options?.enabled) !== false;
    if (refetch && enabled && (entry.active || offScreenToo)) {
      cache.invalidate(entry);
      now.push(entry);
    } else {
      emptyEntry(cache, entry);
    }
  }
  await Promise.allSettled(now.map((entry) => cache.refresh(entry)));
}

/** Revoked or deleted data should leave the screen, not linger. */
function gone(error: unknown): boolean {
  return (
    error instanceof ApiError && (error.status === 403 || error.status === 404)
  );
}

/**
 * The data a screen shows: `null` while nothing has loaded yet, matching the
 * "null means loading" convention of the controllers. A failed background
 * refresh keeps the last data, except when the server says it is gone or no
 * longer readable (403 or 404).
 */
export function shownData<T>(query: QueryView<T>): ComputedRef<T | null> {
  return computed(() =>
    gone(query.error.value) ? null : (query.data.value ?? null),
  );
}

/**
 * The load error a screen shows, or `null`. By default it appears only when
 * there is no data to show (or the data is gone), so a failed background
 * refresh stays quiet. `withData` also shows it above older data, for
 * screens that say when their list may be out of date. Never shown while a
 * request is running, so a retry shows the loading state again.
 */
export function loadFailure(
  query: QueryView<unknown>,
  copy: string | ((error: unknown) => string),
  { withData = false }: { withData?: boolean } = {},
): ComputedRef<string | null> {
  return computed(() => {
    const error = query.error.value;
    if (!error || query.isLoading.value) return null;
    if (!withData && query.data.value !== undefined && !gone(error))
      return null;
    return typeof copy === "string" ? copy : copy(error);
  });
}
