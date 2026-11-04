import { computed, ref } from "vue";
import { useRouter, type RouteLocationRaw } from "vue-router";
import { useQuery, useQueryCache } from "@pinia/colada";
import type { User } from "~/services/users.api";
import type { Role } from "~/services/roles.api";
import { revokePersonalToken } from "~/services/personal-tokens.api";
import type { Grant } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import type {
  CreatedPersonalToken,
  PersonalToken,
} from "~/services/personal-tokens.api";
import { useAuthStore } from "~/stores/auth.store";
import { userAccessQuery } from "~/queries/access";
import { loadFailure, onlyWhen, readQuery, shownData } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { agentTokensQuery } from "~/queries/personal-tokens";
import { rolesQuery, usersQuery } from "~/queries/users";
import { formatDate, formatRelativeTime } from "~/utils/format";
import { isAgent, tokenState } from "~/pages/Users/people";
import { useAccountActions } from "~/pages/Users/account-actions";
import { transferCandidates } from "./components/TransferOwnerDialog.controller";

/** Token state as the Token column shows it. */
export type AgentTokenStatus = "active" | "expiring" | "none" | "unknown";

/** A token this close to expiring is flagged. */
const EXPIRING_SOON_MS = 7 * 24 * 60 * 60 * 1000;

export interface AgentRow {
  agent: User;
  owner: string;
  token: PersonalToken | null;
  status: AgentTokenStatus;
  /** "active", "expires in 2 days", "no token". */
  statusLabel: string;
  expires: string;
  lastUsed: string;
  /** You own it, or may manage users. */
  canManage: boolean;
  /** You manage it and someone else could own it. */
  canTransfer: boolean;
  activity: RouteLocationRaw;
}

/** The token an agent works with: the newest live one. */
export function liveToken(tokens: PersonalToken[]): PersonalToken | null {
  return (
    tokens
      .filter((token) => tokenState(token) === "active")
      .sort((a, b) => b.createdAt.localeCompare(a.createdAt))[0] ?? null
  );
}

/**
 * The "Use it" lines under a new agent token: export it, then run with the
 * place it reaches. An environment token names its environment; a project
 * or all-projects token leaves the part to fill in as a placeholder.
 */
export function agentTokenUsage(created: CreatedPersonalToken): string {
  const { token } = created;
  const target =
    token.scopeType === "environment"
      ? token.scopeLabel
      : `${token.scopeLabel || "<project>"}/<environment>`;
  return `export DOPBASE_TOKEN=${created.plaintextToken}\ndopbase run ${target} -- <command>`;
}

function daysUntil(iso: string, now: number): number {
  return Math.max(0, Math.ceil((Date.parse(iso) - now) / 86_400_000));
}

/**
 * AI agents page: token-only accounts for bots and CI. Each has an owner
 * and never reaches more than that person. The row menu creates a token
 * (replacing the live one), transfers the owner, opens its activity,
 * revokes the token, or deletes the agent.
 */
