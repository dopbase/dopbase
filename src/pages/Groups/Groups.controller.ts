import { computed, onMounted, ref } from "vue";
import {
  addGroupMember,
  createGroup,
  deleteGroup,
  fetchGroup,
  fetchGroups,
  removeGroupMember,
} from "~/services/access.api";
import type { Group, GroupDetail } from "~/services/access.api";
import { fetchUsers } from "~/services/users.api";
import type { User } from "~/services/users.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { useAuthStore } from "~/stores/auth.store";

/**
 * Groups: named sets of users that receive grants together. Changes run
 * through the shared password confirmation, since the server wants a
 * recent sign-in.
 */
export function useGroupsController() {
  const auth = useAuthStore();
  const { runWithReauth } = useReauthentication();

  const groups = ref<Group[] | null>(null);
  const users = ref<User[]>([]);
  const loadError = ref<string | null>(null);
  const open = ref<GroupDetail | null>(null);
  const detailError = ref<string | null>(null);
  const canManage = computed(() => auth.can("groups.manage"));

  /** Users who can still join the open group. Root never joins groups. */
  const candidates = computed(() => {
    const members = new Set(open.value?.members.map((member) => member.id));
    return users.value.filter(
      (user) => user.role !== "root" && !members.has(user.id),
    );
  });

  async function load(): Promise<void> {
    loadError.value = null;
    try {
      groups.value = await fetchGroups();
    } catch (cause) {
      groups.value = [];
      loadError.value = errorMessage(cause, "Could not load groups.");
    }
    // Picking members needs users.view; without it the list stays empty.
    if (auth.can("users.view")) {
      users.value = await fetchUsers().catch(() => []);
    }
  }

  async function show(id: string): Promise<void> {
    detailError.value = null;
    try {
      open.value = await fetchGroup(id);
    } catch (cause) {
      detailError.value = errorMessage(cause, "Could not load the group.");
    }
  }

  function close(): void {
    open.value = null;
    detailError.value = null;
  }

  async function create(name: string, description: string): Promise<void> {
    let created: GroupDetail | null = null;
    await runWithReauth(async () => {
      created = await createGroup(name, description);
    });
    await load();
    if (created) open.value = created;
  }

  /** Runs a change to the open group and shows the group it returns. */
  async function change(
    action: (id: string) => Promise<GroupDetail>,
    fallback: string,
  ): Promise<void> {
    if (!open.value) return;
    const id = open.value.group.id;
    detailError.value = null;
    try {
      await runWithReauth(async () => {
        open.value = await action(id);
      });
      await load();
    } catch (cause) {
      detailError.value = errorMessage(cause, fallback);
    }
  }

  function addMember(userId: string): Promise<void> {
    return change(
      (id) => addGroupMember(id, userId),
      "Could not add that member.",
    );
  }

  function removeMember(userId: string): Promise<void> {
    return change(
      (id) => removeGroupMember(id, userId),
      "Could not remove that member.",
    );
  }

  async function remove(): Promise<void> {
    if (!open.value) return;
    const id = open.value.group.id;
    await runWithReauth(() => deleteGroup(id));
    close();
    await load();
  }

  onMounted(load);

  return {
    groups,
    users,
    candidates,
    loadError,
    open,
    detailError,
    canManage,
    load,
    show,
    close,
    create,
    addMember,
    removeMember,
    remove,
  };
}

export type GroupsController = ReturnType<typeof useGroupsController>;
