import { useQueryCache, type EntryKey } from "@pinia/colada";
import { refreshWhere } from "./helpers";
import { hasPrefix, keys, PUBLIC } from "./keys";

/**
 * Refreshes the cached reads a change affects, named after what changed.
 *
 * Writes stay plain service calls with their own busy and error state; after
 * the server confirms, they call one of these. The matching reads on screen
 * refetch at once, and the rest are emptied, so data known to be out of date
 * is never shown again. Each helper resolves when the refetches settle and
 * never rejects: a failed reload shows up on the query itself and must not
 * turn a successful write into an error.
 *
 * Every helper also refreshes the audit feed, since every write is audited.
 *
 * With `{ refetch: false }` the helpers empty every matching read without
 * refetching, for a write that navigates away next: the screen being left
 * doesn't refetch, and the next one loads fresh data when it mounts.
 */
export function useInvalidate({ refetch = true }: { refetch?: boolean } = {}) {
  const cache = useQueryCache();

  /** Removes matching items from every cached list under `prefix`. */
  function drop(prefix: EntryKey, gone: (item: { id?: unknown }) => boolean) {
    for (const entry of cache.getEntries({ key: prefix })) {
      const state = entry.state.value;
      if (!Array.isArray(state.data)) continue;
      cache.setEntryState(entry, {
        ...state,
        data: state.data.filter((item) => !gone(item)),
      });
    }
  }

  /**
   * Refreshes every read under `prefixes` together, so overlapping prefixes
   * don't cancel each other's refetch. `keep` is an entry that was just
   * written with the server's answer and needs no refetch.
   */
  function refreshAll(prefixes: EntryKey[], keep?: EntryKey): Promise<void> {
    const kept = keep && JSON.stringify(keep);
    return refreshWhere(
      cache,
      (entry) =>
        JSON.stringify(entry.key) !== kept &&
        prefixes.some((prefix) => hasPrefix(entry.key, prefix)),
      { refetch },
    );
  }

  const refresh = (...prefixes: EntryKey[]) => refreshAll(prefixes);

  // The project and environment lists also refresh when no screen shows
  // them: they're small, and the next screen would otherwise flash an old
  // name before its own refresh lands.
  const projectTreeChanged = () =>
    Promise.all([
      refreshWhere(
        cache,
        (entry) =>
          hasPrefix(entry.key, keys.projects) ||
          hasPrefix(entry.key, keys.environments.root),
        { refetch, offScreenToo: refetch },
      ),
      refresh(
        keys.environmentStats.root,
        keys.compare.root,
        keys.access.root,
        keys.audit.root,
      ),
    ]).then(() => undefined);

  return {
    /** A project or environment was created, renamed, deleted or protected. */
    projectTreeChanged,
    /**
     * A project was deleted. It leaves every cached list at once, so a
     * screen opened next never shows it while the lists refresh.
     */
    projectDeleted: (projectId: string) => {
      drop(keys.projects, (item) => item.id === projectId);
      drop(
        keys.environments.root,
        (item) => (item as { projectId?: unknown }).projectId === projectId,
      );
      return projectTreeChanged();
    },
    /** An environment was deleted; it leaves every cached list at once. */
    environmentDeleted: (environmentId: string) => {
      drop(keys.environments.root, (item) => item.id === environmentId);
      return projectTreeChanged();
    },
    /**
     * Secrets in these environments changed. Environment lists carry a
     * secret count, so they refresh too.
     */
    secretsChanged: (...environmentIds: string[]) =>
      refresh(
        ...environmentIds.flatMap((id) => [
          keys.secrets.of(id),
          ["secret-versions", id],
        ]),
        keys.environments.root,
        keys.environmentStats.root,
        keys.compare.root,
        keys.audit.root,
      ),
    /** Runner tokens in this environment were created or revoked. */
    runnerTokensChanged: (environmentId: string) =>
      refresh(
        keys.tokens.of(environmentId),
        keys.environmentStats.root,
        keys.audit.root,
      ),
    /**
     * A grant was given, changed or removed. What the viewer reaches may have
     * changed with it, so the project and environment lists refresh too.
     */
    accessChanged: () =>
      refresh(
        keys.access.root,
        keys.groups.root,
        keys.users.root,
        keys.projects,
        keys.environments.root,
        keys.audit.root,
      ),
    /** A group or its members changed. */
    groupsChanged: (keep?: EntryKey) =>
      refreshAll(
        [
          keys.groups.root,
          keys.access.root,
          keys.users.root,
          keys.projects,
          keys.environments.root,
          keys.audit.root,
        ],
        keep,
      ),
    /** A user or AI agent was created, edited, disabled or deleted. */
    usersChanged: () =>
      refresh(
        keys.users.root,
        keys.access.root,
        keys.groups.root,
        keys.personalTokens.root,
        keys.audit.root,
      ),
    /** A role was created, edited or deleted. */
    rolesChanged: () =>
      refresh(
        keys.roles,
        keys.users.root,
        keys.access.root,
        keys.projects,
        keys.environments.root,
        keys.audit.root,
      ),
    /** A personal or agent token was created or revoked. */
    personalTokensChanged: () =>
      refresh(keys.personalTokens.root, keys.audit.root),
    /** Single sign-on settings or an account's SSO link changed. */
    signInChanged: (keep?: EntryKey) =>
      refreshAll(
        [
          keys.authMethods,
          keys.public.authMethods,
          keys.sso.root,
          keys.users.root,
          keys.audit.root,
        ],
        keep,
      ),
    /** The whole database was replaced, as by a backup restore. */
    everythingChanged: () =>
      refreshWhere(cache, (entry) => entry.key[0] !== PUBLIC, { refetch }),
  };
}
