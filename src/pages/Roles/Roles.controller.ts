import { computed, onMounted, ref } from "vue";
import {
  ROOT_ONLY_CAPABILITIES,
  createRole,
  deleteRole,
  fetchRoles,
  updateRole,
} from "~/services/roles.api";
import type { Role, RoleSettings } from "~/services/roles.api";
import type { Capability } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";

/** What each capability lets a role's holders do, in the order shown. */
export const CAPABILITY_LABELS: Partial<Record<Capability, string>> = {
  "users.view": "See people and where their access comes from",
  "users.manage":
    "Add, change, and remove people with no more access than theirs",
  "groups.manage": "Create groups and change their members",
  "grants.manage": "Manage project access",
  "projects.create": "Create projects",
  "access.report": "See who can reach any project",
  "audit.read": "Read the audit log for their own projects",
  "audit.read_all": "Read the whole audit log",
  "audit.verify": "Verify the audit log has not been changed",
  "tokens.personal": "Hold personal tokens",
  "tokens.revoke_others": "Revoke other people's personal tokens",
  "instance.view": "See instance status",
};

/** Capabilities a custom role can hold. */
export const ROLE_CAPABILITIES = (
  Object.keys(CAPABILITY_LABELS) as Capability[]
).filter((capability) => !ROOT_ONLY_CAPABILITIES.includes(capability));

/**
 * Roles: the built-in ones and root's custom roles. Changing a role changes
 * what everyone holding it can do on their next request.
 */
export function useRolesController() {
  const { runWithReauth } = useReauthentication();
  const roles = ref<Role[] | null>(null);
  const error = ref<string | null>(null);
  const custom = computed(() =>
    (roles.value ?? []).filter((role) => !role.builtin),
  );

  async function load(): Promise<void> {
    error.value = null;
    try {
      roles.value = await fetchRoles();
    } catch (cause) {
      roles.value = [];
      error.value = errorMessage(cause, "Could not load roles.");
    }
  }

  /** Creates a role, or changes the custom role with `id`. */
  async function save(
    settings: RoleSettings,
    id: string | null,
  ): Promise<void> {
    await runWithReauth(async () => {
      if (id) {
        // A role's sign-in never changes, so it is not sent.
        await updateRole(id, { ...settings, signIn: undefined });
      } else {
        await createRole(settings);
      }
    });
    await load();
  }

  async function remove(role: Role): Promise<void> {
    await runWithReauth(() => deleteRole(role.id));
    await load();
  }

  onMounted(load);

  return { roles, custom, error, load, save, remove };
}
