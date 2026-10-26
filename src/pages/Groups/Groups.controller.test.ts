import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useGroupsController } from "./Groups.controller";
import * as accessApi from "~/services/access.api";
import * as usersApi from "~/services/users.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";
import { mountController } from "~/tests/mount-controller";

vi.mock("~/services/access.api");
vi.mock("~/services/users.api");

const backend: accessApi.GroupDetail = {
  group: {
    id: "grp_backend",
    name: "Backend",
    description: "",
    memberCount: 1,
    createdAt: "",
    updatedAt: "",
  },
  members: [{ id: "usr_member", email: "member@example.com", addedAt: "" }],
  grants: [],
};

const user = (id: string, role: "root" | "admin" | "member") => ({
  id,
  email: `${id}@example.com`,
  role,
  createdAt: "",
  updatedAt: "",
  hasPassword: true,
  ssoLinked: false,
});

function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve));
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  useAuthStore().session = {
    adminId: "usr_admin",
    email: "admin@example.com",
    sessionKind: "browser",
    recentAuthentication: true,
    role: "admin",
    capabilities: ["users.view", "groups.manage"],
  };
  vi.mocked(accessApi.fetchGroups).mockResolvedValue([backend.group]);
  vi.mocked(accessApi.fetchGroup).mockResolvedValue(backend);
  vi.mocked(usersApi.fetchUsers).mockResolvedValue([
    user("usr_root", "root"),
    user("usr_member", "member"),
    user("usr_other", "member"),
  ]);
});

describe("useGroupsController", () => {
  it("offers only users who can still join the open group", async () => {
    const { controller } = mountController(useGroupsController);
    await settle();
    expect(controller.groups.value).toEqual([backend.group]);
    await controller.show("grp_backend");
    expect(controller.candidates.value.map((item) => item.id)).toEqual([
      "usr_other",
    ]);
  });

  it("adds and removes members on the open group", async () => {
    vi.mocked(accessApi.addGroupMember).mockResolvedValueOnce({
      ...backend,
      members: [
        ...backend.members,
        { id: "usr_other", email: "usr_other@example.com", addedAt: "" },
      ],
    });
    vi.mocked(accessApi.removeGroupMember).mockRejectedValueOnce(
      new ApiError(403, {
        ACCESS_EXCEEDS_YOURS: "This group has access you don't manage.",
      }),
    );
    const { controller } = mountController(useGroupsController);
    await settle();
    await controller.show("grp_backend");
    await controller.addMember("usr_other");
    expect(accessApi.addGroupMember).toHaveBeenCalledWith(
      "grp_backend",
      "usr_other",
    );
    expect(controller.open.value?.members).toHaveLength(2);
    await controller.removeMember("usr_member");
    expect(controller.detailError.value).toBe(
      "This group has access you don't manage.",
    );
  });

  it("skips the user list without users.view", async () => {
    useAuthStore().session!.capabilities = ["groups.manage"];
    const { controller } = mountController(useGroupsController);
    await settle();
    expect(usersApi.fetchUsers).not.toHaveBeenCalled();
    expect(controller.canManage.value).toBe(true);
  });

  it("closes the group it deletes", async () => {
    vi.mocked(accessApi.deleteGroup).mockResolvedValueOnce();
    const { controller } = mountController(useGroupsController);
    await settle();
    await controller.show("grp_backend");
    await controller.remove();
    expect(accessApi.deleteGroup).toHaveBeenCalledWith("grp_backend");
    expect(controller.open.value).toBeNull();
  });
});
