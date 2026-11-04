import { computed, ref, watch } from "vue";
import { useRoute, useRouter, type RouteLocationRaw } from "vue-router";
import { useQuery } from "@pinia/colada";
import type { User } from "~/services/users.api";
import type { Role } from "~/services/roles.api";
import { addGroupMember, removeGroupMember } from "~/services/access.api";
import type { GroupSummary, UserAccess } from "~/services/access.api";
import { revokePersonalToken } from "~/services/personal-tokens.api";
import type { PersonalToken } from "~/services/personal-tokens.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { useAuthStore } from "~/stores/auth.store";
import { groupsQuery, userAccessQuery } from "~/queries/access";
import { authMethodsQuery } from "~/queries/auth";
import { loadFailure, onlyWhen, shownData } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { personalTokensQuery } from "~/queries/personal-tokens";
import { rolesQuery, userQuery } from "~/queries/users";
import {
  formatDate,
  formatLevel,
  formatList,
  formatRelativeTime,
} from "~/utils/format";
import {
  isAgent,
  roleOf,
  signInMethod,
  SIGN_IN_LABELS,
  tokenState,
  type TokenState,
} from "./people";

export type { TokenState };
import { useAccountActions } from "./account-actions";

export type UserTab = "overview" | "access" | "groups" | "tokens" | "danger";

const TAB_ROUTES: Record<UserTab, string> = {
  overview: "user",
  access: "user-access",
  groups: "user-groups",
  tokens: "user-tokens",
  danger: "user-danger",
};
const TAB_FOR_ROUTE = Object.fromEntries(
  Object.entries(TAB_ROUTES).map(([tab, route]) => [route, tab]),
) as Record<string, UserTab>;

export interface TokenRow {
  token: PersonalToken;
  scope: string;
  state: TokenState;
  expires: string;
  lastUsed: string;
}

/**
 * One user's page: Overview, Access (where each grant comes from), Groups,
 * Tokens and the Danger zone. AI agents open on their own page instead.
 *
 * Changing the account needs `users.manage`, and nobody changes root or
 * themselves here; the server refuses both too.
 */
