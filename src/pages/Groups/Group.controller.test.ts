import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { addedLabel, useGroupController } from "./Group.controller";
import * as accessApi from "~/services/access.api";
import * as usersApi from "~/services/users.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import {
  groupDetailFixture,
  lead,
  maya,
  memberFixture,
  reviewBot,
  settle,
  signInAs,
} from "~/tests/people";

const router = vi.hoisted(() => ({ push: vi.fn() }));
// The route is reactive so tests can switch tabs and groups under a
// mounted controller. Tests reach it through `liveRoute()`.
vi.mock("vue-router", async () => {
  const { reactive } = await import("vue");
  const live = reactive({
    name: "group",
    params: { groupId: "grp_backend" } as Record<string, string>,
  });
  return { useRoute: () => live, useRouter: () => router, __route: live };
});

vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  fetchGroup: vi.fn(),
  removeGroupMember: vi.fn(),
  deleteGroup: vi.fn(),
}));
vi.mock("~/services/users.api");

async function liveRoute(): Promise<{
  name: string;
  params: Record<string, string>;
}> {
  const mod = (await import("vue-router")) as unknown as {
    __route: { name: string; params: Record<string, string> };
  };
  return mod.__route;
}

const detail = groupDetailFixture({
  members: [
    memberFixture(),
    memberFixture({
      id: "usr_bot",
      email: "review-bot",
      addedBy: null,
    }),
  ],
});

beforeEach(async () => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  const current = await liveRoute();
  current.name = "group";
  current.params = { groupId: "grp_backend" };
  vi.mocked(accessApi.fetchGroup).mockResolvedValue(detail);
  vi.mocked(usersApi.fetchUsers).mockResolvedValue([lead, maya, reviewBot]);
});

describe("addedLabel", () => {
  it("names who added a member by the start of their email", () => {
    expect(addedLabel(memberFixture())).toMatch(/ by lead$/);
  });

  it("drops the name when the account that added them is gone", () => {
    expect(addedLabel(memberFixture({ addedBy: null }))).not.toContain(" by ");
  });
});

