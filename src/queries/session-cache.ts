import { useQueryCache } from "@pinia/colada";
import { emptyEntry, refreshWhere } from "./helpers";
import { PUBLIC } from "./keys";

type QueryCache = ReturnType<typeof useQueryCache>;

/**
 * Empties the query cache when the signed-in account changes or the session
 * ends, so no data read as one account is ever shown to the next.
 *
 * Every pending request is cancelled, so a late response can't write back.
 * Entries no screen uses are removed. Entries a mounted screen still shows
 * are emptied in place rather than removed: a mounted query keeps its entry
 * until its own options change, so removing it would leave the screen
 * showing a detached copy. Emptied entries show the loading state and
 * refetch on next use. Pre-session reads a mounted screen shows (sign-in
 * methods on the login page) stay, request and all, since they belong to no
 * account.
 */
export function clearQueryCache(cache: QueryCache = useQueryCache()): void {
  for (const entry of cache.getEntries()) {
    if (!entry.active) {
      cache.cancel(entry);
      cache.remove(entry);
    } else if (entry.key[0] !== PUBLIC) {
      emptyEntry(cache, entry);
    }
  }
}

/** Refetches every signed-in read on screen, after the account changed. */
export function refetchActiveQueries(
  cache: QueryCache = useQueryCache(),
): Promise<void> {
  return refreshWhere(cache, (entry) => entry.key[0] !== PUBLIC);
}