export function useUserController() {
  const route = useRoute();
  const router = useRouter();
  const auth = useAuthStore();
  const { runWithReauth } = useReauthentication();
  const invalidate = useInvalidate();
  const markStale = useInvalidate({ refetch: false });

  const userId = computed(() => String(route.params.userId ?? ""));
  const activeTab = computed<UserTab>(
    () => TAB_FOR_ROUTE[String(route.name)] ?? "overview",
  );

  // Keyed by account, so another account starts empty and a late answer
  // for the previous one never lands here.
  const account = useQuery(() =>
    onlyWhen(userId.value !== "", userQuery(userId.value)),
  );
  const accountAccess = useQuery(() =>
    onlyWhen(userId.value !== "", userAccessQuery(userId.value)),
  );
  // Roles and sign-in methods only add detail, so a failure leaves them out.
  const roleList = useQuery(rolesQuery);
  const methods = useQuery(authMethodsQuery);
  const accountData = shownData<User>(account);
  const accessData = shownData<UserAccess>(accountAccess);
  const failed = (cause: unknown) =>
    errorMessage(cause, "Could not load this account.");
  const accountFailure = loadFailure(account, failed);
  const accessFailure = loadFailure(accountAccess, failed);
  /** Their access changed but the new grants didn't load. */
  const reloadError = ref<string | null>(null);
  const loadError = computed(
    () => accountFailure.value ?? accessFailure.value ?? reloadError.value,
  );

  /** The account, once it and its access have loaded. Agents never show. */
  const user = computed<User | null>(() =>
    accountData.value && accessData.value && !isAgent(accountData.value)
      ? accountData.value
      : null,
  );
  const access = computed<UserAccess | null>(() =>
    user.value ? accessData.value : null,
  );
  const roles = computed<Role[]>(() => roleList.data.value ?? []);
  const ssoEnabled = computed(() => methods.data.value?.sso != null);

  // Agents have their own page, with their tokens.
  watch(
    accountData,
    (current) => {
      if (!current || !isAgent(current)) return;
      void router.replace({
        name: activeTab.value === "tokens" ? "agent-tokens" : "agent",
        params: { agentId: current.id },
      });
    },
    { immediate: true },
  );

  /** Tries again after a failed load. */
  async function load(): Promise<void> {
    await Promise.all([
      account.refetch(),
      accountAccess.refetch(),
      roleList.refresh(),
      methods.refresh(),
    ]);
  }

  const role = computed(() =>
    user.value ? roleOf(user.value, roles.value) : null,
  );
  const isRoot = computed(() => user.value?.role === "root");
  const isYou = computed(() => user.value?.id === auth.session?.adminId);
  const canManage = computed(
    () => auth.can("users.manage") && !isRoot.value && !isYou.value,
  );

  const subtitle = computed(() => {
    const account = user.value;
    if (!account) return "";
    const parts = [
      account.roleName ?? account.role,
      SIGN_IN_LABELS[signInMethod(account)],
      account.lastSignInAt
        ? `last sign-in ${formatRelativeTime(account.lastSignInAt)}`
        : "never signed in",
    ];
    return parts.join(" · ");
  });

  const tabs = computed(() => {
    const id = userId.value;
    const all: { key: UserTab; label: string }[] = [
      { key: "overview", label: "Overview" },
      { key: "access", label: "Access" },
      { key: "groups", label: "Groups" },
      { key: "tokens", label: "Tokens" },
    ];
    if (canManage.value) all.push({ key: "danger", label: "Danger zone" });
    return all.map((tab) => ({
      ...tab,
      to: {
        name: TAB_ROUTES[tab.key],
        params: { userId: id },
      } as RouteLocationRaw,
    }));
  });

  // Access
  const accessSummary = computed(() => {
    const data = access.value;
    if (!data) return "";
    const groups = data.groups.map((group) => group.name);
    const from =
      groups.length === 0
        ? "Access comes from direct grants."
        : `Access comes from direct grants and from the ${formatList(groups)} ${groups.length === 1 ? "group" : "groups"}.`;
    return `Their role allows up to ${formatLevel(data.maxLevel)}. ${from}`;
  });

  const principal = computed(() => ({
    type: "user" as const,
    id: userId.value,
    label: user.value?.email ?? "",
    maxLevel: access.value?.maxLevel,
  }));

  /**
   * After a grant changes: every access view and what it decides refresh.
   * Says so when theirs didn't, since the grants shown are then out of date.
   */
  async function reloadAccess(): Promise<void> {
    await invalidate.accessChanged();
    reloadError.value = accountAccess.error.value
      ? "Could not reload their access."
      : null;
  }

  // Groups
  const groupsError = ref<string | null>(null);
  const canManageGroups = computed(
    () => auth.can("groups.manage") && !isRoot.value,
  );
  const groupList = useQuery(() =>
    onlyWhen(
      activeTab.value === "groups" && canManageGroups.value && !!user.value,
      groupsQuery,
    ),
  );
  const allGroups = computed<GroupSummary[]>(() => groupList.data.value ?? []);
  const joinable = computed(() =>
    allGroups.value.filter(
      (group) => !access.value?.groups.some((own) => own.id === group.id),
    ),
  );

  async function joinGroup(groupId: string): Promise<void> {
    groupsError.value = null;
    try {
      await runWithReauth(async () => {
        await addGroupMember(groupId, userId.value);
      });
      await invalidate.groupsChanged();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      groupsError.value = errorMessage(
        cause,
        "Could not add them to that group.",
      );
    }
  }

  async function leaveGroup(groupId: string): Promise<void> {
    groupsError.value = null;
    try {
      await runWithReauth(async () => {
        await removeGroupMember(groupId, userId.value);
      });
      await invalidate.groupsChanged();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      groupsError.value = errorMessage(
        cause,
        "Could not take them out of that group.",
      );
    }
  }

  // Tokens, read while their tab is open and keyed by account.
  const tokenList = useQuery(() =>
    onlyWhen(
      activeTab.value === "tokens" && !!user.value,
      personalTokensQuery(userId.value),
    ),
  );
  const tokensFailure = loadFailure(tokenList, (cause) =>
    errorMessage(cause, "Could not load their tokens."),
  );
  const revokeError = ref<string | null>(null);
  const tokensError = computed(() => revokeError.value ?? tokensFailure.value);
  const tokens = computed<PersonalToken[] | null>(
    () => tokenList.data.value ?? (tokensFailure.value ? [] : null),
  );
  const canRevokeTokens = computed(() => auth.can("tokens.revoke_others"));

  const tokenRows = computed<TokenRow[]>(() =>
    (tokens.value ?? []).map((token) => ({
      token,
      scope: token.scopeLabel || "All projects they reach",
      state: tokenState(token),
      expires: token.expiresAt ? formatDate(token.expiresAt) : "Never",
      lastUsed: token.lastUsedAt
        ? formatRelativeTime(token.lastUsedAt)
        : "Never",
    })),
  );

  async function revokeToken(token: PersonalToken): Promise<void> {
    revokeError.value = null;
    try {
      await runWithReauth(async () => {
        await revokePersonalToken(userId.value, token.id);
      });
      await invalidate.personalTokensChanged();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      revokeError.value = errorMessage(cause, "Could not revoke that token.");
    }
  }

  // Edit and account actions
  const editOpen = ref(false);

  async function edited(): Promise<void> {
    editOpen.value = false;
    await invalidate.usersChanged();
  }

  const actions = useAccountActions(async (action) => {
    if (action.kind === "delete") {
      void markStale.usersChanged();
      await router.push({ name: "users" });
      return;
    }
    await invalidate.usersChanged();
  });

  const created = computed(() =>
    user.value ? formatDate(user.value.createdAt) : "",
  );

  return {
    userId,
    activeTab,
    tabs,
    user,
    access,
    roles,
    role,
    ssoEnabled,
    loadError,
    load,
    isRoot,
    isYou,
    canManage,
    subtitle,
    created,
    accessSummary,
    principal,
    reloadAccess,
    canManageGroups,
    joinable,
    groupsError,
    joinGroup,
    leaveGroup,
    tokens,
    tokenRows,
    tokensError,
    canRevokeTokens,
    revokeToken,
    editOpen,
    edited,
    actions,
  };
}
