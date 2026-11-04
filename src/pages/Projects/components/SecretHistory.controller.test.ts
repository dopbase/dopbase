import { beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick, ref } from "vue";
import { cacheContents, freshPinia } from "~/tests/pinia";
import {
  HISTORY_PAGE_SIZE,
  changeLabel,
  useSecretHistoryController,
} from "./SecretHistory.controller";
import { REVEAL_SECONDS } from "./SecretsPanel.controller";
import { useReauthentication } from "~/composable";
import * as secretsApi from "~/services/secrets.api";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import type { SecretVersion, SecretVersionPage } from "~/services/secrets.api";
import { mountController } from "~/tests/mount-controller";

vi.mock("~/services/secrets.api");
vi.mock("~/services/auth.api");

function version(
  number: number,
  overrides: Partial<SecretVersion> = {},
): SecretVersion {
  return {
    version: number,
    changeType: "updated",
    sourceVersion: null,
    actorType: "admin",
    actorLabel: "admin@example.com",
    createdAt: "2026-10-20T00:00:00Z",
    current: false,
    restorable: true,
    ...overrides,
  };
}

function page(
  items: SecretVersion[],
  nextBefore: number | null = null,
): SecretVersionPage {
  return { items, nextBefore, retainedVersions: 20 };
}

const firstPage = page([version(3, { current: true }), version(2)], 2);

beforeEach(() => {
  freshPinia();
  useReauthentication().dismiss();
  vi.mocked(secretsApi.listSecretVersions).mockReset();
  vi.mocked(secretsApi.revealSecretVersion).mockReset();
  vi.mocked(secretsApi.rollbackSecret).mockReset();
  vi.mocked(authApi.reauthenticate).mockReset();
  vi.mocked(secretsApi.listSecretVersions).mockResolvedValue(firstPage);
});

function makeController(key: string | null = "API_KEY") {
  const secretKey = ref<string | null>(key);
  const controller = mountController(() =>
    useSecretHistoryController(ref("env_1"), secretKey),
  ).controller;
  return { controller, secretKey };
}

describe("changeLabel", () => {
  it("names each kind of change", () => {
    expect(changeLabel(version(1, { changeType: "created" }))).toBe("Created");
    expect(changeLabel(version(1, { changeType: "imported" }))).toBe(
      "Imported",
    );
    expect(
      changeLabel(version(4, { changeType: "rolled_back", sourceVersion: 2 })),
    ).toBe("Rolled back from v2");
    expect(changeLabel(version(1, { changeType: "legacy" }))).toBe(
      "Before history",
    );
  });
});

