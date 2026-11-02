import { computed, ref } from "vue";
import type { RouteLocationRaw } from "vue-router";
import { useQuery } from "@pinia/colada";
import { revokePersonalToken } from "~/services/personal-tokens.api";
import type {
  CreatedPersonalToken,
  PersonalToken,
} from "~/services/personal-tokens.api";
import type { Project } from "~/services/projects.api";
import type { Environment } from "~/services/environments.api";
import type { Role } from "~/services/roles.api";
import { LEVELS, type Level } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { useAuthStore } from "~/stores/auth.store";
import { loadFailure, onlyWhen } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { personalTokensQuery } from "~/queries/personal-tokens";
import { environmentsQuery, projectsQuery } from "~/queries/projects";
import { rolesQuery } from "~/queries/users";
import { formatDate, formatRelativeTime } from "~/utils/format";
import { levelMeter, type LevelMeter } from "~/utils/level";
import { copyText } from "~/utils/clipboard";
import type { TokenLimits } from "./PersonalTokenDialog.controller";

/** The deepest a personal token goes, whatever the role. */
const TOKEN_LEVEL_CEILING: Level = "write";
/** The server's longest lifetime when the role doesn't say. */
const DEFAULT_MAX_DAYS = 365;

export interface PersonalTokenRow {
  token: PersonalToken;
  can: LevelMeter;
  where: string;
  lastUsed: string;
  expires: string;
}

function lower(a: Level, b: Level): Level {
  return LEVELS.indexOf(a) <= LEVELS.indexOf(b) ? a : b;
}

/**
 * The Account page's tokens tab: credentials that act as you, for scripts
 * and AI agents on your machines, or that read the audit log for roles
 * such as Auditor. A token reaches at most what you reach, within its own
 * place and level, and always expires.
 */
