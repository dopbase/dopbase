import type { EntryKey } from "@pinia/colada";

/**
 * First key part of reads made before anyone signs in (sign-in methods,
 * server health). Clearing the cache on a session change keeps these, and
 * signed-in screens never read them.
 */
export const PUBLIC = "public";

/**
 * Every query key in the Admin UI. Keys are arrays so a change can refresh a
 * whole group by prefix: `keys.secrets.root` covers the secrets of every
 * environment. Anything the result depends on goes in the key, and a missing
 * value is `null`, never `undefined`, because keys are serialized as JSON.
 */
export const keys = {
  projects: ["projects"],
  environments: {
    root: ["environments"],
    /** `null` lists the environments of every project the viewer reaches. */
    list: (project: string | null): EntryKey => ["environments", { project }],
  },
  environmentStats: {
    root: ["environment-stats"],
    of: (environmentIds: readonly string[]): EntryKey => [
      "environment-stats",
      ...environmentIds,
    ],
  },
  secrets: {
    root: ["secrets"],
    of: (environmentId: string): EntryKey => ["secrets", environmentId],
  },
  secretVersions: {
    root: ["secret-versions"],
    /** Every secret's history in one environment. */
    environment: (environmentId: string): EntryKey => [
      "secret-versions",
      environmentId,
    ],
    of: (environmentId: string, key: string): EntryKey => [
      "secret-versions",
      environmentId,
      key,
    ],
  },
  compare: {
    root: ["compare"],
    of: (baseId: string, withIds: readonly string[]): EntryKey => [
      "compare",
      baseId,
      [...withIds],
    ],
  },
  tokens: {
    root: ["tokens"],
    of: (environmentId: string): EntryKey => ["tokens", environmentId],
  },
  audit: {
    root: ["audit"],
    events: (filters: Record<string, string | number | null>): EntryKey => [
      "audit",
      "events",
      filters,
    ],
  },
  access: {
    root: ["access"],
    project: (projectRef: string): EntryKey => [
      "access",
      "project",
      projectRef,
    ],
    /** `"me"` is the signed-in account. */
    user: (userId: string): EntryKey => ["access", "user", userId],
  },
  groups: {
    root: ["groups"],
    list: ["groups", "list"],
    one: (groupId: string): EntryKey => ["groups", "one", groupId],
  },
  users: {
    root: ["users"],
    list: ["users", "list"],
    one: (userId: string): EntryKey => ["users", "one", userId],
    identities: ["users", "identities"],
  },
  roles: ["roles"],
  personalTokens: {
    root: ["personal-tokens"],
    /** `"me"` is the signed-in account. */
    user: (userId: string): EntryKey => ["personal-tokens", "user", userId],
    agents: (agentIds: readonly string[]): EntryKey => [
      "personal-tokens",
      "agents",
      ...agentIds,
    ],
  },
  authMethods: ["auth-methods"],
  sso: {
    root: ["sso"],
    settings: ["sso", "settings"],
    identity: ["sso", "identity"],
  },
  status: ["status"],
  backups: ["backups"],
  public: {
    root: [PUBLIC],
    authMethods: [PUBLIC, "auth-methods"],
    health: [PUBLIC, "health"],
  },
} satisfies Record<string, unknown>;

/** Whether `key` starts with every part of `prefix`. */
export function hasPrefix(key: EntryKey, prefix: EntryKey): boolean {
  if (prefix.length > key.length) return false;
  return prefix.every(
    (part, index) => JSON.stringify(part) === JSON.stringify(key[index]),
  );
}