describe("useSecretHistoryController", () => {
  it("loads the first page and appends older versions", async () => {
    const { controller: c } = makeController();
    await vi.waitFor(() => expect(c.versions.value).toHaveLength(2));
    expect(secretsApi.listSecretVersions).toHaveBeenCalledWith(
      "env_1",
      "API_KEY",
      expect.objectContaining({ limit: HISTORY_PAGE_SIZE }),
    );
    expect(c.current.value?.version).toBe(3);
    expect(c.retainedVersions.value).toBe(20);
    expect(c.nextBefore.value).toBe(2);

    vi.mocked(secretsApi.listSecretVersions).mockResolvedValueOnce(
      page([version(1, { changeType: "created" })]),
    );
    await c.loadMore();
    expect(secretsApi.listSecretVersions).toHaveBeenLastCalledWith(
      "env_1",
      "API_KEY",
      expect.objectContaining({ before: 2 }),
    );
    expect(c.versions.value?.map((item) => item.version)).toEqual([3, 2, 1]);
    expect(c.nextBefore.value).toBeNull();
  });

  it("does nothing while no secret is selected and resets when it changes", async () => {
    const { controller: c, secretKey } = makeController(null);
    await nextTick();
    expect(secretsApi.listSecretVersions).not.toHaveBeenCalled();
    expect(c.versions.value).toBeNull();

    secretKey.value = "API_KEY";
    await vi.waitFor(() => expect(c.versions.value).toHaveLength(2));
    secretKey.value = null;
    await nextTick();
    expect(c.versions.value).toBeNull();
  });

  it("drops a response for a secret that is no longer selected", async () => {
    let resolveStale!: (value: SecretVersionPage) => void;
    vi.mocked(secretsApi.listSecretVersions)
      .mockReturnValueOnce(new Promise((resolve) => (resolveStale = resolve)))
      .mockResolvedValueOnce(page([version(9, { current: true })]));
    const { controller: c, secretKey } = makeController("STALE");
    secretKey.value = "CURRENT";
    await vi.waitFor(() => expect(c.versions.value).toHaveLength(1));
    resolveStale(firstPage);
    await nextTick();
    expect(c.versions.value?.[0]?.version).toBe(9);
  });

  it("reveals an earlier value after reauthentication and hides it after 30s", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(secretsApi.revealSecretVersion)
        .mockRejectedValueOnce(
          new ApiError(403, { RECENT_AUTHENTICATION_REQUIRED: "confirm" }),
        )
        .mockResolvedValueOnce({
          key: "API_KEY",
          value: "old-value",
          version: 2,
        });
      const { controller: c } = makeController();
      await vi.waitFor(() => expect(c.versions.value).toHaveLength(2));
      const reveal = c.reveal(2);
      await vi.waitFor(() =>
        expect(secretsApi.revealSecretVersion).toHaveBeenCalledTimes(1),
      );
      expect(c.revealedValue.value).toBeNull();

      vi.mocked(authApi.reauthenticate).mockResolvedValueOnce();
      await useReauthentication().submit("correct-password");
      await reveal;
      expect(secretsApi.revealSecretVersion).toHaveBeenLastCalledWith(
        "env_1",
        "API_KEY",
        2,
      );
      expect(c.revealedVersion.value).toBe(2);
      expect(c.revealedValue.value).toBe("old-value");
      // Kept in this controller only; the cached history is metadata.
      expect(cacheContents()).not.toContain("old-value");

      await vi.advanceTimersByTimeAsync(REVEAL_SECONDS * 1000);
      expect(c.revealedVersion.value).toBeNull();
      expect(c.revealedValue.value).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  it("rolls back against the current version it showed and reloads", async () => {
    const { controller: c } = makeController();
    await vi.waitFor(() => expect(c.versions.value).toHaveLength(2));
    vi.mocked(secretsApi.rollbackSecret).mockResolvedValueOnce({
      key: "API_KEY",
      version: 4,
      sourceVersion: 2,
      previousVersion: 3,
      createdAt: "2026-10-01T00:00:00Z",
      updatedAt: "2026-10-23T00:00:00Z",
    });
    vi.mocked(secretsApi.listSecretVersions).mockResolvedValueOnce(
      page([
        version(4, {
          current: true,
          changeType: "rolled_back",
          sourceVersion: 2,
        }),
        version(3),
        version(2),
      ]),
    );

    c.askRollback(c.versions.value![1]!);
    expect(c.rollbackTarget.value?.version).toBe(2);
    const result = await c.confirmRollback();

    expect(secretsApi.rollbackSecret).toHaveBeenCalledWith(
      "env_1",
      "API_KEY",
      2,
      3,
    );
    expect(result?.version).toBe(4);
    expect(c.rollbackTarget.value).toBeNull();
    expect(c.notice.value).toBe("Rolled back to v2. Now at v4.");
    expect(c.current.value?.version).toBe(4);
  });

  it("reloads the history when the secret changed underneath the rollback", async () => {
    const { controller: c } = makeController();
    await vi.waitFor(() => expect(c.versions.value).toHaveLength(2));
    vi.mocked(secretsApi.rollbackSecret).mockRejectedValueOnce(
      new ApiError(409, { SECRET_VERSION_CONFLICT: "changed" }),
    );
    vi.mocked(secretsApi.listSecretVersions).mockResolvedValueOnce(
      page([version(4, { current: true }), version(3), version(2)]),
    );

    c.askRollback(c.versions.value![1]!);
    expect(await c.confirmRollback()).toBeNull();

    expect(c.rollbackError.value).toContain("changed while you were looking");
    expect(c.rollbackTarget.value?.version).toBe(2);
    expect(c.current.value?.version).toBe(4);
  });

  it("explains expired versions and stays quiet when reauthentication is dismissed", async () => {
    const { controller: c } = makeController();
    await vi.waitFor(() => expect(c.versions.value).toHaveLength(2));
    vi.mocked(secretsApi.rollbackSecret).mockRejectedValueOnce(
      new ApiError(410, { SECRET_VERSION_EXPIRED: "gone" }),
    );
    c.askRollback(c.versions.value![1]!);
    await c.confirmRollback();
    expect(c.rollbackError.value).toContain("past the retention limit");

    vi.mocked(secretsApi.rollbackSecret).mockRejectedValueOnce(
      new ApiError(403, { RECENT_AUTHENTICATION_REQUIRED: "confirm" }),
    );
    c.askRollback(c.versions.value![1]!);
    const pending = c.confirmRollback();
    await vi.waitFor(() =>
      expect(secretsApi.rollbackSecret).toHaveBeenCalledTimes(2),
    );
    // Let the rejected call reach the reauthentication queue before dismissing.
    await new Promise((resolve) => setTimeout(resolve, 0));
    useReauthentication().dismiss();
    expect(await pending).toBeNull();
    expect(c.rollbackError.value).toBeNull();
  });
});
