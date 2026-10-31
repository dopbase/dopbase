import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import {
  memberLabel,
  toGroupRow,
  useGroupsController,
} from "./Groups.controller";
import * as accessApi from "~/services/access.api";
import * as usersApi from "~/services/users.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import {
  grantFixture,
  groupDetailFixture,
  groupSummaryFixture,
  lead,
  maya,
  reader,
  root,
  settle,
  signInAs,
} from "~/tests/people";

const router = vi.hoisted(() => ({ push: vi.fn() }));
vi.mock("vue-router", () => ({ useRouter: () => router }));

vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  fetchGroups: vi.fn(),
  deleteGroup: vi.fn(),
}));
vi.mock("~/services/users.api");

const backend = groupSummaryFixture({
  members: [
    { id: "usr_maya", email: "maya@example.com" },
    { id: "usr_reader", email: "reader@example.com" },
    { id: "usr_sam", email: "sam@example.com" },
    { id: "usr_kai", email: "kai@example.com" },
  ],
  memberCount: 4,
  grants: [
    grantFixture({ principalType: "group", principalId: "grp_backend" }),
  ],
});
const contractors = groupSummaryFixture({
  id: "grp_contractors",
  name: "Contractors",
  description: "",
  createdBy: null,
});

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(accessApi.fetchGroups).mockResolvedValue([backend, contractors]);
  vi.mocked(usersApi.fetchUsers).mockResolvedValue([root, lead, maya, reader]);
});

describe("group rows", () => {
  it("stacks three avatars and counts the rest", () => {
    const row = toGroupRow(backend);
    expect(row.preview.map((member) => member.email)).toEqual([
      "maya@example.com",
      "reader@example.com",
      "sam@example.com",
    ]);
    expect(row.more).toBe(1);
    expect(row.memberLabel).toBe("4 members");
    expect(row.reach).toEqual(["payments"]);
    expect(row.createdBy).toBe("lead@example.com");
  });

  it("shows an empty group with no access and a deleted creator", () => {
    const row = toGroupRow(contractors);
    expect(row.preview).toEqual([]);
    expect(row.more).toBe(0);
    expect(row.memberLabel).toBe("No members");
    expect(row.reach).toEqual([]);
    expect(row.createdBy).toBe("Unknown");
  });

  it("calls an all-projects grant what it is", () => {
    const row = toGroupRow(
      groupSummaryFixture({
        grants: [grantFixture({ scopeType: "all", scopeLabel: "" })],
      }),
    );
    expect(row.reach).toEqual(["All projects"]);
  });

  it("says one member, not one members", () => {
    expect(memberLabel(1)).toBe("1 member");
  });
});

