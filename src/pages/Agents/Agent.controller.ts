import { computed, ref, watch } from "vue";
import { useRoute, useRouter, type RouteLocationRaw } from "vue-router";
import { useQuery } from "@pinia/colada";
import type { User } from "~/services/users.api";
import type { Role } from "~/services/roles.api";
import type { UserAccess } from "~/services/access.api";
import { revokePersonalToken } from "~/services/personal-tokens.api";
import type {
  CreatedPersonalToken,
  PersonalToken,
} from "~/services/personal-tokens.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { useAuthStore } from "~/stores/auth.store";
import { userAccessQuery } from "~/queries/access";
import { loadFailure, onlyWhen, shownData } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { personalTokensQuery } from "~/queries/personal-tokens";
import { rolesQuery, userQuery, usersQuery } from "~/queries/users";
import { formatDate, formatLevel, formatRelativeTime } from "~/utils/format";
import { isAgent, tokenState, type TokenState } from "~/pages/Users/people";
import { useAccountActions } from "~/pages/Users/account-actions";
import { liveToken } from "./Agents.controller";
import { transferCandidates } from "./components/TransferOwnerDialog.controller";

export type AgentTab = "access" | "tokens" | "danger";

const TAB_ROUTES: Record<AgentTab, string> = {
  access: "agent",
  tokens: "agent-tokens",
  danger: "agent-danger",
};
const TAB_FOR_ROUTE = Object.fromEntries(
  Object.entries(TAB_ROUTES).map(([tab, route]) => [route, tab]),
) as Record<string, AgentTab>;

