import { computed, ref } from "vue";
import { useRouter, type RouteLocationRaw } from "vue-router";
import { useQuery } from "@pinia/colada";
import { deleteRole } from "~/services/roles.api";
import type { Role } from "~/services/roles.api";
import type { Capability } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { loadFailure, shownData } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { rolesQuery } from "~/queries/users";
import { levelMeter, type LevelMeter } from "~/utils/level";

/** What each capability lets a role's holders do. */
export const CAPABILITY_LABELS: Partial<Record<Capability, string>> = {
  "users.view": "See people and where their access comes from",
  "users.manage":
    "Add, change and remove people with no more access than theirs",
  "groups.manage": "Create groups and change their members",
  "projects.create": "Create projects",
  "grants.manage": "Manage project access",
  "audit.read": "Read the audit log for their own projects",
  "audit.read_all": "Read the whole audit log",
  "audit.verify": "Check the audit log hasn't been changed",
  "access.report": "See who can reach any project",
  "tokens.personal": "Hold personal tokens",
  "tokens.revoke_others": "Revoke other people's personal tokens",
  "instance.view": "See instance status",
};

/** The editor's checklist, grouped. Root-only powers aren't listed. */
export const CAPABILITY_GROUPS: {
  label: string;
  capabilities: Capability[];
}[] = [
  {
    label: "People",
    capabilities: ["users.view", "users.manage", "groups.manage"],
  },
  { label: "Projects", capabilities: ["projects.create", "grants.manage"] },
  {
    label: "Audit",
    capabilities: [
      "audit.read",
      "audit.read_all",
      "audit.verify",
      "access.report",
    ],
  },
  {
    label: "Tokens",
    capabilities: ["tokens.personal", "tokens.revoke_others"],
  },
  { label: "Instance", capabilities: ["instance.view"] },
];

/** Capabilities a custom role can hold, in checklist order. */
export const ROLE_CAPABILITIES: Capability[] = CAPABILITY_GROUPS.flatMap(
  (group) => group.capabilities,
);

export interface RoleRow {
  role: Role;
  reach: LevelMeter;
  /** Capabilities holders have, out of those a role can hold. */
  capabilityCount: number;
  signsIn: string;
  holders: number;
  /** The editor, or the read-only view for a built-in role. */
  to: RouteLocationRaw;
}

export function toRoleRow(role: Role): RoleRow {
  return {
    role,
    reach: levelMeter(role.maxLevel),
    capabilityCount: role.capabilities.filter((capability) =>
      ROLE_CAPABILITIES.includes(capability),
    ).length,
    signsIn: role.signIn === "token_only" ? "Token only" : "Password or SSO",
    holders: role.members,
    to: { name: "role", params: { roleId: role.id } },
  };
}

/**
 * Roles: the built-in ones and root's custom roles. Built-ins can't change;
 * duplicating one starts a custom role from its settings.
 */
export function useRolesController() {
  const router = useRouter();
  const { runWithReauth } = useReauthentication();

  const invalidate = useInvalidate();
  const roleList = useQuery(rolesQuery);
  const roleData = shownData<Role[]>(roleList);
  // Says so even above the old list, which may be out of date.
  const error = loadFailure(
    roleList,
    (cause) => errorMessage(cause, "Could not load roles."),
    { withData: true },
  );
  const roles = computed<Role[] | null>(
    () => roleData.value ?? (error.value ? [] : null),
  );

  /** Tries again after a failed load. */
  async function load(): Promise<void> {
    await roleList.refetch();
  }

  /** Built-ins first; the server already sorts each by rank, then name. */
  const rows = computed(() =>
    [...(roles.value ?? [])]
      .sort((a, b) => Number(b.builtin) - Number(a.builtin))
      .map(toRoleRow),
  );

  function duplicate(role: Role): void {
    void router.push({ name: "role-new", query: { from: role.id } });
  }

  // Delete
  const deleting = ref<Role | null>(null);
  const deleteBusy = ref(false);
  const deleteError = ref<string | null>(null);

  function askDelete(role: Role): void {
    deleteError.value = null;
    deleting.value = role;
  }

  function cancelDelete(): void {
    if (!deleteBusy.value) deleting.value = null;
  }

  async function confirmDelete(): Promise<void> {
    const role = deleting.value;
    if (!role) return;
    deleteBusy.value = true;
    deleteError.value = null;
    try {
      await runWithReauth(() => deleteRole(role.id));
      deleting.value = null;
      await invalidate.rolesChanged();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      deleteError.value = errorMessage(cause, "Could not delete the role.");
    } finally {
      deleteBusy.value = false;
    }
  }

  return {
    roles,
    error,
    load,
    rows,
    duplicate,
    deleting,
    deleteBusy,
    deleteError,
    askDelete,
    cancelDelete,
    confirmDelete,
  };
}
