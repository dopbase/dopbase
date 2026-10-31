import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import {
  agentTokenUsage,
  liveToken,
  useAgentsController,
} from "./Agents.controller";
import type { PersonalToken } from "~/services/personal-tokens.api";
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
  grantFixture,
  lead,
  maya,
  memberRole,
  reviewBot,
  roleFixture,
  settle,
  signInAs,
  tokenFixture,
  userFixture,
} from "~/tests/people";

const { push } = vi.hoisted(() => ({ push: vi.fn() }));

vi.mock("vue-router", () => ({
  useRouter: () => ({ push }),
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

const NOW = new Date("2026-10-31T00:00:00Z");

/** Owned by lead, so maya does not own it. */
const deployBot = userFixture({
  ...reviewBot,
  id: "usr_deploy",
  email: "deploy-bot",
  ownerId: "usr_lead",
});

function tokensFor(lists: Record<string, tokensApi.PersonalToken[] | Error>) {
  vi.mocked(tokensApi.listPersonalTokens).mockImplementation(async (userId) => {
    const list = lists[userId ?? "me"] ?? [];
    if (list instanceof Error) throw list;
    return list;
  });
}

async function mountAgents() {
  const mounted = mountController(useAgentsController);
  await settle();
  return mounted.controller;
}

function rowOf(
  controller: ReturnType<typeof useAgentsController>,
  agentId: string,
) {
  const row = controller.rows.value.find((item) => item.agent.id === agentId);
  if (!row) throw new Error(`No row for ${agentId}`);
  return row;
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(NOW);
  signInAs(["users.view", "users.manage"]);
  vi.mocked(usersApi.fetchUsers).mockResolvedValue([
    maya,
    lead,
    reviewBot,
    deployBot,
  ]);
  vi.mocked(rolesApi.fetchRoles).mockResolvedValue([memberRole, agentRole]);
  tokensFor({ usr_bot: [tokenFixture()], usr_deploy: [] });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("liveToken", () => {
  it("picks the newest active token and skips newer revoked or expired ones", () => {
    const older = tokenFixture({
      id: "pat_old",
      createdAt: "2026-09-01T00:00:00Z",
    });
    const newer = tokenFixture({
      id: "pat_new",
      createdAt: "2026-10-10T00:00:00Z",
    });
    const revoked = tokenFixture({
      id: "pat_revoked",
      createdAt: "2026-10-20T00:00:00Z",
      revokedAt: "2026-10-21T00:00:00Z",
    });
    const expired = tokenFixture({
      id: "pat_expired",
      createdAt: "2026-10-25T00:00:00Z",
      expiresAt: "2026-10-30T00:00:00Z",
    });

    expect(liveToken([older, revoked, newer, expired])?.id).toBe("pat_new");
  });

  it("returns null when every token is revoked or expired", () => {
    expect(
      liveToken([
        tokenFixture({ revokedAt: "2026-10-21T00:00:00Z" }),
        tokenFixture({ id: "pat_2", expiresAt: "2026-10-01T00:00:00Z" }),
      ]),
    ).toBeNull();
  });
});

describe("useAgentsController", () => {
  it("lists only AI agents and keeps people apart as possible owners", async () => {
    const controller = await mountAgents();

    expect(controller.agents.value?.map((agent) => agent.id)).toEqual([
      "usr_bot",
      "usr_deploy",
    ]);
    expect(controller.people.value.map((person) => person.id)).toEqual([
      "usr_maya",
      "usr_lead",
    ]);
    expect(tokensApi.listPersonalTokens).toHaveBeenCalledWith("usr_bot");
    expect(tokensApi.listPersonalTokens).toHaveBeenCalledWith("usr_deploy");
  });

  it("says it could not load the agents when the users list fails", async () => {
    vi.mocked(usersApi.fetchUsers).mockRejectedValue(
      new ApiError(500, { INTERNAL: "The server broke." }),
    );
    const controller = await mountAgents();

    expect(controller.loadError.value).toBe("Could not load AI agents.");
    expect(controller.agents.value).toEqual([]);
    expect(controller.rows.value).toEqual([]);
  });

  it("names the owner by email, or says No owner or Unknown", async () => {
    const orphan = userFixture({
      ...reviewBot,
      id: "usr_orphan",
      ownerId: null,
    });
    const stray = userFixture({
      ...reviewBot,
      id: "usr_stray",
      ownerId: "usr_gone",
    });
    vi.mocked(usersApi.fetchUsers).mockResolvedValue([
      maya,
      reviewBot,
      orphan,
      stray,
    ]);
    const controller = await mountAgents();

    expect(rowOf(controller, "usr_bot").owner).toBe("maya@example.com");
    expect(rowOf(controller, "usr_orphan").owner).toBe("No owner");
    expect(rowOf(controller, "usr_stray").owner).toBe("Unknown");
  });

  it("shows how far an agent can reach from its role's highest level", async () => {
    const controller = await mountAgents();

    expect(rowOf(controller, "usr_bot").reach).toEqual({
      label: "Read",
      filled: 2,
      strong: false,
    });
  });

  it("still lists agents without a reach when the roles list fails", async () => {
    vi.mocked(rolesApi.fetchRoles).mockRejectedValue(new Error("down"));
    const controller = await mountAgents();

    expect(controller.loadError.value).toBeNull();
    expect(rowOf(controller, "usr_bot").reach).toBeNull();
    expect(controller.agentRole.value).toBeNull();
  });

  it("marks a token with months left as active", async () => {
    tokensFor({
      usr_bot: [tokenFixture({ lastUsedAt: "2026-10-30T23:59:50Z" })],
    });
    const controller = await mountAgents();
    const row = rowOf(controller, "usr_bot");

    expect(row.token?.id).toBe("pat_1");
    expect(row.status).toBe("active");
    expect(row.statusLabel).toBe("active");
    expect(row.expires).toBe(formatDate("2099-01-01T00:00:00Z"));
    expect(row.lastUsed).toBe("just now");
  });

  it("flags a token expiring in 3 days", async () => {
    tokensFor({
      usr_bot: [tokenFixture({ expiresAt: "2026-11-03T00:00:00Z" })],
    });
    const controller = await mountAgents();
    const row = rowOf(controller, "usr_bot");

    expect(row.status).toBe("expiring");
    expect(row.statusLabel).toBe("expires in 3 days");
    expect(row.lastUsed).toBe("Never");
  });

  it("says expires in 1 day when the token ends within hours", async () => {
    tokensFor({
      usr_bot: [tokenFixture({ expiresAt: "2026-10-31T06:00:00Z" })],
    });
    const controller = await mountAgents();

    expect(rowOf(controller, "usr_bot").statusLabel).toBe("expires in 1 day");
  });

  it("shows no token when an agent only has revoked or expired ones", async () => {
    tokensFor({
      usr_bot: [
        tokenFixture({ revokedAt: "2026-10-20T00:00:00Z" }),
        tokenFixture({ id: "pat_2", expiresAt: "2026-10-01T00:00:00Z" }),
      ],
    });
    const controller = await mountAgents();
    const row = rowOf(controller, "usr_bot");

    expect(row.token).toBeNull();
    expect(row.status).toBe("none");
    expect(row.statusLabel).toBe("no token");
    expect(row.expires).toBe("—");
  });

  it("shows the token as unknown when that agent's tokens fail to load", async () => {
    tokensFor({
      usr_bot: new Error("down"),
      usr_deploy: [tokenFixture({ id: "pat_9" })],
    });
    const controller = await mountAgents();

    expect(rowOf(controller, "usr_bot").status).toBe("unknown");
    expect(rowOf(controller, "usr_bot").statusLabel).toBe("—");
    expect(rowOf(controller, "usr_deploy").status).toBe("active");
  });

  it("lets someone with users.manage manage every agent", async () => {
    const controller = await mountAgents();

    expect(controller.rows.value.every((row) => row.canManage)).toBe(true);
  });

  it("lets an owner without users.manage manage only their own agent", async () => {
    signInAs(["tokens.personal"], { adminId: "usr_maya", role: "member" });
    const controller = await mountAgents();

    expect(rowOf(controller, "usr_bot").canManage).toBe(true);
    expect(rowOf(controller, "usr_deploy").canManage).toBe(false);
  });

  it("links each row to the agent's audit activity", async () => {
    const controller = await mountAgents();

    expect(rowOf(controller, "usr_bot").activity).toEqual({
      name: "audit",
      query: { actor: "usr_bot" },
    });
  });

  it("offers each owner once, sorted, as a filter", async () => {
    const twin = userFixture({
      ...reviewBot,
      id: "usr_twin",
      email: "twin-bot",
    });
    vi.mocked(usersApi.fetchUsers).mockResolvedValue([
      maya,
      lead,
      reviewBot,
      deployBot,
      twin,
    ]);
    const controller = await mountAgents();

    expect(controller.ownerFilters.value).toEqual([
      { value: "lead@example.com", label: "lead@example.com" },
      { value: "maya@example.com", label: "maya@example.com" },
    ]);
  });

  it("narrows the list by owner and by a case-insensitive name search", async () => {
    const controller = await mountAgents();
    const visible = () =>
      controller.visibleRows.value.map((row) => row.agent.id);

    controller.ownerFilter.value = "lead@example.com";
    expect(visible()).toEqual(["usr_deploy"]);

    controller.ownerFilter.value = null;
    controller.filter.value = "  REVIEW ";
    expect(visible()).toEqual(["usr_bot"]);

    controller.ownerFilter.value = "lead@example.com";
    expect(visible()).toEqual([]);
  });

  it("gives new agents the token-only role you may assign", async () => {
    const locked = roleFixture({
      ...agentRole,
      id: "rol_locked",
      assignable: false,
    });
    vi.mocked(rolesApi.fetchRoles).mockResolvedValue([
      memberRole,
      locked,
      agentRole,
    ]);
    const controller = await mountAgents();

    expect(controller.agentRole.value?.id).toBe("rol_ai_agent");
    expect(controller.canAdd.value).toBe(true);
  });

  it("hides Add when no token-only role can be assigned", async () => {
    vi.mocked(rolesApi.fetchRoles).mockResolvedValue([
      memberRole,
      roleFixture({ ...agentRole, assignable: false }),
    ]);
    const controller = await mountAgents();

    expect(controller.agentRole.value).toBeNull();
    expect(controller.canAdd.value).toBe(false);
  });

  it("hides Add from an owner without users.manage", async () => {
    signInAs(["tokens.personal"], { adminId: "usr_maya", role: "member" });
    const controller = await mountAgents();

    expect(controller.agentRole.value?.id).toBe("rol_ai_agent");
    expect(controller.canAdd.value).toBe(false);
  });

  it("opens the new agent's page once it is added", async () => {
    const controller = await mountAgents();
    controller.addOpen.value = true;

    await controller.added(deployBot);

    expect(controller.addOpen.value).toBe(false);
    expect(push).toHaveBeenCalledWith({
      name: "agent",
      params: { agentId: "usr_deploy" },
    });
  });

  it("loads the agent's grants when opening New token", async () => {
    const grants = [grantFixture({ principalId: "usr_bot", level: "read" })];
    vi.mocked(accessApi.fetchUserAccess).mockResolvedValue(
      accessFixture({ userId: "usr_bot", grants }),
    );
    const controller = await mountAgents();
    const row = rowOf(controller, "usr_bot");

    await controller.openToken(row);

    expect(accessApi.fetchUserAccess).toHaveBeenCalledWith("usr_bot");
    expect(controller.tokenFor.value).toEqual(row);
    expect(controller.tokenGrants.value).toEqual(grants);
  });

  it("still opens New token with no grants when they fail to load", async () => {
    vi.mocked(accessApi.fetchUserAccess)
      .mockResolvedValueOnce(accessFixture())
      .mockRejectedValueOnce(new Error("down"));
    const controller = await mountAgents();
    await controller.openToken(rowOf(controller, "usr_bot"));

    await controller.openToken(rowOf(controller, "usr_deploy"));

    expect(controller.tokenFor.value?.agent.id).toBe("usr_deploy");
    expect(controller.tokenGrants.value).toEqual([]);
  });

  it("closes New token, keeps the plaintext and reloads after a token is made", async () => {
    const controller = await mountAgents();
    await controller.openToken(rowOf(controller, "usr_bot"));
    const created = {
      token: tokenFixture({ id: "pat_2" }),
      plaintextToken: "dop_pat_x",
    };

    controller.tokenCreated(created);
    await settle();

    expect(controller.tokenFor.value).toBeNull();
    expect(controller.createdToken.value).toEqual(created);
    expect(usersApi.fetchUsers).toHaveBeenCalledTimes(2);
  });

  it("lets an owner without users.manage revoke their agent's token", async () => {
    signInAs(["tokens.personal"], { adminId: "usr_maya", role: "member" });
    const controller = await mountAgents();
    controller.askRevoke(rowOf(controller, "usr_bot"));

    await controller.confirmRevoke();

    expect(tokensApi.revokePersonalToken).toHaveBeenCalledWith(
      "usr_bot",
      "pat_1",
    );
    expect(controller.revoking.value).toBeNull();
    expect(controller.revokeError.value).toBeNull();
    expect(usersApi.fetchUsers).toHaveBeenCalledTimes(2);
  });

  it("keeps the confirmation open with the server's reason when revoking fails", async () => {
    vi.mocked(tokensApi.revokePersonalToken).mockRejectedValue(
      new ApiError(404, { TOKEN_NOT_FOUND: "That token no longer exists." }),
    );
    const controller = await mountAgents();
    controller.askRevoke(rowOf(controller, "usr_bot"));

    await controller.confirmRevoke();

    expect(controller.revoking.value?.agent.id).toBe("usr_bot");
    expect(controller.revokeError.value).toBe("That token no longer exists.");
    expect(controller.revokeBusy.value).toBe(false);
    expect(usersApi.fetchUsers).toHaveBeenCalledTimes(1);
  });

  it("does nothing when the agent has no live token to revoke", async () => {
    const controller = await mountAgents();
    controller.askRevoke(rowOf(controller, "usr_deploy"));

    await controller.confirmRevoke();

    expect(tokensApi.revokePersonalToken).not.toHaveBeenCalled();
  });

  it("ignores cancel while the revoke is still in flight", async () => {
    let finish!: () => void;
    vi.mocked(tokensApi.revokePersonalToken).mockReturnValue(
      new Promise((resolve) => {
        finish = () => resolve(tokenFixture({ revokedAt: NOW.toISOString() }));
      }),
    );
    const controller = await mountAgents();
    controller.askRevoke(rowOf(controller, "usr_bot"));

    const confirming = controller.confirmRevoke();
    controller.cancelRevoke();
    expect(controller.revokeBusy.value).toBe(true);
    expect(controller.revoking.value?.agent.id).toBe("usr_bot");

    finish();
    await confirming;
    expect(controller.revoking.value).toBeNull();
  });

  it("closes the revoke confirmation on cancel when nothing is in flight", async () => {
    const controller = await mountAgents();
    controller.askRevoke(rowOf(controller, "usr_bot"));

    controller.cancelRevoke();

    expect(controller.revoking.value).toBeNull();
  });

  it("closes Transfer owner and reloads once the agent moves", async () => {
    const controller = await mountAgents();
    controller.transferring.value = reviewBot;

    await controller.transferred();

    expect(controller.transferring.value).toBeNull();
    expect(usersApi.fetchUsers).toHaveBeenCalledTimes(2);
  });

  it("reloads the list after deleting an agent from its row menu", async () => {
    const controller = await mountAgents();
    controller.actions.ask("delete", deployBot);

    await controller.actions.confirm();

    expect(usersApi.deleteUser).toHaveBeenCalledWith("usr_deploy");
    expect(usersApi.fetchUsers).toHaveBeenCalledTimes(2);
  });
});

describe("agentTokenUsage", () => {
  const created = (overrides: Partial<PersonalToken>) => ({
    token: tokenFixture(overrides),
    plaintextToken: "dpp_example",
  });

  it("names the environment an environment token reaches", () => {
    expect(
      agentTokenUsage(
        created({ scopeType: "environment", scopeLabel: "payments/prod" }),
      ),
    ).toBe(
      "export DOPBASE_TOKEN=dpp_example\ndopbase run payments/prod -- <command>",
    );
  });

  it("leaves the environment to fill in for a project token", () => {
    expect(
      agentTokenUsage(
        created({ scopeType: "project", scopeLabel: "payments" }),
      ),
    ).toContain("dopbase run payments/<environment> -- <command>");
  });

  it("leaves both to fill in for an all-projects token", () => {
    expect(
      agentTokenUsage(created({ scopeType: "all", scopeLabel: "" })),
    ).toContain("dopbase run <project>/<environment> -- <command>");
  });
});
