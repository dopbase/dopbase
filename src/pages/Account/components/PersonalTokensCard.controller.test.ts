import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { usePersonalTokensCardController } from "./PersonalTokensCard.controller";
import * as tokensApi from "~/services/personal-tokens.api";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import * as rolesApi from "~/services/roles.api";
import type { Capability } from "~/services/access.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";
import { mountController } from "~/tests/mount-controller";
import {
  adminRole,
  memberRole,
  settle,
  signInAs,
  tokenFixture,
} from "~/tests/people";

vi.mock("~/services/personal-tokens.api");
vi.mock("~/services/projects.api");
vi.mock("~/services/environments.api");
vi.mock("~/services/roles.api");

const { copyText } = vi.hoisted(() => ({ copyText: vi.fn() }));
vi.mock("~/utils/clipboard", () => ({ copyText }));

/** maya, a Member: her tokens reach project data. */
function signInAsMember(
  capabilities: Capability[] = ["tokens.personal"],
  tokenScope: "data" | "audit" = "data",
): void {
  signInAs(capabilities, { adminId: "usr_maya", role: "member" });
  Object.assign(useAuthStore().session!, {
    roleName: "Member",
    maxLevel: "project_manager",
    tokenScope,
  });
}

async function mounted() {
  const { controller } = mountController(usePersonalTokensCardController);
  await settle();
  return controller;
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(tokensApi.listPersonalTokens).mockResolvedValue([
    tokenFixture({
      id: "pat_live",
      name: "laptop-agent",
      level: "metadata",
      scopeType: "project",
      scopeLabel: "payments",
      lastUsedAt: new Date(Date.now() - 120_000).toISOString(),
    }),
    tokenFixture({ id: "pat_revoked", revokedAt: "2026-10-02T00:00:00Z" }),
    tokenFixture({ id: "pat_expired", expiresAt: "2020-01-01T00:00:00Z" }),
  ]);
  vi.mocked(tokensApi.revokePersonalToken).mockResolvedValue(tokenFixture());
  vi.mocked(projectsApi.listProjects).mockResolvedValue([
    { id: "prj_pay", name: "payments", createdAt: "", updatedAt: "" },
  ]);
  vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([]);
  vi.mocked(rolesApi.fetchRoles).mockResolvedValue([adminRole, memberRole]);
});

describe("usePersonalTokensCardController", () => {
  it("lists live tokens with what each can do, where, and when it was used", async () => {
    signInAsMember();
    const controller = await mounted();

    expect(controller.visible.value).toBe(true);
    expect(controller.rows.value).toHaveLength(1);
    expect(controller.rows.value[0]).toMatchObject({
      can: { label: "Names only", filled: 1 },
      where: "payments",
      lastUsed: "2m ago",
      expires: "Jan 1, 2099",
    });
    expect(controller.projects.value).toHaveLength(1);
  });

  it("stays hidden for root and roles without personal tokens", async () => {
    signInAs([], { role: "root" });
    const root = mountController(usePersonalTokensCardController).controller;
    signInAsMember([]);
    const member = mountController(usePersonalTokensCardController).controller;
    await settle();

    expect(root.visible.value).toBe(false);
    expect(member.visible.value).toBe(false);
    expect(tokensApi.listPersonalTokens).not.toHaveBeenCalled();
  });

  it("loads no projects for roles whose tokens read the audit log", async () => {
    signInAsMember(["tokens.personal", "audit.read_all"], "audit");
    const controller = await mounted();

    expect(controller.auditOnly.value).toBe(true);
    expect(projectsApi.listProjects).not.toHaveBeenCalled();
    expect(environmentsApi.listEnvironments).not.toHaveBeenCalled();
  });

  it("caps tokens at Write and a year when your role's limits are hidden", async () => {
    signInAsMember();
    const controller = await mounted();

    expect(rolesApi.fetchRoles).not.toHaveBeenCalled();
    expect(controller.limits.value).toEqual({ level: "write", days: 365 });
  });

  it("takes the token limits from your role when you can see roles", async () => {
    signInAsMember(["tokens.personal", "users.view"]);
    vi.mocked(rolesApi.fetchRoles).mockResolvedValue([
      { ...memberRole, tokenMaxLevel: "metadata", tokenMaxTtlDays: 30 },
    ]);
    const controller = await mounted();

    expect(controller.limits.value).toEqual({ level: "metadata", days: 30 });
  });

  it("never offers more than your deepest access", async () => {
    signInAsMember();
    useAuthStore().session!.maxLevel = "read";
    const controller = await mounted();

    expect(controller.limits.value.level).toBe("read");
  });

  it("shows a created token once and reloads the list", async () => {
    signInAsMember();
    const controller = await mounted();
    controller.dialogOpen.value = true;

    controller.tokenCreated({
      token: tokenFixture(),
      plaintextToken: "dop_pat_x",
    });
    await settle();

    expect(controller.dialogOpen.value).toBe(false);
    expect(controller.created.value?.plaintextToken).toBe("dop_pat_x");
    expect(tokensApi.listPersonalTokens).toHaveBeenCalledTimes(2);
    controller.acknowledge();
    expect(controller.created.value).toBeNull();
  });

  it("revokes a token only after you confirm", async () => {
    signInAsMember();
    const controller = await mounted();
    const token = controller.rows.value[0].token;

    controller.askRevoke(token);
    expect(tokensApi.revokePersonalToken).not.toHaveBeenCalled();
    await controller.confirmRevoke();

    expect(tokensApi.revokePersonalToken).toHaveBeenCalledWith(
      "me",
      "pat_live",
    );
    expect(controller.revoking.value).toBeNull();
    expect(tokensApi.listPersonalTokens).toHaveBeenCalledTimes(2);
  });

  it("keeps the confirmation open with the reason when revoking fails", async () => {
    signInAsMember();
    vi.mocked(tokensApi.revokePersonalToken).mockRejectedValue(
      new ApiError(404, { TOKEN_NOT_FOUND: "That token is gone." }),
    );
    const controller = await mounted();
    controller.askRevoke(controller.rows.value[0].token);

    await controller.confirmRevoke();

    expect(controller.revoking.value?.id).toBe("pat_live");
    expect(controller.revokeError.value).toBe("That token is gone.");
    controller.cancelRevoke();
    expect(controller.revoking.value).toBeNull();
  });

  it("copies a token's name and links to your activity", async () => {
    signInAsMember(["tokens.personal"], "audit");
    copyText.mockResolvedValue(true);
    const controller = await mounted();

    await controller.copyName(tokenFixture({ name: "splunk-forwarder" }));

    expect(copyText).toHaveBeenCalledWith("splunk-forwarder");
    expect(controller.notice.value).toBe("Copied splunk-forwarder.");
    expect(controller.activity.value).toEqual({
      name: "audit",
      query: { actor: "usr_maya" },
    });
  });

  it("shows how a collector pulls events from this server", async () => {
    signInAsMember(["tokens.personal"], "audit");
    const controller = await mounted();

    expect(controller.pullCommand.value).toBe(
      `curl -H "Authorization: Bearer $DOPBASE_TOKEN" \\\n  ${window.location.origin}/api/v1/audit-events`,
    );
  });
});
