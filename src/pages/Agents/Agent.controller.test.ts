import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAgentController } from "./Agent.controller";
import * as accessApi from "~/services/access.api";
import * as rolesApi from "~/services/roles.api";
import * as tokensApi from "~/services/personal-tokens.api";
import * as usersApi from "~/services/users.api";
import { ApiError } from "~/services/http.client";
import { formatDate } from "~/utils/format";
import { mountController } from "~/tests/mount-controller";
import {
  accessFixture,
  agentRole,
  lead,
  maya,
  memberRole,
  reviewBot,
  settle,
  signInAs,
  tokenFixture,
  userFixture,
} from "~/tests/people";

const { route, push, replace } = await vi.hoisted(async () => {
  const { reactive } = await import("vue");
  return {
    route: reactive({
      name: "agent" as string,
      params: { agentId: "usr_bot" } as Record<string, string>,
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
vi.mock("~/services/personal-tokens.api");
vi.mock("~/services/roles.api", async (original) => ({
  ...(await original<typeof rolesApi>()),
  fetchRoles: vi.fn(),
}));
vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  fetchUserAccess: vi.fn(),
}));

const botAccess = accessFixture({
  userId: "usr_bot",
  email: "review-bot",
  roleName: "AI agent",
  maxLevel: "read",
});

async function mountAgent() {
  const mounted = mountController(useAgentController);
  await settle();
  return mounted.controller;
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date("2026-10-31T00:00:00Z"));
  route.name = "agent";
  route.params = { agentId: "usr_bot" };
  signInAs(["users.view", "users.manage"]);
  vi.mocked(usersApi.fetchUser).mockResolvedValue(reviewBot);
  vi.mocked(usersApi.fetchUsers).mockResolvedValue([maya, lead, reviewBot]);
  vi.mocked(accessApi.fetchUserAccess).mockResolvedValue(botAccess);
  vi.mocked(tokensApi.listPersonalTokens).mockResolvedValue([tokenFixture()]);
  vi.mocked(rolesApi.fetchRoles).mockResolvedValue([memberRole, agentRole]);
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useAgentController", () => {
  it("loads the agent with its access, tokens, role and owner", async () => {
    const controller = await mountAgent();

    expect(usersApi.fetchUser).toHaveBeenCalledWith("usr_bot");
    expect(accessApi.fetchUserAccess).toHaveBeenCalledWith("usr_bot");
    expect(tokensApi.listPersonalTokens).toHaveBeenCalledWith("usr_bot");
    expect(controller.agent.value).toEqual(reviewBot);
    expect(controller.access.value).toEqual(botAccess);
    expect(controller.role.value?.id).toBe("rol_ai_agent");
    expect(controller.owner.value?.id).toBe("usr_maya");
    expect(controller.people.value.map((person) => person.id)).toEqual([
      "usr_maya",
      "usr_lead",
    ]);
    expect(controller.principal.value).toEqual({
      type: "user",
      id: "usr_bot",
      label: "review-bot",
      maxLevel: "read",
    });
  });

  it("sends a person's account to the user page instead", async () => {
    vi.mocked(usersApi.fetchUser).mockResolvedValue(maya);
    route.params = { agentId: "usr_maya" };
    const controller = await mountAgent();

    expect(replace).toHaveBeenCalledWith({
      name: "user",
      params: { userId: "usr_maya" },
    });
    expect(controller.agent.value).toBeNull();
  });

  it("shows the server's reason when the agent can't load", async () => {
    vi.mocked(usersApi.fetchUser).mockRejectedValue(
      new ApiError(404, { USER_NOT_FOUND: "No such account." }),
    );
    const controller = await mountAgent();

    expect(controller.loadError.value).toBe("No such account.");
    expect(controller.agent.value).toBeNull();
  });

  it("ignores a load that finishes after the page moved to another agent", async () => {
    const otherBot = userFixture({
      ...reviewBot,
      id: "usr_other",
      email: "other-bot",
    });
    let finishFirst!: () => void;
    vi.mocked(usersApi.fetchUser).mockImplementation((id) =>
      id === "usr_bot"
        ? new Promise((resolve) => {
            finishFirst = () => resolve(reviewBot);
          })
        : Promise.resolve(otherBot),
    );
    const controller = await mountAgent();

    route.params = { agentId: "usr_other" };
    await settle();
    finishFirst();
    await settle();

    expect(controller.agent.value?.id).toBe("usr_other");
  });

  it("picks the tab from the route name and falls back to Access", async () => {
    const controller = await mountAgent();
    const seen: Record<string, string> = {};

    for (const name of ["agent", "agent-tokens", "agent-danger", "agents"]) {
      route.name = name;
      seen[name] = controller.activeTab.value;
    }

    expect(seen).toEqual({
      agent: "access",
      "agent-tokens": "tokens",
      "agent-danger": "danger",
      agents: "access",
    });
  });

  it("offers the Danger zone tab to someone who manages users", async () => {
    const controller = await mountAgent();

    expect(controller.tabs.value).toEqual([
      {
        key: "access",
        label: "Access",
        to: { name: "agent", params: { agentId: "usr_bot" } },
      },
      {
        key: "tokens",
        label: "Tokens",
        to: { name: "agent-tokens", params: { agentId: "usr_bot" } },
      },
      {
        key: "danger",
        label: "Danger zone",
        to: { name: "agent-danger", params: { agentId: "usr_bot" } },
      },
    ]);
  });

  it("offers the Danger zone to the owner even without users.manage", async () => {
    signInAs(["tokens.personal"], { adminId: "usr_maya", role: "member" });
    const controller = await mountAgent();

    expect(controller.canManage.value).toBe(true);
    expect(controller.tabs.value.map((tab) => tab.key)).toContain("danger");
  });

  it("hides the Danger zone from someone who only views users", async () => {
    signInAs(["users.view"]);
    const controller = await mountAgent();

    expect(controller.canManage.value).toBe(false);
    expect(controller.tabs.value.map((tab) => tab.key)).toEqual([
      "access",
      "tokens",
    ]);
  });

  it("sums up the owner and how far the agent reaches under its name", async () => {
    const controller = await mountAgent();

    expect(controller.subtitle.value).toBe(
      "Owned by maya@example.com · reaches at most Read",
    );
    expect(controller.accessSummary.value).toBe(
      "review-bot reaches at most Read, and never more than maya@example.com does.",
    );
  });

  it("marks a disabled agent whose owner is no longer listed", async () => {
    vi.mocked(usersApi.fetchUser).mockResolvedValue(
      userFixture({ ...reviewBot, ownerId: "usr_gone", status: "disabled" }),
    );
    const controller = await mountAgent();

    expect(controller.subtitle.value).toBe(
      "Owner unknown · reaches at most Read · disabled",
    );
    expect(controller.accessSummary.value).toBe(
      "review-bot reaches at most Read, and never more than its owner does.",
    );
  });

  it("lists every token with its state and picks the live one as current", async () => {
    const revoked = tokenFixture({
      id: "pat_old",
      name: "old-ci",
      scopeType: "project",
      scopeLabel: "payments",
      revokedAt: "2026-10-10T00:00:00Z",
      expiresAt: null,
      lastUsedAt: "2026-10-30T23:59:50Z",
    });
    vi.mocked(tokensApi.listPersonalTokens).mockResolvedValue([
      tokenFixture(),
      revoked,
    ]);
    const controller = await mountAgent();

    expect(controller.current.value?.id).toBe("pat_1");
    expect(controller.tokenRows.value).toEqual([
      {
        token: tokenFixture(),
        state: "active",
        scope: "Every project it reaches",
        created: formatDate("2026-10-01T00:00:00Z"),
        expires: formatDate("2099-01-01T00:00:00Z"),
        lastUsed: "Never",
      },
      {
        token: revoked,
        state: "revoked",
        scope: "payments",
        created: formatDate("2026-10-01T00:00:00Z"),
        expires: "Never",
        lastUsed: "just now",
      },
    ]);
  });

  it("revokes a token and reloads the list", async () => {
    const controller = await mountAgent();
    vi.mocked(tokensApi.listPersonalTokens).mockResolvedValue([
      tokenFixture({ revokedAt: "2026-10-31T00:00:00Z" }),
    ]);

    await controller.revokeToken(tokenFixture());

    expect(tokensApi.revokePersonalToken).toHaveBeenCalledWith(
      "usr_bot",
      "pat_1",
    );
    expect(controller.current.value).toBeNull();
    expect(controller.tokensError.value).toBeNull();
  });

  it("shows why a revoke failed under the tokens", async () => {
    vi.mocked(tokensApi.revokePersonalToken).mockRejectedValue(
      new ApiError(404, { TOKEN_NOT_FOUND: "That token no longer exists." }),
    );
    const controller = await mountAgent();

    await controller.revokeToken(tokenFixture());

    expect(controller.tokensError.value).toBe("That token no longer exists.");
    expect(tokensApi.listPersonalTokens).toHaveBeenCalledTimes(1);
  });

  it("closes New token, keeps the plaintext and reloads the tokens", async () => {
    const controller = await mountAgent();
    controller.tokenOpen.value = true;
    const fresh = tokenFixture({
      id: "pat_2",
      createdAt: "2026-10-31T00:00:00Z",
    });
    vi.mocked(tokensApi.listPersonalTokens).mockResolvedValue([fresh]);
    const created = { token: fresh, plaintextToken: "dop_pat_x" };

    await controller.tokenCreated(created);

    expect(controller.tokenOpen.value).toBe(false);
    expect(controller.createdToken.value).toEqual(created);
    expect(controller.current.value?.id).toBe("pat_2");
  });

  it("says when the tokens can't reload", async () => {
    const controller = await mountAgent();
    vi.mocked(tokensApi.listPersonalTokens).mockRejectedValue(new Error(""));

    await controller.tokenCreated({
      token: tokenFixture(),
      plaintextToken: "dop_pat_x",
    });

    expect(controller.tokensError.value).toBe("Could not load its tokens.");
  });

  it("refreshes its access after a grant changes", async () => {
    const controller = await mountAgent();
    vi.mocked(accessApi.fetchUserAccess).mockResolvedValue({
      ...botAccess,
      grants: [],
    });

    await controller.reloadAccess();

    expect(controller.access.value?.grants).toEqual([]);
  });

  it("reloads the agent after its owner is transferred", async () => {
    const controller = await mountAgent();
    controller.transferOpen.value = true;
    vi.mocked(usersApi.fetchUser).mockResolvedValue(
      userFixture({ ...reviewBot, ownerId: "usr_lead" }),
    );

    await controller.transferred();

    expect(controller.transferOpen.value).toBe(false);
    expect(controller.owner.value?.id).toBe("usr_lead");
  });

  it("goes back to the agents list after deleting the agent", async () => {
    const controller = await mountAgent();
    controller.actions.ask("delete", reviewBot);

    await controller.actions.confirm();

    expect(usersApi.deleteUser).toHaveBeenCalledWith("usr_bot");
    expect(push).toHaveBeenCalledWith({ name: "agents" });
    expect(usersApi.fetchUser).toHaveBeenCalledTimes(1);
  });

  it("reloads the agent after disabling it", async () => {
    const controller = await mountAgent();
    vi.mocked(usersApi.fetchUser).mockResolvedValue(
      userFixture({ ...reviewBot, status: "disabled" }),
    );
    controller.actions.ask("disable", reviewBot);

    await controller.actions.confirm();

    expect(usersApi.disableUser).toHaveBeenCalledWith("usr_bot");
    expect(push).not.toHaveBeenCalled();
    expect(controller.agent.value?.status).toBe("disabled");
  });

  it("links to the agent's audit activity", async () => {
    const controller = await mountAgent();

    expect(controller.activity.value).toEqual({
      name: "audit",
      query: { actor: "usr_bot" },
    });
  });
});
