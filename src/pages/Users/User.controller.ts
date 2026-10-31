import { computed, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter, type RouteLocationRaw } from "vue-router";
import { fetchUser } from "~/services/users.api";
import type { User } from "~/services/users.api";
import { fetchRoles } from "~/services/roles.api";
import type { Role } from "~/services/roles.api";
import {
  addGroupMember,
  fetchGroups,
  fetchUserAccess,
  removeGroupMember,
} from "~/services/access.api";
import type { GroupSummary, UserAccess } from "~/services/access.api";
import { fetchAuthMethods } from "~/services/auth.api";
import {
  listPersonalTokens,
  revokePersonalToken,
} from "~/services/personal-tokens.api";
import type { PersonalToken } from "~/services/personal-tokens.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { useAuthStore } from "~/stores/auth.store";
import { formatDate, formatLevel, formatRelativeTime } from "~/utils/format";
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

  const userId = computed(() => String(route.params.userId ?? ""));
  const activeTab = computed<UserTab>(
    () => TAB_FOR_ROUTE[String(route.name)] ?? "overview",
  );

  const user = ref<User | null>(null);
  const access = ref<UserAccess | null>(null);
  const roles = ref<Role[]>([]);
  const ssoEnabled = ref(false);
  const loadError = ref<string | null>(null);
  const tokens = ref<PersonalToken[] | null>(null);
  let version = 0;

  async function load(): Promise<void> {
    const current = ++version;
    const id = userId.value;
    loadError.value = null;
    try {
      const [account, grants, available, methods] = await Promise.all([
        fetchUser(id),
        fetchUserAccess(id),
        fetchRoles().catch(() => [] as Role[]),
        fetchAuthMethods().catch(() => null),
      ]);
      if (current !== version) return;
      if (isAgent(account)) {
        // Agents have their own page, with their tokens.
        await router.replace({
          name: activeTab.value === "tokens" ? "agent-tokens" : "agent",
          params: { agentId: account.id },
        });
        return;
      }
      user.value = account;
      access.value = grants;
      roles.value = available;
      ssoEnabled.value = methods?.sso != null;
    } catch (cause) {
      if (current !== version) return;
      loadError.value = errorMessage(cause, "Could not load this account.");
    }
  }

  watch(
    userId,
    () => {
      user.value = null;
      access.value = null;
      tokens.value = null;
      void load();
    },
    { immediate: true },
  );
  onUnmounted(() => {
    version++;
  });

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
        : `Access comes from direct grants and from ${groups.length === 1 ? `the ${groups[0]} group` : `the ${groups.slice(0, -1).join(", ")} and ${groups.at(-1)} groups`}.`;
    return `Their role allows up to ${formatLevel(data.maxLevel)}. ${from}`;
  });

  const principal = computed(() => ({
    type: "user" as const,
    id: userId.value,
    label: user.value?.email ?? "",
    maxLevel: access.value?.maxLevel,
  }));

  async function reloadAccess(): Promise<void> {
    try {
      access.value = await fetchUserAccess(userId.value);
    } catch (cause) {
      loadError.value = errorMessage(cause, "Could not reload their access.");
    }
  }

  // Groups
  const allGroups = ref<GroupSummary[] | null>(null);
  const groupsError = ref<string | null>(null);
  const canManageGroups = computed(
    () => auth.can("groups.manage") && !isRoot.value,
  );
  const joinable = computed(() =>
    (allGroups.value ?? []).filter(
      (group) => !access.value?.groups.some((own) => own.id === group.id),
    ),
  );

  async function loadGroups(): Promise<void> {
    if (!canManageGroups.value || allGroups.value) return;
    try {
      allGroups.value = await fetchGroups();
    } catch {
      allGroups.value = [];
    }
  }

  async function joinGroup(groupId: string): Promise<void> {
    groupsError.value = null;
    try {
      await runWithReauth(async () => {
        await addGroupMember(groupId, userId.value);
      });
      await reloadAccess();
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
      await reloadAccess();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      groupsError.value = errorMessage(
        cause,
        "Could not take them out of that group.",
      );
    }
  }

  // Tokens
  const tokensError = ref<string | null>(null);
  const canRevokeTokens = computed(() => auth.can("tokens.revoke_others"));

  async function loadTokens(): Promise<void> {
    tokensError.value = null;
    try {
      tokens.value = await listPersonalTokens(userId.value);
    } catch (cause) {
      tokens.value = [];
      tokensError.value = errorMessage(cause, "Could not load their tokens.");
    }
  }

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
    tokensError.value = null;
    try {
      await runWithReauth(async () => {
        await revokePersonalToken(userId.value, token.id);
      });
      await loadTokens();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      tokensError.value = errorMessage(cause, "Could not revoke that token.");
    }
  }

  watch(
    [activeTab, user],
    ([tab, account]) => {
      if (!account) return;
      if (tab === "tokens" && tokens.value === null) void loadTokens();
      if (tab === "groups") void loadGroups();
    },
    { immediate: true },
  );

  // Edit and account actions
  const editOpen = ref(false);

  async function edited(): Promise<void> {
    editOpen.value = false;
    await load();
  }

  const actions = useAccountActions(async (action) => {
    if (action.kind === "delete") {
      await router.push({ name: "users" });
      return;
    }
    await load();
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
