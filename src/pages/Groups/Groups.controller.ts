import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { useQuery } from "@pinia/colada";
import { deleteGroup } from "~/services/access.api";
import type {
  AccountRef,
  GroupDetail,
  GroupSummary,
} from "~/services/access.api";
import type { User } from "~/services/users.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { useAuthStore } from "~/stores/auth.store";
import { groupsQuery } from "~/queries/access";
import { loadFailure, onlyWhen, shownData } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { usersQuery } from "~/queries/users";
import type { MembersTarget } from "./components/AddMembersDialog.controller";

/** Avatars shown in the Members column before "+N". */
export const MEMBER_PREVIEW = 3;

export interface GroupRow {
  group: GroupSummary;
  /** The first few members, for the avatar stack. */
  preview: AccountRef[];
  /** Members past the preview. */
  more: number;
  /** "2 members", "No members". */
  memberLabel: string;
  /** Where the group reaches, one entry per grant. */
  reach: string[];
  createdBy: string;
}

export function memberLabel(count: number): string {
  if (count === 0) return "No members";
  return count === 1 ? "1 member" : `${count} members`;
}

/** Where a grant reaches, as the Access column shows it. */
export function reachLabel(scopeLabel: string): string {
  return scopeLabel || "All projects";
}

export function toGroupRow(group: GroupSummary): GroupRow {
  return {
    group,
    preview: group.members.slice(0, MEMBER_PREVIEW),
    more: Math.max(0, group.members.length - MEMBER_PREVIEW),
    memberLabel: memberLabel(group.members.length),
    reach: group.grants.map((grant) => reachLabel(grant.scopeLabel)),
    createdBy: group.createdBy?.email ?? "Unknown",
  };
}

/**
 * Groups: named sets of accounts that get access together. The row menu
 * renames a group, adds members, gives it access or deletes it; the group
 * page holds the rest.
 */
export function useGroupsController() {
  const auth = useAuthStore();
  const router = useRouter();
  const { runWithReauth } = useReauthentication();

  const invalidate = useInvalidate();
  const markStale = useInvalidate({ refetch: false });

  const groupList = useQuery(groupsQuery);
  // Picking members needs users.view; without it Add members has nobody.
  const peopleList = useQuery(() =>
    onlyWhen(auth.can("users.view"), usersQuery),
  );
  const groupData = shownData<GroupSummary[]>(groupList);
  // Says so even above the old list, which may be out of date.
  const loadError = loadFailure(
    groupList,
    (cause) => errorMessage(cause, "Could not load groups."),
    { withData: true },
  );
  const groups = computed<GroupSummary[] | null>(
    () => groupData.value ?? (loadError.value ? [] : null),
  );
  const people = computed<User[]>(() => peopleList.data.value ?? []);
  const filter = ref("");
  const canManage = computed(() => auth.can("groups.manage"));

  /** Tries again after a failed load. */
  async function load(): Promise<void> {
    await Promise.all([groupList.refetch(), peopleList.refresh()]);
  }

  const rows = computed(() => (groups.value ?? []).map(toGroupRow));
  const visibleRows = computed(() => {
    const needle = filter.value.trim().toLowerCase();
    if (!needle) return rows.value;
    return rows.value.filter(
      (row) =>
        row.group.name.toLowerCase().includes(needle) ||
        row.group.description.toLowerCase().includes(needle),
    );
  });

  // New and rename
  const dialogOpen = ref(false);
  const editing = ref<GroupSummary | null>(null);

  function openNew(): void {
    editing.value = null;
    dialogOpen.value = true;
  }

  function openRename(group: GroupSummary): void {
    editing.value = group;
    dialogOpen.value = true;
  }

  async function saved(detail: GroupDetail): Promise<void> {
    const created = editing.value === null;
    dialogOpen.value = false;
    editing.value = null;
    if (created) {
      void markStale.groupsChanged();
      await router.push({
        name: "group",
        params: { groupId: detail.group.id },
      });
      return;
    }
    await invalidate.groupsChanged();
  }

  // Add members
  const membersTarget = ref<MembersTarget | null>(null);

  function openMembers(group: GroupSummary): void {
    membersTarget.value = {
      id: group.id,
      name: group.name,
      memberIds: group.members.map((member) => member.id),
    };
  }

  async function membersAdded(): Promise<void> {
    membersTarget.value = null;
    await invalidate.groupsChanged();
  }

  // Give access happens on its own page.
  function giveAccess(group: GroupSummary): void {
    void router.push({
      name: "group-access-give",
      params: { groupId: group.id },
    });
  }

  // Delete
  const deleting = ref<GroupSummary | null>(null);
  const deleteBusy = ref(false);
  const deleteError = ref<string | null>(null);

  function askDelete(group: GroupSummary): void {
    deleteError.value = null;
    deleting.value = group;
  }

  function cancelDelete(): void {
    if (!deleteBusy.value) deleting.value = null;
  }

  async function confirmDelete(): Promise<void> {
    const group = deleting.value;
    if (!group) return;
    deleteBusy.value = true;
    deleteError.value = null;
    try {
      await runWithReauth(() => deleteGroup(group.id));
      deleting.value = null;
      await invalidate.groupsChanged();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      deleteError.value = errorMessage(cause, "Could not delete the group.");
    } finally {
      deleteBusy.value = false;
    }
  }

  return {
    groups,
    people,
    loadError,
    load,
    filter,
    canManage,
    rows,
    visibleRows,
    dialogOpen,
    editing,
    openNew,
    openRename,
    saved,
    membersTarget,
    openMembers,
    membersAdded,
    giveAccess,
    deleting,
    deleteBusy,
    deleteError,
    askDelete,
    cancelDelete,
    confirmDelete,
  };
}

export type GroupsController = ReturnType<typeof useGroupsController>;
