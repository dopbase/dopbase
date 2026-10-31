import { computed, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter, type RouteLocationRaw } from "vue-router";
import {
  deleteGroup,
  fetchGroup,
  removeGroupMember,
} from "~/services/access.api";
import type { GroupDetail, GroupMember } from "~/services/access.api";
import { fetchUsers } from "~/services/users.api";
import type { User } from "~/services/users.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { useAuthStore } from "~/stores/auth.store";
import { formatDate } from "~/utils/format";
import type { GrantPrincipal } from "~/components/app/GiveAccessDialog.controller";
import type { MembersTarget } from "./components/AddMembersDialog.controller";
import { memberLabel } from "./Groups.controller";

export type GroupTab = "members" | "access" | "danger";

const TAB_ROUTES: Record<GroupTab, string> = {
  members: "group",
  access: "group-access",
  danger: "group-danger",
};
const TAB_FOR_ROUTE = Object.fromEntries(
  Object.entries(TAB_ROUTES).map(([tab, route]) => [route, tab]),
) as Record<string, GroupTab>;

export interface MemberRow {
  member: GroupMember;
  /** The member's role, when the users list has them. */
  role: string;
  /** "Oct 20 by lead". */
  added: string;
  /** Their user or agent page. */
  to: RouteLocationRaw;
}

/** "lead" for lead@example.com: the Added column stays short. */
function shortName(email: string): string {
  return email.split("@")[0] || email;
}

export function addedLabel(member: GroupMember): string {
  const when = formatDate(member.addedAt);
  return member.addedBy
    ? `${when} by ${shortName(member.addedBy.email)}`
    : when;
}

/**
 * One group's page: Members (who's in it and who added them), Access (what
 * it reaches) and the Danger zone. Changing anything needs groups.manage.
 */
export function useGroupController() {
  const route = useRoute();
  const router = useRouter();
  const auth = useAuthStore();
  const { runWithReauth } = useReauthentication();

  const groupId = computed(() => String(route.params.groupId ?? ""));
  const activeTab = computed<GroupTab>(
    () => TAB_FOR_ROUTE[String(route.name)] ?? "members",
  );
  const canManage = computed(() => auth.can("groups.manage"));

  const detail = ref<GroupDetail | null>(null);
  const people = ref<User[]>([]);
  const loadError = ref<string | null>(null);
  const membersError = ref<string | null>(null);
  let version = 0;

  async function load(): Promise<void> {
    const current = ++version;
    const id = groupId.value;
    loadError.value = null;
    try {
      const [group, accounts] = await Promise.all([
        fetchGroup(id),
        auth.can("users.view")
          ? fetchUsers().catch(() => [] as User[])
          : Promise.resolve([] as User[]),
      ]);
      if (current !== version) return;
      detail.value = group;
      people.value = accounts;
    } catch (cause) {
      if (current !== version) return;
      loadError.value = errorMessage(cause, "Could not load this group.");
    }
  }

  watch(
    groupId,
    () => {
      detail.value = null;
      void load();
    },
    { immediate: true },
  );
  onUnmounted(() => {
    version++;
  });

  const group = computed(() => detail.value?.group ?? null);

  const subtitle = computed(() => {
    if (!detail.value) return "";
    const parts = [memberLabel(detail.value.members.length)];
    if (group.value?.description) parts.unshift(group.value.description);
    return parts.join(" · ");
  });

  const tabs = computed(() => {
    const all: { key: GroupTab; label: string }[] = [
      { key: "members", label: "Members" },
      { key: "access", label: "Access" },
    ];
    if (canManage.value) all.push({ key: "danger", label: "Danger zone" });
    return all.map((tab) => ({
      ...tab,
      to: {
        name: TAB_ROUTES[tab.key],
        params: { groupId: groupId.value },
      } as RouteLocationRaw,
    }));
  });

  const memberRows = computed<MemberRow[]>(() =>
    (detail.value?.members ?? []).map((member) => {
      const account = people.value.find((person) => person.id === member.id);
      const agent = account?.role === "ai_agent";
      return {
        member,
        role: account ? (account.roleName ?? account.role) : "",
        added: addedLabel(member),
        to: agent
          ? { name: "agent", params: { agentId: member.id } }
          : { name: "user", params: { userId: member.id } },
      };
    }),
  );

  const principal = computed<GrantPrincipal>(() => ({
    type: "group",
    id: groupId.value,
    label: group.value?.name ?? "",
  }));

  const accessSummary = computed(() =>
    group.value
      ? `Everyone in ${group.value.name} gets this access, but never more than their own role allows.`
      : "",
  );

  async function reload(): Promise<void> {
    try {
      detail.value = await fetchGroup(groupId.value);
    } catch (cause) {
      loadError.value = errorMessage(cause, "Could not reload the group.");
    }
  }

  // Members
  const membersTarget = ref<MembersTarget | null>(null);

  function openMembers(): void {
    if (!detail.value) return;
    membersTarget.value = {
      id: detail.value.group.id,
      name: detail.value.group.name,
      memberIds: detail.value.members.map((member) => member.id),
    };
  }

  async function membersAdded(): Promise<void> {
    membersTarget.value = null;
    await reload();
  }

  async function removeMember(member: GroupMember): Promise<void> {
    membersError.value = null;
    try {
      await runWithReauth(async () => {
        detail.value = await removeGroupMember(groupId.value, member.id);
      });
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      membersError.value = errorMessage(
        cause,
        `Could not take ${member.email} out of the group.`,
      );
    }
  }

  // Rename
  const renameOpen = ref(false);

  async function renamed(saved: GroupDetail): Promise<void> {
    renameOpen.value = false;
    detail.value = saved;
  }

  // Delete
  const deleteOpen = ref(false);
  const deleteBusy = ref(false);
  const deleteError = ref<string | null>(null);

  function askDelete(): void {
    deleteError.value = null;
    deleteOpen.value = true;
  }

  function cancelDelete(): void {
    if (!deleteBusy.value) deleteOpen.value = false;
  }

  async function confirmDelete(): Promise<void> {
    deleteBusy.value = true;
    deleteError.value = null;
    try {
      await runWithReauth(() => deleteGroup(groupId.value));
      deleteOpen.value = false;
      await router.push({ name: "groups" });
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      deleteError.value = errorMessage(cause, "Could not delete the group.");
    } finally {
      deleteBusy.value = false;
    }
  }

  return {
    groupId,
    activeTab,
    tabs,
    canManage,
    detail,
    group,
    people,
    loadError,
    load,
    reload,
    subtitle,
    memberRows,
    membersError,
    principal,
    accessSummary,
    membersTarget,
    openMembers,
    membersAdded,
    removeMember,
    renameOpen,
    renamed,
    deleteOpen,
    deleteBusy,
    deleteError,
    askDelete,
    cancelDelete,
    confirmDelete,
  };
}
