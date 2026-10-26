import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { usePersonalTokensCardController } from "./PersonalTokensCard.controller";
import * as tokensApi from "~/services/personal-tokens.api";
import * as projectsApi from "~/services/projects.api";
import { useAuthStore } from "~/stores/auth.store";
import { mountController } from "~/tests/mount-controller";

vi.mock("~/services/personal-tokens.api");
vi.mock("~/services/projects.api");

const token = (
  overrides: Partial<tokensApi.PersonalToken>,
): tokensApi.PersonalToken => ({
  id: "ptk_1",
  userId: "usr_member",
  name: "laptop-agent",
  scopeType: "all",
  scopeId: "",
  scopeLabel: "",
  level: "read",
  createdBy: "usr_member",
  createdAt: "2026-10-01T00:00:00Z",
  expiresAt: "2999-01-01T00:00:00Z",
  lastUsedAt: null,
  revokedAt: null,
  ...overrides,
});

function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve));
}

function signIn(role: "root" | "member", capabilities: string[]): void {
  useAuthStore().session = {
    adminId: "usr_member",
    email: "member@example.com",
    sessionKind: "browser",
    recentAuthentication: true,
    role,
    capabilities: capabilities as never,
  };
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(tokensApi.listPersonalTokens).mockResolvedValue([
    token({}),
    token({ id: "ptk_revoked", revokedAt: "2026-10-02T00:00:00Z" }),
    token({ id: "ptk_expired", expiresAt: "2020-01-01T00:00:00Z" }),
  ]);
  vi.mocked(projectsApi.listProjects).mockResolvedValue([
    { id: "prj_pay", name: "payments", createdAt: "", updatedAt: "" },
  ]);
});

describe("usePersonalTokensCardController", () => {
  it("shows only active tokens", async () => {
    signIn("member", ["tokens.personal"]);
    const { controller } = mountController(usePersonalTokensCardController);
    await settle();
    expect(controller.visible.value).toBe(true);
    expect(controller.active.value.map((item) => item.id)).toEqual(["ptk_1"]);
    expect(controller.projects.value).toHaveLength(1);
  });

  it("stays hidden for root and roles without personal tokens", async () => {
    signIn("root", []);
    const root = mountController(usePersonalTokensCardController).controller;
    signIn("member", []);
    const member = mountController(usePersonalTokensCardController).controller;
    await settle();
    expect(root.visible.value).toBe(false);
    expect(member.visible.value).toBe(false);
    expect(tokensApi.listPersonalTokens).not.toHaveBeenCalled();
  });

  it("creates a token for every project or one project", async () => {
    signIn("member", ["tokens.personal"]);
    vi.mocked(tokensApi.createPersonalToken).mockResolvedValue({
      token: token({}),
      plaintextToken: "dpp_value",
    });
    const { controller } = mountController(usePersonalTokensCardController);
    await settle();
    await controller.create("laptop-agent", "metadata", "", "7d");
    expect(tokensApi.createPersonalToken).toHaveBeenLastCalledWith("me", {
      name: "laptop-agent",
      level: "metadata",
      expiresIn: "7d",
      scopeType: "all",
    });
    await controller.create("ci", "write", "prj_pay", "30d");
    expect(tokensApi.createPersonalToken).toHaveBeenLastCalledWith("me", {
      name: "ci",
      level: "write",
      expiresIn: "30d",
      scopeType: "project",
      scope: "prj_pay",
    });
    expect(controller.created.value?.plaintextToken).toBe("dpp_value");
    controller.acknowledge();
    expect(controller.created.value).toBeNull();
  });
});
