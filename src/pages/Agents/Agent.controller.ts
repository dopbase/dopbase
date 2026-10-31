import { computed, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter, type RouteLocationRaw } from "vue-router";
import { fetchUser, fetchUsers } from "~/services/users.api";
import type { User } from "~/services/users.api";
import { fetchRoles } from "~/services/roles.api";
import type { Role } from "~/services/roles.api";
import { fetchUserAccess } from "~/services/access.api";
import type { UserAccess } from "~/services/access.api";
import {
  listPersonalTokens,
  revokePersonalToken,
} from "~/services/personal-tokens.api";
import type {
  CreatedPersonalToken,
  PersonalToken,
} from "~/services/personal-tokens.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { useAuthStore } from "~/stores/auth.store";
import { formatDate, formatLevel, formatRelativeTime } from "~/utils/format";
import { isAgent, tokenState, type TokenState } from "~/pages/Users/people";
import { useAccountActions } from "~/pages/Users/account-actions";
import { liveToken } from "./Agents.controller";

export type AgentTab = "access" | "tokens" | "danger";

const TAB_ROUTES: Record<AgentTab, string> = {
  access: "agent",
  tokens: "agent-tokens",
  danger: "agent-danger",
};
const TAB_FOR_ROUTE = Object.fromEntries(
  Object.entries(TAB_ROUTES).map(([tab, route]) => [route, tab]),
) as Record<string, AgentTab>;

export interface AgentTokenRow {
  token: PersonalToken;
  state: TokenState;
  scope: string;
  created: string;
  expires: string;
  lastUsed: string;
}

/**
 * One AI agent's page: Access (what it reaches, where it comes from),
 * Tokens (its one live token and the history) and the Danger zone. Its
 * owner, or anyone who manages users, may change it.
 */
