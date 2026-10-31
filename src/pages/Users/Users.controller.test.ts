import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { STATUS_FILTERS, useUsersController } from "./Users.controller";
import * as usersApi from "~/services/users.api";
import * as rolesApi from "~/services/roles.api";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import {
  adminRole,
  lead,
  maya,
  memberRole,
  reader,
  reviewBot,
  root,
  settle,
  signInAs,
  userFixture,
  viewerRole,
} from "~/tests/people";

const { push } = vi.hoisted(() => ({ push: vi.fn() }));

vi.mock("vue-router", () => ({
  useRouter: () => ({ push }),
}));
vi.mock("~/services/users.api");
vi.mock("~/services/roles.api");
vi.mock("~/services/auth.api");

const ssoUser = userFixture({
  id: "usr_sso",
  email: "Sam@Example.com",
  hasPassword: false,
  ssoLinked: true,
});
const disabled = userFixture({
  id: "usr_gone",
  email: "gone@example.com",
  status: "disabled",
});

const methods = (sso: { name: string } | null): authApi.AuthMethods => ({
  password: true,
  passwordForm: true,
  sso,
  cliBrowser: true,
});

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(usersApi.fetchUsers).mockResolvedValue([
    root,
    lead,
    maya,
    reader,
    reviewBot,
  ]);
  vi.mocked(rolesApi.fetchRoles).mockResolvedValue([
    adminRole,
    memberRole,
    viewerRole,
  ]);
  vi.mocked(authApi.fetchAuthMethods).mockResolvedValue(methods(null));
  vi.mocked(usersApi.disableUser).mockResolvedValue(disabled);
});

async function mountAs(
  capabilities: Parameters<typeof signInAs>[0] = ["users.view", "users.manage"],
) {
  signInAs(capabilities);
  const { controller } = mountController(useUsersController);
  await settle();
  return controller;
}

const ids = (rows: { user: usersApi.User }[]) => rows.map((row) => row.user.id);
const rowFor = (c: Awaited<ReturnType<typeof mountAs>>, id: string) =>
  c.rows.value.find((row) => row.user.id === id)!;