/** History rows shown before "Show older". */
export const HISTORY_SHOWN = 10;

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
 * owner, while their role runs agents, or anyone who manages agents may
 * change it.
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

  const invalidate = useInvalidate();
  const markStale = useInvalidate({ refetch: false });

  // Keyed by agent, so another agent starts empty and a late answer for the
  // previous one never lands here.
  const known = computed(() => agentId.value !== "");
  const account = useQuery(() =>
    onlyWhen(known.value, userQuery(agentId.value)),
  );
  const accountAccess = useQuery(() =>
    onlyWhen(known.value, userAccessQuery(agentId.value)),
  );
  const tokenList = useQuery(() =>
    onlyWhen(known.value, personalTokensQuery(agentId.value)),
  );
  // Roles and owners only add detail, so a failure leaves them out.
  const roleList = useQuery(rolesQuery);
  const peopleList = useQuery(usersQuery);
  const accountData = shownData<User>(account);
  const accessData = shownData<UserAccess>(accountAccess);
  const tokenData = shownData<PersonalToken[]>(tokenList);
  const failed = (cause: unknown) =>
    errorMessage(cause, "Could not load this agent.");
  const accountFailure = loadFailure(account, failed);
  const accessFailure = loadFailure(accountAccess, failed);
  const tokensFailure = loadFailure(tokenList, failed);
  /** Its access changed but the new grants didn't load. */
  const reloadError = ref<string | null>(null);
  const loadError = computed(
    () =>
      accountFailure.value ??
      accessFailure.value ??
      tokensFailure.value ??
      reloadError.value,
  );
  const tokensError = ref<string | null>(null);

  /** The agent, once it, its access and its tokens have loaded. */
  const agent = computed<User | null>(() =>
    accountData.value &&
    accessData.value &&
    tokenData.value &&
    isAgent(accountData.value)
      ? accountData.value
      : null,
  );
  const access = computed<UserAccess | null>(() =>
    agent.value ? accessData.value : null,
  );
  const tokens = computed<PersonalToken[] | null>(() =>
    agent.value ? tokenData.value : null,
  );
  const roles = computed<Role[]>(() => roleList.data.value ?? []);
  const people = computed<User[]>(() =>
    (peopleList.data.value ?? []).filter((candidate) => !isAgent(candidate)),
  );

  // People have their own page.
  watch(
    accountData,
    (current) => {
      if (current && !isAgent(current))
        void router.replace({ name: "user", params: { userId: current.id } });
    },
    { immediate: true },
  );

  /** Tries again after a failed load. */
  async function load(): Promise<void> {
    await Promise.all([
      account.refetch(),
      accountAccess.refetch(),
      tokenList.refetch(),
      roleList.refresh(),
      peopleList.refresh(),
    ]);
  }

  const role = computed(
    () =>
      roles.value.find((candidate) => candidate.id === agent.value?.roleId) ??
      null,
  );
  /** The owner's email, from the agent itself or the people list. */
  const ownerEmail = computed(
    () =>
      agent.value?.ownerEmail ??
      people.value.find((person) => person.id === agent.value?.ownerId)
        ?.email ??
      null,
  );
  /** Its owner while they may run agents, or anyone who manages agents. */
  const canManage = computed(
    () =>
      auth.can("agents.manage") ||
      (auth.can("agents.view") &&
        agent.value !== null &&
        agent.value.ownerId === auth.session?.adminId),
  );
  /** Transfer owner is for agents.manage, when someone else could take it. */
  const canTransfer = computed(
    () =>
      auth.can("agents.manage") &&
      transferCandidates(people.value, roles.value, agent.value).length > 0,
  );

  const subtitle = computed(() => {
    if (!agent.value) return "";
    const parts = [
      ownerEmail.value
        ? `Owned by ${ownerEmail.value}`
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
    const ownerName = ownerEmail.value ?? "its owner";
    return `${agent.value.email} reaches at most ${formatLevel(access.value.maxLevel)}, and never more than ${ownerName} does.`;
  });

  /**
   * After a grant changes: every access view and what it decides refresh.
   * Says so when its own didn't, since the grants shown are then out of date.
   */
  async function reloadAccess(): Promise<void> {
    await invalidate.accessChanged();
    reloadError.value = accountAccess.error.value
      ? "Could not reload its access."
      : null;
  }

  // Tokens
  const current = computed(() =>
    tokens.value ? liveToken(tokens.value) : null,
  );
  /** Live tokens first, then the history; newest first within each. */
  const tokenRows = computed<AgentTokenRow[]>(() => {
    const rows = [...(tokens.value ?? [])]
      .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
      .map((token) => ({
        token,
        state: tokenState(token),
        scope: token.scopeLabel || "Every project it reaches",
        created: formatDate(token.createdAt),
        expires: token.expiresAt ? formatDate(token.expiresAt) : "Never",
        lastUsed: token.lastUsedAt
          ? formatRelativeTime(token.lastUsedAt)
          : "Never",
      }));
    return [
      ...rows.filter((row) => row.state === "active"),
      ...rows.filter((row) => row.state !== "active"),
    ];
  });
  const liveRows = computed(() =>
    tokenRows.value.filter((row) => row.state === "active"),
  );
  /** Revoked and expired tokens: they no longer work, kept as a record. */
  const historyRows = computed(() =>
    tokenRows.value.filter((row) => row.state !== "active"),
  );
  const showAllHistory = ref(false);
  const shownHistory = computed(() =>
    showAllHistory.value
      ? historyRows.value
      : historyRows.value.slice(0, HISTORY_SHOWN),
  );
  /** History rows behind "Show older". */
  const olderCount = computed(
    () => historyRows.value.length - shownHistory.value.length,
  );
  watch(agentId, () => {
    showAllHistory.value = false;
  });

  async function reloadTokens(): Promise<void> {
    await invalidate.personalTokensChanged();
    tokensError.value = tokenList.error.value
      ? errorMessage(tokenList.error.value, "Could not load its tokens.")
      : null;
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
    await invalidate.usersChanged();
  }

  const actions = useAccountActions(async (action) => {
    if (action.kind === "delete") {
      void markStale.usersChanged();
      await router.push({ name: "agents" });
      return;
    }
    await invalidate.usersChanged();
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
    ownerEmail,
    people,
    loadError,
    load,
    canManage,
    canTransfer,
    subtitle,
    principal,
    accessSummary,
    reloadAccess,
    current,
    tokenRows,
    liveRows,
    historyRows,
    shownHistory,
    showAllHistory,
    olderCount,
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