export function useAgentController() {
  const route = useRoute();
  const router = useRouter();
  const auth = useAuthStore();
  const { runWithReauth } = useReauthentication();

  const agentId = computed(() => String(route.params.agentId ?? ""));
  const activeTab = computed<AgentTab>(
    () => TAB_FOR_ROUTE[String(route.name)] ?? "access",
  );

  const agent = ref<User | null>(null);
  const access = ref<UserAccess | null>(null);
  const tokens = ref<PersonalToken[] | null>(null);
  const roles = ref<Role[]>([]);
  const people = ref<User[]>([]);
  const loadError = ref<string | null>(null);
  const tokensError = ref<string | null>(null);
  let version = 0;

  async function load(): Promise<void> {
    const current = ++version;
    const id = agentId.value;
    loadError.value = null;
    try {
      const [account, grants, list, available, accounts] = await Promise.all([
        fetchUser(id),
        fetchUserAccess(id),
        listPersonalTokens(id),
        fetchRoles().catch(() => [] as Role[]),
        fetchUsers().catch(() => [] as User[]),
      ]);
      if (current !== version) return;
      if (!isAgent(account)) {
        await router.replace({ name: "user", params: { userId: account.id } });
        return;
      }
      agent.value = account;
      access.value = grants;
      tokens.value = list;
      roles.value = available;
      people.value = accounts.filter((candidate) => !isAgent(candidate));
    } catch (cause) {
      if (current !== version) return;
      loadError.value = errorMessage(cause, "Could not load this agent.");
    }
  }

  watch(
    agentId,
    () => {
      agent.value = null;
      void load();
    },
    { immediate: true },
  );
  onUnmounted(() => {
    version++;
  });

  const role = computed(
    () =>
      roles.value.find((candidate) => candidate.id === agent.value?.roleId) ??
      null,
  );
  const owner = computed(
    () =>
      people.value.find((person) => person.id === agent.value?.ownerId) ?? null,
  );
  const canManage = computed(
    () =>
      auth.can("users.manage") ||
      (agent.value !== null && agent.value.ownerId === auth.session?.adminId),
  );

  const subtitle = computed(() => {
    if (!agent.value) return "";
    const parts = [
      owner.value
        ? `Owned by ${owner.value.email}`
        : agent.value.ownerId
          ? "Owner unknown"
          : "No owner",
      `reaches at most ${formatLevel(access.value?.maxLevel ?? role.value?.maxLevel ?? "none")}`,
    ];
    if (agent.value.status === "disabled") parts.push("disabled");
    return parts.join(" · ");
  });

  const tabs = computed(() => {
    const all: { key: AgentTab; label: string }[] = [
      { key: "access", label: "Access" },
      { key: "tokens", label: "Tokens" },
    ];
    if (canManage.value) all.push({ key: "danger", label: "Danger zone" });
    return all.map((tab) => ({
      ...tab,
      to: {
        name: TAB_ROUTES[tab.key],
        params: { agentId: agentId.value },
      } as RouteLocationRaw,
    }));
  });

  const principal = computed(() => ({
    type: "user" as const,
    id: agentId.value,
    label: agent.value?.email ?? "",
    maxLevel: access.value?.maxLevel,
  }));

  const accessSummary = computed(() => {
    if (!access.value || !agent.value) return "";
    const ownerName = owner.value?.email ?? "its owner";
    return `${agent.value.email} reaches at most ${formatLevel(access.value.maxLevel)}, and never more than ${ownerName} does.`;
  });

  async function reloadAccess(): Promise<void> {
    try {
      access.value = await fetchUserAccess(agentId.value);
    } catch (cause) {
      loadError.value = errorMessage(cause, "Could not reload its access.");
    }
  }

  // Tokens
  const current = computed(() =>
    tokens.value ? liveToken(tokens.value) : null,
  );
  const tokenRows = computed<AgentTokenRow[]>(() =>
    (tokens.value ?? []).map((token) => ({
      token,
      state: tokenState(token),
      scope: token.scopeLabel || "Every project it reaches",
      created: formatDate(token.createdAt),
      expires: token.expiresAt ? formatDate(token.expiresAt) : "Never",
      lastUsed: token.lastUsedAt
        ? formatRelativeTime(token.lastUsedAt)
        : "Never",
    })),
  );

  async function reloadTokens(): Promise<void> {
    tokensError.value = null;
    try {
      tokens.value = await listPersonalTokens(agentId.value);
    } catch (cause) {
      tokensError.value = errorMessage(cause, "Could not load its tokens.");
    }
  }

  const tokenOpen = ref(false);
  const createdToken = ref<CreatedPersonalToken | null>(null);

  async function tokenCreated(created: CreatedPersonalToken): Promise<void> {
    tokenOpen.value = false;
    createdToken.value = created;
    await reloadTokens();
  }

  async function revokeToken(token: PersonalToken): Promise<void> {
    tokensError.value = null;
    try {
      await runWithReauth(async () => {
        await revokePersonalToken(agentId.value, token.id);
      });
      await reloadTokens();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      tokensError.value = errorMessage(cause, "Could not revoke that token.");
    }
  }

  // Owner and account
  const transferOpen = ref(false);

  async function transferred(): Promise<void> {
    transferOpen.value = false;
    await load();
  }

  const actions = useAccountActions(async (action) => {
    if (action.kind === "delete") {
      await router.push({ name: "agents" });
      return;
    }
    await load();
  });

  const activity = computed<RouteLocationRaw>(() => ({
    name: "audit",
    query: { actor: agentId.value },
  }));

  return {
    agentId,
    activeTab,
    tabs,
    agent,
    access,
    tokens,
    roles,
    role,
    owner,
    people,
    loadError,
    load,
    canManage,
    subtitle,
    principal,
    accessSummary,
    reloadAccess,
    current,
    tokenRows,
    tokensError,
    tokenOpen,
    createdToken,
    tokenCreated,
    revokeToken,
    transferOpen,
    transferred,
    actions,
    activity,
  };
}
