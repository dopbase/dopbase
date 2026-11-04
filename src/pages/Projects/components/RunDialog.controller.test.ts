import { beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick, ref } from "vue";
import { isActiveToken, useRunDialogController } from "./RunDialog.controller";
import * as tokensApi from "~/services/tokens.api";
import type { RunnerToken } from "~/services/tokens.api";
import type { Environment } from "~/services";
import { mountController } from "~/tests/mount-controller";

vi.mock("~/services/tokens.api");

const prod: Environment = {
  id: "env_482731",
  projectId: "prj_1",
  projectName: "payments",
  name: "prod",
  createdAt: "",
  updatedAt: "",
};

function token(overrides: Partial<RunnerToken>): RunnerToken {
  return {
    id: "tok_1",
    environmentId: "env_482731",
    name: "deploy-prod",
    createdAt: "2026-10-01T00:00:00Z",
    expiresAt: null,
    lastUsedAt: null,
    revokedAt: null,
    ...overrides,
  };
}

function dialog(canManage = true, open = true) {
  const isOpen = ref(open);
  const controller = mountController(() =>
    useRunDialogController(ref("payments"), ref(prod), ref(canManage), isOpen),
  ).controller;
  return { controller, isOpen };
}

beforeEach(() => {
  vi.mocked(tokensApi.listTokens).mockReset();
  vi.mocked(tokensApi.listTokens).mockResolvedValue([]);
});

describe("useRunDialogController", () => {
  it("signs in, then runs by project and environment name", () => {
    const { controller: c } = dialog();

    expect(c.command.value).toBe(
      "dopbase login\ndopbase run payments/prod -- npm start",
    );
    expect(c.defaultCommand.value).toBe("dopbase env default payments/prod");
  });

  it("runs Docker and GitHub Actions by the environment id and a token", async () => {
    const { controller: c } = dialog();
    const origin = window.location.origin;

    c.target.value = "docker";
    await nextTick();
    expect(c.command.value).toContain(`-e DOPBASE_URL=${origin}`);
    expect(c.command.value).toContain("dopbase run env_482731 -- npm start");

    c.target.value = "github";
    await nextTick();
    expect(c.command.value).toContain(
      "run: dopbase run env_482731 -- npm test",
    );
    expect(c.command.value).toContain(
      "DOPBASE_TOKEN: ${{ secrets.DOPBASE_TOKEN }}",
    );
  });

  it("names the active token deploys used most recently", async () => {
    vi.mocked(tokensApi.listTokens).mockResolvedValue([
      token({ id: "tok_old", name: "old", lastUsedAt: "2026-10-02T00:00:00Z" }),
      token({
        id: "tok_new",
        lastUsedAt: "2026-10-29T00:00:00Z",
        expiresAt: "2099-01-25T00:00:00Z",
      }),
      token({ id: "tok_revoked", revokedAt: "2026-10-03" }),
    ]);
    const { controller: c } = dialog();

    await vi.waitFor(() => expect(c.tokensLoaded.value).toBe(true));
    expect(c.deployToken.value?.id).toBe("tok_new");
    expect(c.deployTokenSummary.value).toMatch(
      /^Last read .+\. Expires .+2099\.$/,
    );
  });

  it("loads tokens only once opened, and only for managers", async () => {
    const closed = dialog(true, false);
    expect(tokensApi.listTokens).not.toHaveBeenCalled();
    closed.isOpen.value = true;
    await vi.waitFor(() => expect(tokensApi.listTokens).toHaveBeenCalledOnce());

    vi.mocked(tokensApi.listTokens).mockClear();
    dialog(false);
    expect(tokensApi.listTokens).not.toHaveBeenCalled();
  });
});

describe("isActiveToken", () => {
  it("rejects revoked and expired tokens", () => {
    expect(isActiveToken(token({}))).toBe(true);
    expect(isActiveToken(token({ revokedAt: "2026-01-01" }))).toBe(false);
    expect(isActiveToken(token({ expiresAt: "2020-01-01" }))).toBe(false);
  });
});