describe("useGroupsController", () => {
  it("loads groups and the people who could join them", async () => {
    signInAs(["users.view", "groups.manage"]);
    const { controller } = mountController(useGroupsController);
    await controller.load();
    expect(controller.rows.value.map((row) => row.group.name)).toEqual([
      "Backend",
      "Contractors",
    ]);
    expect(controller.people.value).toHaveLength(4);
    expect(controller.canManage.value).toBe(true);
  });

  it("doesn't ask for people without users.view", async () => {
    signInAs(["groups.manage"]);
    const { controller } = mountController(useGroupsController);
    await controller.load();
    expect(usersApi.fetchUsers).not.toHaveBeenCalled();
    expect(controller.people.value).toEqual([]);
  });

  it("hides the row menu from someone who can't manage groups", async () => {
    signInAs(["users.view"]);
    const { controller } = mountController(useGroupsController);
    expect(controller.canManage.value).toBe(false);
  });

  it("filters by name or description", async () => {
    signInAs(["groups.manage"]);
    const { controller } = mountController(useGroupsController);
    await controller.load();
    controller.filter.value = "service team";
    expect(controller.visibleRows.value.map((row) => row.group.id)).toEqual([
      "grp_backend",
    ]);
    controller.filter.value = "CONTRACT";
    expect(controller.visibleRows.value.map((row) => row.group.id)).toEqual([
      "grp_contractors",
    ]);
  });

  it("keeps the last list and says so when a reload fails", async () => {
    signInAs(["groups.manage"]);
    const { controller } = mountController(useGroupsController);
    await controller.load();
    vi.mocked(accessApi.fetchGroups).mockRejectedValueOnce(
      new ApiError(500, { INTERNAL: "Something broke." }),
    );
    await controller.load();
    expect(controller.groups.value).toHaveLength(2);
    expect(controller.loadError.value).toBe("Something broke.");
  });

  it("opens a new group's page once it's created", async () => {
    signInAs(["groups.manage"]);
    const { controller } = mountController(useGroupsController);
    controller.openNew();
    expect(controller.dialogOpen.value).toBe(true);
    expect(controller.editing.value).toBeNull();
    await controller.saved(groupDetailFixture());
    expect(controller.dialogOpen.value).toBe(false);
    expect(router.push).toHaveBeenCalledWith({
      name: "group",
      params: { groupId: "grp_backend" },
    });
  });

  it("stays on the list after a rename and reloads it", async () => {
    signInAs(["groups.manage"]);
    const { controller } = mountController(useGroupsController);
    await settle();
    vi.mocked(accessApi.fetchGroups).mockClear();
    controller.openRename(backend);
    expect(controller.editing.value).toStrictEqual(backend);
    await controller.saved(groupDetailFixture());
    expect(router.push).not.toHaveBeenCalled();
    expect(accessApi.fetchGroups).toHaveBeenCalledTimes(1);
  });

  it("adds members to the group the row belongs to, skipping current ones", async () => {
    signInAs(["groups.manage"]);
    const { controller } = mountController(useGroupsController);
    controller.openMembers(backend);
    expect(controller.membersTarget.value).toEqual({
      id: "grp_backend",
      name: "Backend",
      memberIds: ["usr_maya", "usr_reader", "usr_sam", "usr_kai"],
    });
    await controller.membersAdded();
    expect(controller.membersTarget.value).toBeNull();
  });

  it("gives access to the group as a group", async () => {
    signInAs(["groups.manage"]);
    const { controller } = mountController(useGroupsController);
    controller.accessFor.value = backend;
    expect(controller.accessPrincipal.value).toEqual({
      type: "group",
      id: "grp_backend",
      label: "Backend",
    });
    await controller.accessGiven();
    expect(controller.accessFor.value).toBeNull();
    expect(controller.accessPrincipal.value).toBeNull();
  });

  it("deletes a group after the typed confirmation", async () => {
    signInAs(["groups.manage"]);
    vi.mocked(accessApi.deleteGroup).mockResolvedValueOnce();
    const { controller } = mountController(useGroupsController);
    controller.askDelete(contractors);
    await controller.confirmDelete();
    expect(accessApi.deleteGroup).toHaveBeenCalledWith("grp_contractors");
    expect(controller.deleting.value).toBeNull();
    expect(controller.deleteBusy.value).toBe(false);
  });

  it("keeps the dialog open with the reason when delete fails", async () => {
    signInAs(["groups.manage"]);
    vi.mocked(accessApi.deleteGroup).mockRejectedValueOnce(
      new ApiError(404, { GROUP_NOT_FOUND: "That group is gone." }),
    );
    const { controller } = mountController(useGroupsController);
    controller.askDelete(contractors);
    await controller.confirmDelete();
    expect(controller.deleting.value).toStrictEqual(contractors);
    expect(controller.deleteError.value).toBe("That group is gone.");
  });

  it("won't close the delete dialog while it's working", async () => {
    signInAs(["groups.manage"]);
    let finish!: () => void;
    vi.mocked(accessApi.deleteGroup).mockReturnValueOnce(
      new Promise((resolve) => (finish = resolve)),
    );
    const { controller } = mountController(useGroupsController);
    controller.askDelete(contractors);
    const pending = controller.confirmDelete();
    controller.cancelDelete();
    expect(controller.deleting.value).toStrictEqual(contractors);
    finish();
    await pending;
    expect(controller.deleting.value).toBeNull();
  });
});