export function useAgentsController() {
  const auth = useAuthStore();
  const router = useRouter();
  const { runWithReauth } = useReauthentication();

  const cache = useQueryCache();
  const invalidate = useInvalidate();
  const markStale = useInvalidate({ refetch: false });

  const accountList = useQuery(usersQuery);
  // Roles only add detail, so a failure leaves them out.
  const roleList = useQuery(rolesQuery);
  const accounts = shownData<User[]>(accountList);
  const agentIds = computed(() =>
    (accounts.value ?? []).filter(isAgent).map((agent) => agent.id),
  );
  // Each agent's tokens through its own entry, which its page reads too. A
  // list that fails shows as an unknown token state.
  const tokenLists = useQuery(() =>
    onlyWhen(
      accounts.value !== null,
      agentTokensQuery({ cache, agentIds: agentIds.value }),
    ),
  );
  // Says so even above the old list, which may be out of date.
  const loadError = loadFailure(accountList, "Could not load AI agents.", {
    withData: true,
  });
  const tokens = computed<Record<string, PersonalToken[] | null>>(
    () => tokenLists.data.value ?? {},
  );
  /** Null until the agents and their tokens have loaded. */
  const agents = computed<User[] | null>(() => {
    if (accounts.value && tokenLists.data.value)
      return accounts.value.filter(isAgent);
    return loadError.value ? [] : null;
  });
  const people = computed<User[]>(() =>
    (accounts.value ?? []).filter((account) => !isAgent(account)),
  );
  const roles = computed<Role[]>(() => roleList.data.value ?? []);
  const filter = ref("");
  const ownerFilter = ref<string | null>(null);

  /** Tries again after a failed load. */
  async function load(): Promise<void> {
    await Promise.all([accountList.refetch(), roleList.refresh()]);
    await tokenLists.refresh();
  }

  /** The role new agents get: the token-only role you may give. */
  const agentRole = computed(
    () =>
      roles.value.find(
        (role) => role.signIn === "token_only" && role.assignable,
      ) ?? null,
  );
  const canAdd = computed(
    () => auth.can("users.manage") && agentRole.value !== null,
  );

  const rows = computed<AgentRow[]>(() => {
    const now = Date.now();
    return (agents.value ?? []).map((agent) => {
      const list = tokens.value[agent.id];
      const token = list ? liveToken(list) : null;
      let status: AgentTokenStatus = "unknown";
      let statusLabel = "—";
      if (list) {
        if (!token) {
          status = "none";
          statusLabel = "no token";
        } else if (
          token.expiresAt &&
          Date.parse(token.expiresAt) - now < EXPIRING_SOON_MS
        ) {
          status = "expiring";
          const days = daysUntil(token.expiresAt, now);
          statusLabel =
            days <= 1 ? "expires in 1 day" : `expires in ${days} days`;
        } else {
          status = "active";
          statusLabel = "active";
        }
      }
      const owner = people.value.find((person) => person.id === agent.ownerId);
      const canManage =
        auth.can("users.manage") || agent.ownerId === auth.session?.adminId;
      return {
        agent,
        owner: owner?.email ?? (agent.ownerId ? "Unknown" : "No owner"),
        token,
        status,
        statusLabel,
        expires: token?.expiresAt ? formatDate(token.expiresAt) : "—",
        lastUsed: token?.lastUsedAt
          ? formatRelativeTime(token.lastUsedAt)
          : "Never",
        canManage,
        canTransfer:
          canManage &&
          transferCandidates(people.value, roles.value, agent).length > 0,
        activity: { name: "audit", query: { actor: agent.id } },
      };
    });
  });

  const ownerFilters = computed(() =>
    [...new Set(rows.value.map((row) => row.owner))]
      .sort()
      .map((owner) => ({ value: owner, label: owner })),
  );

  const visibleRows = computed(() => {
    const needle = filter.value.trim().toLowerCase();
    return rows.value.filter(
      (row) =>
        (!needle || row.agent.email.toLowerCase().includes(needle)) &&
        (!ownerFilter.value || row.owner === ownerFilter.value),
    );
  });

  // Add
  const addOpen = ref(false);

  async function added(agent: User): Promise<void> {
    addOpen.value = false;
    void markStale.usersChanged();
    await router.push({ name: "agent", params: { agentId: agent.id } });
  }

  // New token
  const tokenFor = ref<AgentRow | null>(null);
  /** The agent's grants, so a token can be narrowed to one of them. */
  const tokenGrants = ref<Grant[]>([]);
  const createdToken = ref<CreatedPersonalToken | null>(null);

  async function openToken(row: AgentRow): Promise<void> {
    tokenGrants.value = [];
    tokenFor.value = row;
    try {
      const access = await readQuery(cache, userAccessQuery(row.agent.id));
      // A later pick wins.
      if (tokenFor.value?.agent.id === row.agent.id)
        tokenGrants.value = access.grants;
    } catch {
      // The token can still reach everything the agent reaches.
    }
  }

  function tokenCreated(created: CreatedPersonalToken): void {
    tokenFor.value = null;
    // The plaintext stays here until it is acknowledged; the list refresh
    // only carries the new token's metadata.
    createdToken.value = created;
    void invalidate.personalTokensChanged();
  }

  // Transfer
  const transferring = ref<User | null>(null);

  async function transferred(): Promise<void> {
    transferring.value = null;
    await invalidate.usersChanged();
  }

  // Revoke
  const revoking = ref<AgentRow | null>(null);
  const revokeBusy = ref(false);
  const revokeError = ref<string | null>(null);

  function askRevoke(row: AgentRow): void {
    revokeError.value = null;
    revoking.value = row;
  }

  function cancelRevoke(): void {
    if (!revokeBusy.value) revoking.value = null;
  }

  async function confirmRevoke(): Promise<void> {
    const row = revoking.value;
    if (!row?.token) return;
    revokeBusy.value = true;
    revokeError.value = null;
    try {
      await runWithReauth(async () => {
        await revokePersonalToken(row.agent.id, row.token!.id);
      });
      revoking.value = null;
      await invalidate.personalTokensChanged();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      revokeError.value = errorMessage(cause, "Could not revoke the token.");
    } finally {
      revokeBusy.value = false;
    }
  }

  const actions = useAccountActions(() => invalidate.usersChanged());

  /** Root holds no personal tokens, so the hint to make one is for others. */
  const showsPersonalTokenHint = computed(
    () => !auth.isRoot && auth.can("tokens.personal"),
  );

  return {
    agents,
    people,
    roles,
    loadError,
    load,
    filter,
    ownerFilter,
    ownerFilters,
    rows,
    visibleRows,
    agentRole,
    canAdd,
    showsPersonalTokenHint,
    addOpen,
    added,
    tokenFor,
    tokenGrants,
    openToken,
    createdToken,
    revoking,
    revokeBusy,
    revokeError,
    askRevoke,
    cancelRevoke,
    confirmRevoke,
    tokenCreated,
    transferring,
    transferred,
    actions,
  };
}
