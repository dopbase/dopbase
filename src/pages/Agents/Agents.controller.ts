import { computed, onMounted, ref } from "vue";
import { useRouter, type RouteLocationRaw } from "vue-router";
import { fetchUsers } from "~/services/users.api";
import type { User } from "~/services/users.api";
import { fetchRoles } from "~/services/roles.api";
import type { Role } from "~/services/roles.api";
import {
  listPersonalTokens,
  revokePersonalToken,
} from "~/services/personal-tokens.api";
import { fetchUserAccess } from "~/services/access.api";
import type { Grant } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import type {
  CreatedPersonalToken,
  PersonalToken,
} from "~/services/personal-tokens.api";
import { useAuthStore } from "~/stores/auth.store";
import { formatDate, formatRelativeTime } from "~/utils/format";
import { levelMeter, type LevelMeter } from "~/utils/level";
import { isAgent, tokenState } from "~/pages/Users/people";
import { useAccountActions } from "~/pages/Users/account-actions";

/** Token state as the Token column shows it. */
export type AgentTokenStatus = "active" | "expiring" | "none" | "unknown";

/** A token this close to expiring is flagged. */
const EXPIRING_SOON_MS = 7 * 24 * 60 * 60 * 1000;

export interface AgentRow {
  agent: User;
  owner: string;
  /** The most its role lets it reach. */
  reach: LevelMeter | null;
  token: PersonalToken | null;
  status: AgentTokenStatus;
  /** "active", "expires in 2 days", "no token". */
  statusLabel: string;
  expires: string;
  lastUsed: string;
  /** You own it, or may manage users. */
  canManage: boolean;
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

  const agents = ref<User[] | null>(null);
  const people = ref<User[]>([]);
  const roles = ref<Role[]>([]);
  const tokens = ref<Record<string, PersonalToken[] | null>>({});
  const loadError = ref<string | null>(null);
  const filter = ref("");
  const ownerFilter = ref<string | null>(null);

  async function load(): Promise<void> {
    loadError.value = null;
    try {
      const [accounts, available] = await Promise.all([
        fetchUsers(),
        fetchRoles().catch(() => [] as Role[]),
      ]);
      roles.value = available;
      people.value = accounts.filter((account) => !isAgent(account));
      const list = accounts.filter(isAgent);
      const lists = await Promise.allSettled(
        list.map((agent) => listPersonalTokens(agent.id)),
      );
      tokens.value = Object.fromEntries(
        list.map((agent, index) => {
          const result = lists[index];
          return [
            agent.id,
            result?.status === "fulfilled" ? result.value : null,
          ];
        }),
      );
      agents.value = list;
    } catch {
      agents.value ??= [];
      loadError.value = "Could not load AI agents.";
    }
  }
  onMounted(load);

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
      const role = roles.value.find(
        (candidate) => candidate.id === agent.roleId,
      );
      const owner = people.value.find((person) => person.id === agent.ownerId);
      return {
        agent,
        owner: owner?.email ?? (agent.ownerId ? "Unknown" : "No owner"),
        reach: role ? levelMeter(role.maxLevel) : null,
        token,
        status,
        statusLabel,
        expires: token?.expiresAt ? formatDate(token.expiresAt) : "—",
        lastUsed: token?.lastUsedAt
          ? formatRelativeTime(token.lastUsedAt)
          : "Never",
        canManage:
          auth.can("users.manage") || agent.ownerId === auth.session?.adminId,
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
      tokenGrants.value = (await fetchUserAccess(row.agent.id)).grants;
    } catch {
      // The token can still reach everything the agent reaches.
    }
  }

  function tokenCreated(created: CreatedPersonalToken): void {
    tokenFor.value = null;
    createdToken.value = created;
    void load();
  }

  // Transfer
  const transferring = ref<User | null>(null);

  async function transferred(): Promise<void> {
    transferring.value = null;
    await load();
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
      await load();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      revokeError.value = errorMessage(cause, "Could not revoke the token.");
    } finally {
      revokeBusy.value = false;
    }
  }

  const actions = useAccountActions(load);

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