export function usePersonalTokensCardController() {
  const auth = useAuthStore();
  const { runWithReauth } = useReauthentication();
  const invalidate = useInvalidate();

  const created = ref<CreatedPersonalToken | null>(null);
  const dialogOpen = ref(false);
  const visible = computed(() => !auth.isRoot && auth.can("tokens.personal"));
  /** Roles such as Auditor hold tokens that read the audit log only. */
  const auditOnly = computed(() => auth.session?.tokenScope === "audit");
  /** Where a new token may reach; audit tokens reach no project. */
  const placesWanted = computed(() => visible.value && !auditOnly.value);

  const tokenList = useQuery(() =>
    onlyWhen(visible.value, personalTokensQuery("me")),
  );
  const projectList = useQuery(() =>
    onlyWhen(placesWanted.value, projectsQuery),
  );
  const environmentList = useQuery(() =>
    onlyWhen(placesWanted.value, environmentsQuery(null)),
  );
  // Only people who can see roles read the limits; for everyone else the
  // server still refuses anything beyond them.
  const roleList = useQuery(() =>
    onlyWhen(auth.can("users.view", "roles.manage"), rolesQuery),
  );

  const failed = (cause: unknown) =>
    errorMessage(cause, "Could not load your tokens.");
  const tokensFailure = loadFailure(tokenList, failed);
  const projectsFailure = loadFailure(projectList, failed);
  const environmentsFailure = loadFailure(environmentList, failed);
  const loadError = computed(
    () =>
      tokensFailure.value ?? projectsFailure.value ?? environmentsFailure.value,
  );
  const copyError = ref<string | null>(null);
  const error = computed(() => copyError.value ?? loadError.value);

  /** Null until everything the tab needs has loaded; empty when it failed. */
  const tokens = computed<PersonalToken[] | null>(() => {
    if (loadError.value) return [];
    const mine = tokenList.data.value;
    if (!mine) return null;
    if (
      placesWanted.value &&
      (!projectList.data.value || !environmentList.data.value)
    )
      return null;
    return mine;
  });
  const projects = computed<Project[]>(() =>
    placesWanted.value ? (projectList.data.value ?? []) : [],
  );
  const environments = computed<Environment[]>(() =>
    placesWanted.value ? (environmentList.data.value ?? []) : [],
  );
  const role = computed<Role | null>(
    () =>
      roleList.data.value?.find(
        (candidate) => candidate.name === auth.session?.roleName,
      ) ?? null,
  );

  const active = computed(() =>
    (tokens.value ?? []).filter(
      (token) =>
        !token.revokedAt &&
        (!token.expiresAt || new Date(token.expiresAt) > new Date()),
    ),
  );
  const rows = computed<PersonalTokenRow[]>(() =>
    active.value.map((token) => ({
      token,
      can: levelMeter(token.level),
      where: token.scopeLabel || "Every project",
      lastUsed: token.lastUsedAt
        ? formatRelativeTime(token.lastUsedAt)
        : "Never",
      expires: token.expiresAt ? formatDate(token.expiresAt) : "Never",
    })),
  );

  /**
   * Your role's token limits. Only people who can see roles read them; for
   * everyone else the server still refuses anything beyond them.
   */
  const limits = computed<TokenLimits>(() => ({
    level: lower(
      lower(auth.session?.maxLevel ?? TOKEN_LEVEL_CEILING, TOKEN_LEVEL_CEILING),
      role.value?.tokenMaxLevel ?? TOKEN_LEVEL_CEILING,
    ),
    days: role.value?.tokenMaxTtlDays ?? DEFAULT_MAX_DAYS,
  }));

  /** Tries again after a failed load. */
  async function load(): Promise<void> {
    if (!visible.value) return;
    copyError.value = null;
    await Promise.all([
      tokenList.refetch(),
      placesWanted.value ? projectList.refetch() : undefined,
      placesWanted.value ? environmentList.refetch() : undefined,
    ]);
  }

  function tokenCreated(token: CreatedPersonalToken): void {
    dialogOpen.value = false;
    created.value = token;
    void invalidate.personalTokensChanged();
  }

  function acknowledge(): void {
    created.value = null;
  }

  // Revoking asks first, then confirms your password if it's been a while.
  const revoking = ref<PersonalToken | null>(null);
  const revokeBusy = ref(false);
  const revokeError = ref<string | null>(null);

  function askRevoke(token: PersonalToken): void {
    revoking.value = token;
    revokeError.value = null;
  }

  function cancelRevoke(): void {
    if (revokeBusy.value) return;
    revoking.value = null;
  }

  async function confirmRevoke(): Promise<void> {
    const token = revoking.value;
    if (!token || revokeBusy.value) return;
    revokeBusy.value = true;
    revokeError.value = null;
    try {
      await runWithReauth(async () => {
        await revokePersonalToken("me", token.id);
      });
      revoking.value = null;
      await invalidate.personalTokensChanged();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      revokeError.value = errorMessage(cause, "Could not revoke that token.");
    } finally {
      revokeBusy.value = false;
    }
  }

  /** A short confirmation after Copy name. */
  const notice = ref<string | null>(null);

  async function copyName(token: PersonalToken): Promise<void> {
    notice.value = (await copyText(token.name))
      ? `Copied ${token.name}.`
      : null;
    copyError.value = notice.value
      ? null
      : "Copy failed. Select the name and copy it.";
  }

  /** Requests with your tokens are recorded with you as the actor. */
  const activity = computed<RouteLocationRaw>(() => ({
    name: "audit",
    query: { actor: auth.session?.adminId ?? "" },
  }));

  /** How a log collector pulls events with an audit token. */
  const pullCommand = computed(
    () =>
      `curl -H "Authorization: Bearer $DOPBASE_TOKEN" \\\n  ${window.location.origin}/api/v1/audit-events`,
  );

  return {
    tokens,
    active,
    rows,
    projects,
    environments,
    role,
    limits,
    error,
    created,
    dialogOpen,
    visible,
    auditOnly,
    load,
    tokenCreated,
    acknowledge,
    revoking,
    revokeBusy,
    revokeError,
    askRevoke,
    cancelRevoke,
    confirmRevoke,
    notice,
    copyName,
    activity,
    pullCommand,
  };
}
