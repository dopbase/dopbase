import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { useQuery } from "@pinia/colada";
import type { User } from "~/services/users.api";
import type { Role } from "~/services/roles.api";
import { useAuthStore } from "~/stores/auth.store";
import { authMethodsQuery } from "~/queries/auth";
import { loadFailure, shownData } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { rolesQuery, usersQuery } from "~/queries/users";
import { formatRelativeTime } from "~/utils/format";
import { isAgent, roleOf, signInMethod, type SignInMethod } from "./people";
import { useAccountActions, type AccountActionKind } from "./account-actions";

export interface UserRow {
  user: User;
  roleName: string;
  /** A custom role, not one that ships with Dopbase. */
  customRole: boolean;
  isRoot: boolean;
  isYou: boolean;
  signIn: SignInMethod;
  lastSignIn: string;
  /** Row menu actions you may take on this account. */
  actions: AccountActionKind[];
  canEdit: boolean;
}

/** Filter values for the Status chip. */
export const STATUS_FILTERS = [
  { value: "active", label: "Active" },
  { value: "disabled", label: "Disabled" },
  { value: "sso", label: "Single sign-on" },
] as const;

/**
 * Users page controller: the people who sign in. AI agents have their own
 * page. Filters by email, role and status; the row menu edits, opens
 * access, unlinks single sign-on, disables or deletes.
 *
 * What the menu offers follows `users.manage`; root and your own account
 * can't be changed from here, as the server refuses that too.
 */
export function useUsersController() {
  const auth = useAuthStore();
  const router = useRouter();
  const invalidate = useInvalidate();
  const markStale = useInvalidate({ refetch: false });

  const accountList = useQuery(usersQuery);
  // Roles and sign-in methods only add detail, so a failure leaves them out.
  const roleList = useQuery(rolesQuery);
  const methods = useQuery(authMethodsQuery);
  const accounts = shownData<User[]>(accountList);
  // Says so even above the old list, which may be out of date.
  const loadError = loadFailure(accountList, "Could not load users.", {
    withData: true,
  });
  const users = computed<User[] | null>(() => {
    if (accounts.value)
      return accounts.value.filter((account) => !isAgent(account));
    return loadError.value ? [] : null;
  });
  const roles = computed<Role[]>(() => roleList.data.value ?? []);
  const ssoEnabled = computed(() => methods.data.value?.sso != null);

  const filter = ref("");
  const roleFilter = ref<string | null>(null);
  const statusFilter = ref<string | null>(null);

  const canManage = computed(() => auth.can("users.manage"));
  /**
   * Adding and editing people needs RBAC on the server. Without it, admins
   * still disable, enable, delete and unlink accounts.
   */
  const canManagePeople = computed(
    () => canManage.value && auth.hasFeature("rbac"),
  );

  /** Tries again after a failed load. */
  async function load(): Promise<void> {
    await Promise.all([
      accountList.refetch(),
      roleList.refresh(),
      methods.refresh(),
    ]);
  }

  const rows = computed<UserRow[]>(() =>
    (users.value ?? []).map((user) => {
      const role = roleOf(user, roles.value);
      const isRoot = user.role === "root";
      const isYou = user.id === auth.session?.adminId;
      const fixed = isRoot || isYou || !canManage.value;
      const actions: AccountActionKind[] = [];
      if (!fixed) {
        if (user.ssoLinked) actions.push("unlink");
        actions.push(user.status === "disabled" ? "enable" : "disable");
        actions.push("delete");
      }
      return {
        user,
        roleName: user.roleName ?? role?.name ?? user.role,
        customRole: role ? !role.builtin : false,
        isRoot,
        isYou,
        signIn: signInMethod(user),
        lastSignIn: user.lastSignInAt
          ? formatRelativeTime(user.lastSignInAt)
          : "Never",
        actions,
        canEdit: !fixed && canManagePeople.value,
      };
    }),
  );

  const roleFilters = computed(() =>
    [...new Set(rows.value.map((row) => row.roleName))]
      .sort()
      .map((name) => ({ value: name, label: name })),
  );

  const visibleRows = computed(() => {
    const needle = filter.value.trim().toLowerCase();
    return rows.value.filter((row) => {
      if (needle && !row.user.email.toLowerCase().includes(needle))
        return false;
      if (roleFilter.value && row.roleName !== roleFilter.value) return false;
      if (statusFilter.value === "active" && row.user.status !== "active")
        return false;
      if (statusFilter.value === "disabled" && row.user.status !== "disabled")
        return false;
      if (statusFilter.value === "sso" && !row.user.ssoLinked) return false;
      return true;
    });
  });

  // Add and edit
  const dialogOpen = ref(false);
  const editing = ref<User | null>(null);

  function openAdd(): void {
    editing.value = null;
    dialogOpen.value = true;
  }

  function openEdit(user: User): void {
    editing.value = user;
    dialogOpen.value = true;
  }

  /** A new account opens on its Access tab, where it gets its first grant. */
  async function saved(user: User): Promise<void> {
    const added = editing.value === null;
    dialogOpen.value = false;
    if (added) {
      void markStale.usersChanged();
      await router.push({ name: "user-access", params: { userId: user.id } });
      return;
    }
    await invalidate.usersChanged();
  }

  const actions = useAccountActions(() => invalidate.usersChanged());

  return {
    users,
    roles,
    ssoEnabled,
    loadError,
    load,
    canManage,
    canManagePeople,
    filter,
    roleFilter,
    statusFilter,
    roleFilters,
    rows,
    visibleRows,
    dialogOpen,
    editing,
    openAdd,
    openEdit,
    saved,
    actions,
  };
}
