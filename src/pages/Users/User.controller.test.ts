import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick } from "vue";
import { useUserController } from "./User.controller";
import * as usersApi from "~/services/users.api";
import * as rolesApi from "~/services/roles.api";
import * as accessApi from "~/services/access.api";
import * as authApi from "~/services/auth.api";
import * as tokensApi from "~/services/personal-tokens.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import {
  accessFixture,
  groupDetailFixture,
  groupSummaryFixture,
  lead,
  maya,
  memberRole,
  reviewBot,
  root,
  settle,
  signInAs,
  tokenFixture,
  userFixture,
} from "~/tests/people";

const { route, push, replace } = await vi.hoisted(async () => {
  const { reactive } = await import("vue");
  return {
    route: reactive({
      name: "user" as string,
      params: { userId: "usr_maya" } as Record<string, string>,
      query: {} as Record<string, string>,
    }),
    push: vi.fn(),
    replace: vi.fn(),
  };
});

vi.mock("vue-router", () => ({
  useRoute: () => route,
  useRouter: () => ({ push, replace }),
}));
vi.mock("~/services/users.api");
vi.mock("~/services/roles.api");
vi.mock("~/services/auth.api");
vi.mock("~/services/personal-tokens.api");
vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  fetchUserAccess: vi.fn(),
  fetchGroups: vi.fn(),
  addGroupMember: vi.fn(),
  removeGroupMember: vi.fn(),
}));

const backend = groupSummaryFixture();
const frontend = groupSummaryFixture({ id: "grp_frontend", name: "Frontend" });
const oncall = groupSummaryFixture({ id: "grp_oncall", name: "On-call" });

const MANAGER: accessApi.Capability[] = [
  "users.view",
  "users.manage",
  "groups.manage",
];

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  route.name = "user";
  route.params = { userId: "usr_maya" };
  vi.mocked(usersApi.fetchUser).mockImplementation(async (id) =>
    id === "usr_maya" ? maya : userFixture({ id, email: `${id}@example.com` }),
  );
  vi.mocked(accessApi.fetchUserAccess).mockResolvedValue(accessFixture());
  vi.mocked(accessApi.fetchGroups).mockResolvedValue([backend, frontend]);
  vi.mocked(accessApi.addGroupMember).mockResolvedValue(groupDetailFixture());
  vi.mocked(accessApi.removeGroupMember).mockResolvedValue(
    groupDetailFixture(),
  );
  vi.mocked(rolesApi.fetchRoles).mockResolvedValue([memberRole]);
  vi.mocked(authApi.fetchAuthMethods).mockResolvedValue({
    password: true,
    passwordForm: true,
    sso: null,
    cliBrowser: true,
  });
  vi.mocked(tokensApi.listPersonalTokens).mockResolvedValue([]);
  vi.mocked(tokensApi.revokePersonalToken).mockResolvedValue(
    tokenFixture({ revokedAt: "2026-10-31T00:00:00Z" }),
  );
  vi.mocked(usersApi.disableUser).mockResolvedValue(maya);
  vi.mocked(usersApi.deleteUser).mockResolvedValue();
});

async function open(
  capabilities: accessApi.Capability[] = MANAGER,
  adminId = "usr_lead",
) {
  signInAs(capabilities, { adminId });
  const { controller } = mountController(useUserController);
  await settle();
  return controller;
}