describe("useGroupController", () => {
  it("loads the group with each member's role and page", async () => {
    signInAs(["users.view", "groups.manage"]);
    const { controller } = mountController(useGroupController);
    await settle();
    expect(accessApi.fetchGroup).toHaveBeenCalledWith("grp_backend");
    expect(controller.group.value?.name).toBe("Backend");
    expect(controller.subtitle.value).toBe("Payments service team · 2 members");
    expect(
      controller.memberRows.value.map((row) => [
        row.member.email,
        row.role,
        row.to,
      ]),
    ).toEqual([
      [
        "maya@example.com",
        "Member",
        { name: "user", params: { userId: "usr_maya" } },
      ],
      [
        "review-bot",
        "AI agent",
        { name: "agent", params: { agentId: "usr_bot" } },
      ],
    ]);
  });

  it("leaves roles blank for a viewer who can't list people", async () => {
    signInAs(["groups.manage"]);
    const { controller } = mountController(useGroupController);
    await settle();
    expect(usersApi.fetchUsers).not.toHaveBeenCalled();
    expect(controller.memberRows.value.map((row) => row.role)).toEqual([
      "",
      "",
    ]);
  });

  it("shows the Danger zone tab only to someone who manages groups", async () => {
    signInAs(["users.view"]);
    const viewer = mountController(useGroupController).controller;
    expect(viewer.tabs.value.map((tab) => tab.key)).toEqual([
      "members",
      "access",
    ]);
    signInAs(["groups.manage"]);
    expect(viewer.tabs.value.map((tab) => tab.key)).toEqual([
      "members",
      "access",
      "danger",
    ]);
    expect(viewer.tabs.value[1]?.to).toEqual({
      name: "group-access",
      params: { groupId: "grp_backend" },
    });
  });

  it("picks the tab from the route", async () => {
    signInAs(["groups.manage"]);
    const { controller } = mountController(useGroupController);
    const current = await liveRoute();
    expect(controller.activeTab.value).toBe("members");
    current.name = "group-access";
    expect(controller.activeTab.value).toBe("access");
    current.name = "group-danger";
    expect(controller.activeTab.value).toBe("danger");
  });

  it("gives access as the group and says members stay capped by their role", async () => {
    signInAs(["groups.manage"]);
    const { controller } = mountController(useGroupController);
    await settle();
    expect(controller.principal.value).toEqual({
      type: "group",
      id: "grp_backend",
      label: "Backend",
    });
    expect(controller.accessSummary.value).toContain(
      "never more than their own role allows",
    );
  });

  it("says why it couldn't load the group", async () => {
    signInAs(["groups.manage"]);
    vi.mocked(accessApi.fetchGroup).mockRejectedValueOnce(
      new ApiError(404, { GROUP_NOT_FOUND: "That group doesn't exist." }),
    );
    const { controller } = mountController(useGroupController);
    await settle();
    expect(controller.loadError.value).toBe("That group doesn't exist.");
    expect(controller.detail.value).toBeNull();
  });

  it("loads the new group when the route moves to another one", async () => {
    signInAs(["groups.manage"]);
    const other = groupDetailFixture({
      group: { ...detail.group, id: "grp_data", name: "Data" },
    });
    const { controller } = mountController(useGroupController);
    await settle();
    vi.mocked(accessApi.fetchGroup).mockResolvedValueOnce(other);
    (await liveRoute()).params = { groupId: "grp_data" };
    await settle();
    expect(accessApi.fetchGroup).toHaveBeenLastCalledWith("grp_data");
    expect(controller.group.value?.name).toBe("Data");
  });

  it("ignores a slow answer for a group you already left", async () => {
    signInAs(["groups.manage"]);
    let slow!: (value: accessApi.GroupDetail) => void;
    vi.mocked(accessApi.fetchGroup).mockReturnValueOnce(
      new Promise((resolve) => (slow = resolve)),
    );
    const { controller } = mountController(useGroupController);
    const fresh = groupDetailFixture({
      group: { ...detail.group, id: "grp_data", name: "Data" },
    });
    vi.mocked(accessApi.fetchGroup).mockResolvedValueOnce(fresh);
    (await liveRoute()).params = { groupId: "grp_data" };
    await settle();
    slow(detail);
    await settle();
    expect(controller.group.value?.name).toBe("Data");
  });

  it("adds members to this group, skipping the ones already in it", async () => {
    signInAs(["groups.manage"]);
    const { controller } = mountController(useGroupController);
    await settle();
    controller.openMembers();
    expect(controller.membersTarget.value).toEqual({
      id: "grp_backend",
      name: "Backend",
      memberIds: ["usr_maya", "usr_bot"],
    });
    await controller.membersAdded();
    expect(controller.membersTarget.value).toBeNull();
    expect(accessApi.fetchGroup).toHaveBeenCalledTimes(2);
  });

  it("takes a member out and shows the group the server returns", async () => {
    signInAs(["groups.manage"]);
    vi.mocked(accessApi.removeGroupMember).mockResolvedValueOnce(
      groupDetailFixture({ members: [memberFixture()] }),
    );
    const { controller } = mountController(useGroupController);
    await settle();
    await controller.removeMember(detail.members[1]!);
    expect(accessApi.removeGroupMember).toHaveBeenCalledWith(
      "grp_backend",
      "usr_bot",
    );
    expect(controller.memberRows.value).toHaveLength(1);
  });

  it("names the member it couldn't take out", async () => {
    signInAs(["groups.manage"]);
    // A failure with no message of its own falls back to the named one.
    vi.mocked(accessApi.removeGroupMember).mockRejectedValueOnce(new Error(""));
    const { controller } = mountController(useGroupController);
    await settle();
    await controller.removeMember(detail.members[0]!);
    expect(controller.membersError.value).toBe(
      "Could not take maya@example.com out of the group.",
    );
    expect(controller.memberRows.value).toHaveLength(2);
  });

  it("shows the new name after a rename without reloading", async () => {
    signInAs(["groups.manage"]);
    const { controller } = mountController(useGroupController);
    await settle();
    controller.renameOpen.value = true;
    await controller.renamed(
      groupDetailFixture({ group: { ...detail.group, name: "Platform" } }),
    );
    expect(controller.renameOpen.value).toBe(false);
    expect(controller.group.value?.name).toBe("Platform");
    expect(accessApi.fetchGroup).toHaveBeenCalledTimes(1);
  });

  it("goes back to the list after deleting the group", async () => {
    signInAs(["groups.manage"]);
    vi.mocked(accessApi.deleteGroup).mockResolvedValueOnce();
    const { controller } = mountController(useGroupController);
    await settle();
    controller.askDelete();
    await controller.confirmDelete();
    expect(accessApi.deleteGroup).toHaveBeenCalledWith("grp_backend");
    expect(controller.deleteOpen.value).toBe(false);
    expect(router.push).toHaveBeenCalledWith({ name: "groups" });
  });

  it("stays put with the reason when delete fails", async () => {
    signInAs(["groups.manage"]);
    vi.mocked(accessApi.deleteGroup).mockRejectedValueOnce(
      new ApiError(403, { ACCESS_DENIED: "You can't delete groups." }),
    );
    const { controller } = mountController(useGroupController);
    await settle();
    controller.askDelete();
    await controller.confirmDelete();
    expect(controller.deleteOpen.value).toBe(true);
    expect(controller.deleteError.value).toBe("You can't delete groups.");
    expect(router.push).not.toHaveBeenCalled();
  });
});