describe("useUsersController", () => {
  describe("the list", () => {
    it("lists the people who sign in and leaves AI agents to their own page", async () => {
      const c = await mountAs();
      expect(ids(c.rows.value)).toEqual([
        "usr_root",
        "usr_lead",
        "usr_maya",
        "usr_reader",
      ]);
    });

    it("marks a custom role apart from the ones Dopbase ships with", async () => {
      const c = await mountAs();
      expect(rowFor(c, "usr_reader")).toMatchObject({
        roleName: "Viewer",
        customRole: true,
      });
      expect(rowFor(c, "usr_maya")).toMatchObject({
        roleName: "Member",
        customRole: false,
      });
    });

    it("falls back to the role row's name, then the fixed role, when the account has no role name", async () => {
      vi.mocked(usersApi.fetchUsers).mockResolvedValue([
        userFixture({ id: "usr_a", roleName: undefined }),
        userFixture({ id: "usr_b", roleId: "rol_gone", roleName: undefined }),
      ]);
      const c = await mountAs();
      expect(rowFor(c, "usr_a").roleName).toBe("Member");
      expect(rowFor(c, "usr_b")).toMatchObject({
        roleName: "member",
        customRole: false,
      });
    });

    it("tags root and your own row", async () => {
      const c = await mountAs();
      expect(rowFor(c, "usr_root")).toMatchObject({
        isRoot: true,
        isYou: false,
      });
      expect(rowFor(c, "usr_lead")).toMatchObject({
        isRoot: false,
        isYou: true,
      });
      expect(rowFor(c, "usr_maya")).toMatchObject({
        isRoot: false,
        isYou: false,
      });
    });

    it("shows how each account signs in and when it last did", async () => {
      vi.mocked(usersApi.fetchUsers).mockResolvedValue([
        userFixture({
          lastSignInAt: new Date(Date.now() - 2 * 3600 * 1000).toISOString(),
        }),
        ssoUser,
      ]);
      const c = await mountAs();
      expect(rowFor(c, "usr_maya")).toMatchObject({
        signIn: "password",
        lastSignIn: "2h ago",
      });
      expect(rowFor(c, "usr_sso")).toMatchObject({
        signIn: "sso",
        lastSignIn: "Never",
      });
    });

    it("knows single sign-on is on when the server offers it", async () => {
      vi.mocked(authApi.fetchAuthMethods).mockResolvedValue(
        methods({ name: "Okta" }),
      );
      const c = await mountAs();
      expect(c.ssoEnabled.value).toBe(true);
    });

    it("still lists people when roles and sign-in methods can't load", async () => {
      vi.mocked(rolesApi.fetchRoles).mockRejectedValue(new Error("down"));
      vi.mocked(authApi.fetchAuthMethods).mockRejectedValue(new Error("down"));
      const c = await mountAs();
      expect(c.loadError.value).toBeNull();
      expect(c.roles.value).toEqual([]);
      expect(c.ssoEnabled.value).toBe(false);
      expect(rowFor(c, "usr_reader")).toMatchObject({
        roleName: "Viewer",
        customRole: false,
      });
    });

    it("says so when the list can't load", async () => {
      vi.mocked(usersApi.fetchUsers).mockRejectedValue(
        new ApiError(500, { INTERNAL: "Boom." }),
      );
      const c = await mountAs();
      expect(c.users.value).toEqual([]);
      expect(c.loadError.value).toBe("Could not load users.");
    });

    it("keeps the list on screen when a refresh fails", async () => {
      const c = await mountAs();
      vi.mocked(usersApi.fetchUsers).mockRejectedValue(new Error("down"));
      await c.load();
      expect(c.loadError.value).toBe("Could not load users.");
      expect(ids(c.rows.value)).toContain("usr_maya");
    });
  });

  describe("row actions", () => {
    it("gives a manager every action on an ordinary account", async () => {
      const c = await mountAs();
      expect(rowFor(c, "usr_maya")).toMatchObject({
        actions: ["disable", "delete"],
        canEdit: true,
      });
    });

    it("never lets you change root or your own account", async () => {
      const c = await mountAs();
      expect(rowFor(c, "usr_root")).toMatchObject({
        actions: [],
        canEdit: false,
      });
      expect(rowFor(c, "usr_lead")).toMatchObject({
        actions: [],
        canEdit: false,
      });
    });

    it("offers unlinking only to accounts linked to single sign-on", async () => {
      vi.mocked(usersApi.fetchUsers).mockResolvedValue([maya, ssoUser]);
      const c = await mountAs();
      expect(rowFor(c, "usr_sso").actions).toEqual([
        "unlink",
        "disable",
        "delete",
      ]);
      expect(rowFor(c, "usr_maya").actions).not.toContain("unlink");
    });

    it("offers to turn a disabled account back on instead of disabling it", async () => {
      vi.mocked(usersApi.fetchUsers).mockResolvedValue([disabled]);
      const c = await mountAs();
      expect(rowFor(c, "usr_gone").actions).toEqual(["enable", "delete"]);
    });

    it("gives no actions to someone who can only view users", async () => {
      const c = await mountAs(["users.view"]);
      expect(c.canManage.value).toBe(false);
      expect(c.rows.value.every((row) => row.actions.length === 0)).toBe(true);
      expect(c.rows.value.every((row) => !row.canEdit)).toBe(true);
    });

    it("refreshes the list after disabling someone from it", async () => {
      const c = await mountAs();
      expect(usersApi.fetchUsers).toHaveBeenCalledTimes(1);

      c.actions.ask("disable", maya);
      await c.actions.confirm();

      expect(usersApi.disableUser).toHaveBeenCalledWith("usr_maya");
      expect(usersApi.fetchUsers).toHaveBeenCalledTimes(2);
    });
  });

  describe("filters", () => {
    beforeEach(() => {
      vi.mocked(usersApi.fetchUsers).mockResolvedValue([
        maya,
        reader,
        ssoUser,
        disabled,
      ]);
    });

    it("offers the three status filters", () => {
      expect(STATUS_FILTERS.map((option) => option.value)).toEqual([
        "active",
        "disabled",
        "sso",
      ]);
    });

    it("searches emails ignoring case and surrounding spaces", async () => {
      const c = await mountAs();
      c.filter.value = "  SAM@ ";
      expect(ids(c.visibleRows.value)).toEqual(["usr_sso"]);
    });

    it("offers each role once, sorted, as a role filter", async () => {
      const c = await mountAs();
      expect(c.roleFilters.value).toEqual([
        { value: "Member", label: "Member" },
        { value: "Viewer", label: "Viewer" },
      ]);
    });

    it("narrows the list to one role", async () => {
      const c = await mountAs();
      c.roleFilter.value = "Viewer";
      expect(ids(c.visibleRows.value)).toEqual(["usr_reader"]);
    });

    it("narrows the list by status", async () => {
      const c = await mountAs();
      c.statusFilter.value = "disabled";
      expect(ids(c.visibleRows.value)).toEqual(["usr_gone"]);
      c.statusFilter.value = "active";
      expect(ids(c.visibleRows.value)).toEqual([
        "usr_maya",
        "usr_reader",
        "usr_sso",
      ]);
      c.statusFilter.value = "sso";
      expect(ids(c.visibleRows.value)).toEqual(["usr_sso"]);
    });

    it("combines search, role and status", async () => {
      const c = await mountAs();
      c.filter.value = "example.com";
      c.roleFilter.value = "Member";
      c.statusFilter.value = "active";
      expect(ids(c.visibleRows.value)).toEqual(["usr_maya", "usr_sso"]);
    });
  });

  describe("adding and editing", () => {
    it("opens an empty dialog to add someone", async () => {
      const c = await mountAs();
      c.openEdit(maya);
      c.dialogOpen.value = false;

      c.openAdd();

      expect(c.dialogOpen.value).toBe(true);
      expect(c.editing.value).toBeNull();
    });

    it("sends a new account straight to its Access tab", async () => {
      const c = await mountAs();
      const sam = userFixture({ id: "usr_sam", email: "sam@example.com" });

      c.openAdd();
      await c.saved(sam);

      expect(c.dialogOpen.value).toBe(false);
      expect(push).toHaveBeenCalledWith({
        name: "user-access",
        params: { userId: "usr_sam" },
      });
      expect(usersApi.fetchUsers).toHaveBeenCalledTimes(1);
    });

    it("stays on the list and refreshes it after an edit", async () => {
      const c = await mountAs();

      c.openEdit(maya);
      expect(c.editing.value).toStrictEqual(maya);
      expect(c.dialogOpen.value).toBe(true);
      await c.saved(maya);

      expect(c.dialogOpen.value).toBe(false);
      expect(push).not.toHaveBeenCalled();
      expect(usersApi.fetchUsers).toHaveBeenCalledTimes(2);
    });
  });
});