describe("useUserController", () => {
  describe("loading", () => {
    it("loads the account, its access, roles and sign-in methods", async () => {
      vi.mocked(authApi.fetchAuthMethods).mockResolvedValue({
        password: true,
        passwordForm: true,
        sso: { name: "Okta" },
        cliBrowser: true,
      });
      const c = await open();

      expect(usersApi.fetchUser).toHaveBeenCalledWith("usr_maya");
      expect(accessApi.fetchUserAccess).toHaveBeenCalledWith("usr_maya");
      expect(c.user.value).toStrictEqual(maya);
      expect(c.role.value).toStrictEqual(memberRole);
      expect(c.ssoEnabled.value).toBe(true);
      expect(c.created.value).not.toBe("");
    });

    it("sends an AI agent to its own page", async () => {
      vi.mocked(usersApi.fetchUser).mockResolvedValue(reviewBot);
      const c = await open();

      expect(replace).toHaveBeenCalledWith({
        name: "agent",
        params: { agentId: "usr_bot" },
      });
      expect(c.user.value).toBeNull();
    });

    it("sends an AI agent's tokens link to the agent's Tokens tab", async () => {
      route.name = "user-tokens";
      vi.mocked(usersApi.fetchUser).mockResolvedValue(reviewBot);
      await open();

      expect(replace).toHaveBeenCalledWith({
        name: "agent-tokens",
        params: { agentId: "usr_bot" },
      });
      expect(tokensApi.listPersonalTokens).not.toHaveBeenCalled();
    });

    it("shows the server's reason when the account can't load", async () => {
      vi.mocked(usersApi.fetchUser).mockRejectedValue(
        new ApiError(404, { USER_NOT_FOUND: "No account with that id." }),
      );
      const c = await open();
      expect(c.loadError.value).toBe("No account with that id.");
      expect(c.user.value).toBeNull();
    });

    it("loads the next account when the address changes", async () => {
      const c = await open();
      expect(c.user.value?.id).toBe("usr_maya");

      route.params = { userId: "usr_sam" };
      await nextTick();
      expect(c.user.value).toBeNull();
      await settle();

      expect(usersApi.fetchUser).toHaveBeenLastCalledWith("usr_sam");
      expect(c.user.value?.id).toBe("usr_sam");
    });

    it("ignores a slow answer for the account you already left", async () => {
      let answerMaya!: (user: usersApi.User) => void;
      vi.mocked(usersApi.fetchUser).mockImplementation((id) =>
        id === "usr_maya"
          ? new Promise((resolve) => (answerMaya = resolve))
          : Promise.resolve(userFixture({ id, email: "sam@example.com" })),
      );
      const c = await open();

      route.params = { userId: "usr_sam" };
      await settle();
      answerMaya(maya);
      await settle();

      expect(c.user.value?.id).toBe("usr_sam");
    });
  });

  describe("header", () => {
    it("sums up role, sign-in and that they never signed in", async () => {
      const c = await open();
      expect(c.subtitle.value).toBe("Member · password · never signed in");
    });

    it("says when they last signed in", async () => {
      vi.mocked(usersApi.fetchUser).mockResolvedValue(
        userFixture({
          lastSignInAt: new Date(Date.now() - 3 * 3600 * 1000).toISOString(),
        }),
      );
      const c = await open();
      expect(c.subtitle.value).toBe("Member · password · last sign-in 3h ago");
    });

    it("names single sign-on and accounts not set up yet", async () => {
      vi.mocked(usersApi.fetchUser).mockResolvedValue(
        userFixture({
          hasPassword: false,
          ssoLinked: true,
          roleName: undefined,
        }),
      );
      const sso = await open();
      expect(sso.subtitle.value).toBe("member · sso · never signed in");

      vi.mocked(usersApi.fetchUser).mockResolvedValue(
        userFixture({ hasPassword: false }),
      );
      await sso.load();
      expect(sso.subtitle.value).toBe("Member · not set up · never signed in");
    });
  });

  describe("tabs", () => {
    it("picks the tab from the route", async () => {
      const c = await open();
      const expected = {
        user: "overview",
        "user-access": "access",
        "user-groups": "groups",
        "user-tokens": "tokens",
        "user-danger": "danger",
        "something-else": "overview",
      } as const;
      for (const [name, tab] of Object.entries(expected)) {
        route.name = name;
        expect(c.activeTab.value).toBe(tab);
      }
    });

    it("links every tab to this account", async () => {
      const c = await open();
      expect(c.tabs.value.map((tab) => [tab.label, tab.to])).toEqual([
        ["Overview", { name: "user", params: { userId: "usr_maya" } }],
        ["Access", { name: "user-access", params: { userId: "usr_maya" } }],
        ["Groups", { name: "user-groups", params: { userId: "usr_maya" } }],
        ["Tokens", { name: "user-tokens", params: { userId: "usr_maya" } }],
        [
          "Danger zone",
          { name: "user-danger", params: { userId: "usr_maya" } },
        ],
      ]);
    });

    it("hides the Danger zone from someone who can't manage users", async () => {
      const c = await open(["users.view"]);
      expect(c.canManage.value).toBe(false);
      expect(c.tabs.value.map((tab) => tab.key)).not.toContain("danger");
    });

    it("hides the Danger zone on root's page", async () => {
      route.params = { userId: "usr_root" };
      vi.mocked(usersApi.fetchUser).mockResolvedValue(root);
      const c = await open();
      expect(c.isRoot.value).toBe(true);
      expect(c.canManage.value).toBe(false);
      expect(c.tabs.value.map((tab) => tab.key)).not.toContain("danger");
    });

    it("hides the Danger zone on your own page", async () => {
      route.params = { userId: "usr_lead" };
      vi.mocked(usersApi.fetchUser).mockResolvedValue(lead);
      const c = await open();
      expect(c.isYou.value).toBe(true);
      expect(c.canManage.value).toBe(false);
      expect(c.tabs.value.map((tab) => tab.key)).not.toContain("danger");
    });
  });

  describe("access", () => {
    it("says access comes only from direct grants when they're in no group", async () => {
      const c = await open();
      expect(c.accessSummary.value).toBe(
        "Their role allows up to Project manager. Access comes from direct grants.",
      );
    });

    it("names the one group access also comes from", async () => {
      vi.mocked(accessApi.fetchUserAccess).mockResolvedValue(
        accessFixture({
          maxLevel: "read",
          groups: [{ id: "grp_backend", name: "Backend" }],
        }),
      );
      const c = await open();
      expect(c.accessSummary.value).toBe(
        "Their role allows up to Read. Access comes from direct grants and from the Backend group.",
      );
    });

    it("lists several groups in a sentence", async () => {
      vi.mocked(accessApi.fetchUserAccess).mockResolvedValue(
        accessFixture({
          groups: [backend, frontend, oncall].map(({ id, name }) => ({
            id,
            name,
          })),
        }),
      );
      const c = await open();
      expect(c.accessSummary.value).toContain(
        "from the Backend, Frontend and On-call groups.",
      );
    });

    it("describes the account to the grants panel", async () => {
      const c = await open();
      expect(c.principal.value).toEqual({
        type: "user",
        id: "usr_maya",
        label: "maya@example.com",
        maxLevel: "project_manager",
      });
    });

    it("says so when their access can't reload", async () => {
      const c = await open();
      vi.mocked(accessApi.fetchUserAccess).mockRejectedValue(new Error(""));
      await c.reloadAccess();
      expect(c.loadError.value).toBe("Could not reload their access.");
    });
  });

  describe("groups", () => {
    it("doesn't fetch groups until the Groups tab opens", async () => {
      const c = await open();
      expect(accessApi.fetchGroups).not.toHaveBeenCalled();

      route.name = "user-groups";
      await settle();
      expect(accessApi.fetchGroups).toHaveBeenCalledTimes(1);

      route.name = "user";
      await settle();
      route.name = "user-groups";
      await settle();
      expect(accessApi.fetchGroups).toHaveBeenCalledTimes(1);
      expect(c.canManageGroups.value).toBe(true);
    });

    it("offers only the groups they aren't in yet", async () => {
      route.name = "user-groups";
      vi.mocked(accessApi.fetchUserAccess).mockResolvedValue(
        accessFixture({ groups: [{ id: "grp_backend", name: "Backend" }] }),
      );
      const c = await open();
      expect(c.joinable.value.map((group) => group.id)).toEqual([
        "grp_frontend",
      ]);
    });

    it("doesn't offer groups without groups.manage", async () => {
      route.name = "user-groups";
      const c = await open(["users.view", "users.manage"]);
      expect(c.canManageGroups.value).toBe(false);
      expect(accessApi.fetchGroups).not.toHaveBeenCalled();
      expect(c.joinable.value).toEqual([]);
    });

    it("never puts root in a group", async () => {
      route.name = "user-groups";
      route.params = { userId: "usr_root" };
      vi.mocked(usersApi.fetchUser).mockResolvedValue(root);
      const c = await open();
      expect(c.canManageGroups.value).toBe(false);
      expect(accessApi.fetchGroups).not.toHaveBeenCalled();
    });

    it("offers nothing when the group list fails", async () => {
      route.name = "user-groups";
      vi.mocked(accessApi.fetchGroups).mockRejectedValue(new Error("down"));
      const c = await open();
      expect(c.joinable.value).toEqual([]);
    });

    it("adds them to a group and shows their new access", async () => {
      const c = await open();
      vi.mocked(accessApi.fetchUserAccess).mockResolvedValue(
        accessFixture({ groups: [{ id: "grp_frontend", name: "Frontend" }] }),
      );

      await c.joinGroup("grp_frontend");

      expect(accessApi.addGroupMember).toHaveBeenCalledWith(
        "grp_frontend",
        "usr_maya",
      );
      expect(c.access.value?.groups).toEqual([
        { id: "grp_frontend", name: "Frontend" },
      ]);
      expect(c.groupsError.value).toBeNull();
    });

    it("takes them out of a group", async () => {
      const c = await open();
      await c.leaveGroup("grp_backend");
      expect(accessApi.removeGroupMember).toHaveBeenCalledWith(
        "grp_backend",
        "usr_maya",
      );
      expect(accessApi.fetchUserAccess).toHaveBeenCalledTimes(2);
    });

    it("shows why a group change failed", async () => {
      const c = await open();
      vi.mocked(accessApi.addGroupMember).mockRejectedValue(
        new ApiError(409, { ALREADY_MEMBER: "They're already in that group." }),
      );
      await c.joinGroup("grp_backend");
      expect(c.groupsError.value).toBe("They're already in that group.");

      vi.mocked(accessApi.removeGroupMember).mockRejectedValue("boom");
      await c.leaveGroup("grp_backend");
      expect(c.groupsError.value).toBe(
        "Could not take them out of that group.",
      );
    });
  });

  describe("tokens", () => {
    it("lists their tokens only once the Tokens tab opens", async () => {
      await open();
      expect(tokensApi.listPersonalTokens).not.toHaveBeenCalled();

      route.name = "user-tokens";
      await settle();
      expect(tokensApi.listPersonalTokens).toHaveBeenCalledWith("usr_maya");
    });

    it("describes each token's scope, state, expiry and last use", async () => {
      route.name = "user-tokens";
      vi.mocked(tokensApi.listPersonalTokens).mockResolvedValue([
        tokenFixture({ id: "pat_all", userId: "usr_maya", expiresAt: null }),
        tokenFixture({
          id: "pat_pay",
          userId: "usr_maya",
          scopeType: "project",
          scopeLabel: "payments",
          lastUsedAt: new Date(Date.now() - 5 * 60 * 1000).toISOString(),
        }),
        tokenFixture({
          id: "pat_old",
          userId: "usr_maya",
          revokedAt: "2026-10-20T00:00:00Z",
        }),
        tokenFixture({
          id: "pat_late",
          userId: "usr_maya",
          expiresAt: "2026-01-01T00:00:00Z",
        }),
      ]);
      const c = await open();

      expect(
        c.tokenRows.value.map(({ token, scope, state, expires, lastUsed }) => ({
          id: token.id,
          scope,
          state,
          expires,
          lastUsed,
        })),
      ).toEqual([
        {
          id: "pat_all",
          scope: "All projects they reach",
          state: "active",
          expires: "Never",
          lastUsed: "Never",
        },
        {
          id: "pat_pay",
          scope: "payments",
          state: "active",
          expires: expect.not.stringMatching(/^Never$/),
          lastUsed: "5m ago",
        },
        expect.objectContaining({ id: "pat_old", state: "revoked" }),
        expect.objectContaining({ id: "pat_late", state: "expired" }),
      ]);
    });

    it("lets only holders of tokens.revoke_others revoke their tokens", async () => {
      const without = await open();
      expect(without.canRevokeTokens.value).toBe(false);

      setActivePinia(createPinia());
      const holder = await open([...MANAGER, "tokens.revoke_others"]);
      expect(holder.canRevokeTokens.value).toBe(true);
    });

    it("revokes a token and refreshes the list", async () => {
      route.name = "user-tokens";
      const c = await open();
      const token = tokenFixture({ id: "pat_ci", userId: "usr_maya" });

      await c.revokeToken(token);

      expect(tokensApi.revokePersonalToken).toHaveBeenCalledWith(
        "usr_maya",
        "pat_ci",
      );
      expect(tokensApi.listPersonalTokens).toHaveBeenCalledTimes(2);
      expect(c.tokensError.value).toBeNull();
    });

    it("shows why revoking failed", async () => {
      route.name = "user-tokens";
      vi.mocked(tokensApi.revokePersonalToken).mockRejectedValue(
        new ApiError(409, { TOKEN_REVOKED: "That token is already revoked." }),
      );
      const c = await open();
      await c.revokeToken(tokenFixture());
      expect(c.tokensError.value).toBe("That token is already revoked.");
    });

    it("shows an empty list with a reason when tokens can't load", async () => {
      route.name = "user-tokens";
      vi.mocked(tokensApi.listPersonalTokens).mockRejectedValue("boom");
      const c = await open();
      expect(c.tokens.value).toEqual([]);
      expect(c.tokensError.value).toBe("Could not load their tokens.");
    });
  });

  describe("changing the account", () => {
    it("reloads the page after an edit", async () => {
      const c = await open();
      c.editOpen.value = true;
      await c.edited();
      expect(c.editOpen.value).toBe(false);
      expect(usersApi.fetchUser).toHaveBeenCalledTimes(2);
    });

    it("stays on the page and reloads it after disabling them", async () => {
      const c = await open();
      c.actions.ask("disable", maya);
      await c.actions.confirm();
      expect(usersApi.disableUser).toHaveBeenCalledWith("usr_maya");
      expect(usersApi.fetchUser).toHaveBeenCalledTimes(2);
      expect(push).not.toHaveBeenCalled();
    });

    it("goes back to the users list after deleting them", async () => {
      const c = await open();
      c.actions.ask("delete", maya);
      await c.actions.confirm();
      expect(usersApi.deleteUser).toHaveBeenCalledWith("usr_maya");
      expect(push).toHaveBeenCalledWith({ name: "users" });
      expect(usersApi.fetchUser).toHaveBeenCalledTimes(1);
    });
  });
});
