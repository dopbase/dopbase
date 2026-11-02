import { createApp } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as secretsApi from "~/services/secrets.api";
import * as tokensApi from "~/services/tokens.api";
import * as accessApi from "~/services/access.api";
import type { Environment, Project } from "~/services";
import SettingsPanel from "./SettingsPanel.vue";

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
}));
vi.mock("~/services/projects.api");
vi.mock("~/services/environments.api");
vi.mock("~/services/secrets.api");
vi.mock("~/services/tokens.api");
vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  fetchProjectAccess: vi.fn(),
}));

const payments: Project = {
  id: "prj_1",
  name: "payments",
  level: "project_manager",
  createdAt: "",
  updatedAt: "",
};

function environment(id: string, name: string): Environment {
  return {
    id,
    projectId: "prj_1",
    projectName: "payments",
    name,
    level: "manage",
    createdAt: "",
    updatedAt: "",
  };
}

let app: ReturnType<typeof createApp> | null = null;

beforeEach(() => {
  vi.mocked(secretsApi.listSecrets).mockResolvedValue([]);
  vi.mocked(tokensApi.listTokens).mockResolvedValue([]);
  vi.mocked(accessApi.fetchProjectAccess).mockResolvedValue({
    grants: [],
  } as unknown as Awaited<ReturnType<typeof accessApi.fetchProjectAccess>>);
});

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
});

describe("SettingsPanel", () => {
  it("shows each environment's ID in its row", () => {
    const host = document.createElement("div");
    document.body.append(host);
    app = createApp(SettingsPanel, {
      project: payments,
      environments: [environment("env_1", "dev"), environment("env_2", "prod")],
    });
    app.mount(host);

    const rows = host.querySelectorAll('[data-testid="settings-environment"]');
    expect(
      Array.from(rows, (row) =>
        row
          .querySelector('[data-testid="environment-id"]')
          ?.textContent?.trim(),
      ),
    ).toEqual(["env_1", "env_2"]);
  });
});
