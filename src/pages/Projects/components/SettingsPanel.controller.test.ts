import { beforeEach, describe, expect, it, vi } from "vitest";
import { ref } from "vue";
import { useSettingsPanelController } from "./SettingsPanel.controller";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import * as secretsApi from "~/services/secrets.api";
import * as tokensApi from "~/services/tokens.api";
import * as accessApi from "~/services/access.api";
import type { Environment, Project } from "~/services";
import { mountController } from "~/tests/mount-controller";

const { push, replace } = vi.hoisted(() => ({
  push: vi.fn(),
  replace: vi.fn(),
}));
vi.mock("vue-router", () => ({ useRouter: () => ({ push, replace }) }));
vi.mock("~/services/projects.api");
vi.mock("~/services/environments.api");
vi.mock("~/services/secrets.api");
vi.mock("~/services/tokens.api");
vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  fetchProjectAccess: vi.fn(),
}));

const payments: Project = {
  id: "prj_pay",
  name: "payments",
  level: "project_manager",
  createdAt: "",
  updatedAt: "",
};

function environment(
  id: string,
  name: string,
  extra: Partial<Environment> = {},
) {
  return {
    id,
    projectId: "prj_pay",
    projectName: "payments",
    name,
    level: "manage" as const,
    createdAt: "",
    updatedAt: "",
    ...extra,
  };
}

const dev = environment("env_dev", "dev");
const prod = environment("env_prod", "prod", { protected: true });
const secret = { key: "A", version: 1, createdAt: "", updatedAt: "" };
const token = {
  id: "tok",
  environmentId: "env_prod",
  name: "deploy",
  createdAt: "",
  expiresAt: null,
  lastUsedAt: null,
  revokedAt: null,
};

let changed = vi.fn();

function panel() {
  changed = vi.fn();
  return mountController(() =>
    useSettingsPanelController(ref(payments), ref([dev, prod]), changed),
  ).controller;
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(secretsApi.listSecrets).mockImplementation(async (id) =>
    id === "env_dev" ? [secret, secret] : [secret],
  );
  vi.mocked(tokensApi.listTokens).mockImplementation(async (id) =>
    id === "env_prod"
      ? [token, { ...token, id: "tok_old", revokedAt: "2026-01-01" }]
      : [],
  );
  vi.mocked(accessApi.fetchProjectAccess).mockResolvedValue({
    project: payments,
    environments: [dev, prod],
    grants: [
      { scopeId: "env_prod" },
      { scopeId: "prj_pay" },
    ] as accessApi.Grant[],
  });
});

describe("useSettingsPanelController", () => {
  it("counts secrets, live runner tokens and grants", async () => {
    const c = panel();

    await vi.waitFor(() =>
      expect(c.stats.value).toEqual({
        env_dev: { secrets: 2, tokens: 0 },
        env_prod: { secrets: 1, tokens: 1 },
      }),
    );
    await vi.waitFor(() =>
      expect(c.projectDeleteSummary.value).toBe(
        "Deletes 2 environments, 3 secrets, 1 runner token and 2 grants. Audit events stay.",
      ),
    );
  });

  it("cancels the counts still loading when it unmounts", async () => {
    vi.mocked(secretsApi.listSecrets).mockReturnValue(new Promise(() => {}));
    const { unmount } = mountController(() =>
      useSettingsPanelController(ref(payments), ref([dev, prod]), vi.fn()),
    );
    const signal = vi.mocked(secretsApi.listSecrets).mock.calls[0]![1]!;

    unmount();

    expect(signal.aborted).toBe(true);
  });

  it("renames the project and moves to its new address", async () => {
    vi.mocked(projectsApi.renameProject).mockResolvedValue({
      ...payments,
      name: "billing",
    });
    const c = panel();
    expect(c.nameChanged.value).toBe(false);

    c.name.value = "billing";
    expect(c.nameChanged.value).toBe(true);
    await c.saveName();

    expect(projectsApi.renameProject).toHaveBeenCalledWith(
      "prj_pay",
      "billing",
    );
    expect(replace).toHaveBeenCalledWith({
      name: "project-settings",
      params: { projectRef: "billing" },
    });
    expect(changed).toHaveBeenCalled();
  });

  it("turns protection on and off", async () => {
    vi.mocked(environmentsApi.setEnvironmentProtected).mockResolvedValue(dev);
    const c = panel();

    await c.setProtected(dev, true);

    expect(environmentsApi.setEnvironmentProtected).toHaveBeenCalledWith(
      "env_dev",
      true,
    );
    expect(changed).toHaveBeenCalled();
  });

  it("lists what deleting an environment removes, then deletes it", async () => {
    vi.mocked(environmentsApi.deleteEnvironment).mockResolvedValue({
      projects: 0,
      environments: 1,
      secrets: 1,
      tokens: 1,
    });
    const c = panel();
    await vi.waitFor(() => expect(c.stats.value.env_prod).toBeDefined());
    await vi.waitFor(() =>
      expect(c.projectDeleteSummary.value).toContain("2 grants"),
    );

    c.startDelete({ kind: "environment", environment: prod });
    expect(c.deleteCounts.value).toEqual([
      { label: "secret and its history", count: 1 },
      { label: "runner token", count: 1 },
      { label: "grant on this environment", count: 1 },
    ]);

    await c.confirmDelete();
    expect(environmentsApi.deleteEnvironment).toHaveBeenCalledWith("env_prod");
    expect(c.deleting.value).toBeNull();
    expect(changed).toHaveBeenCalled();
  });

  it("deletes the project and goes back to Projects", async () => {
    vi.mocked(projectsApi.deleteProject).mockResolvedValue({
      projects: 1,
      environments: 2,
      secrets: 3,
      tokens: 1,
    });
    const c = panel();

    c.startDelete({ kind: "project" });
    await c.confirmDelete();

    expect(projectsApi.deleteProject).toHaveBeenCalledWith("prj_pay");
    expect(push).toHaveBeenCalledWith({ name: "projects" });
  });

  it("creates and renames environments through the name dialog", async () => {
    const c = panel();

    c.dialog.value = { kind: "new-environment" };
    await c.submitName("qa");
    c.dialog.value = { kind: "rename-environment", environment: dev };
    await c.submitName("development");

    expect(environmentsApi.createEnvironment).toHaveBeenCalledWith(
      "payments",
      "qa",
    );
    expect(environmentsApi.renameEnvironment).toHaveBeenCalledWith(
      "env_dev",
      "development",
    );
    expect(changed).toHaveBeenCalledTimes(2);
  });
});
