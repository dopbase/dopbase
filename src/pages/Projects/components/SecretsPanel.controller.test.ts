import { beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick, ref } from "vue";
import { createPinia, setActivePinia } from "pinia";
import {
  REVEAL_SECONDS,
  useSecretsPanelController,
} from "./SecretsPanel.controller";
import { useReauthentication } from "~/composable";
import * as secretsApi from "~/services/secrets.api";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import { useAuthStore } from "~/stores/auth.store";

vi.mock("~/services/secrets.api");
vi.mock("~/services/auth.api");

beforeEach(() => {
  setActivePinia(createPinia());
  useReauthentication().dismiss();
  vi.mocked(secretsApi.listSecrets).mockReset();
  vi.mocked(secretsApi.setSecret).mockReset();
  vi.mocked(secretsApi.deleteSecret).mockReset();
  vi.mocked(secretsApi.revealSecret).mockReset();
  vi.mocked(secretsApi.getEnvLayout).mockReset();
  vi.mocked(secretsApi.exportSecrets).mockReset();
  vi.mocked(secretsApi.importSecrets).mockReset();
  vi.mocked(secretsApi.listSecrets).mockResolvedValue([]);
  vi.mocked(authApi.reauthenticate).mockReset();
});

const metadata = {
  key: "DATABASE_URL",
  version: 2,
  createdAt: "2026-08-28T00:00:00Z",
  updatedAt: "2026-08-28T00:00:00Z",
};

function makeController(environmentId = "env_1") {
  return mountController(() => useSecretsPanelController(ref(environmentId)))
    .controller;
}

const storedLayout = "# app\nDATABASE_URL=\nAPI_KEY=\n";
const exported = {
  revision: "revision-1",
  envLayout: storedLayout,
  entries: [
    { key: "DATABASE_URL", value: "postgres://secret" },
    { key: "API_KEY", value: "k-123" },
  ],
};

function mockEditorLoad(): void {
  vi.mocked(secretsApi.getEnvLayout).mockResolvedValue({
    layout: storedLayout,
  });
  vi.mocked(secretsApi.exportSecrets).mockResolvedValue(exported);
}

describe("useSecretsPanelController", () => {
  it("ignores an older environment response that resolves last", async () => {
    let resolveFirst!: (value: (typeof metadata)[]) => void;
    let resolveSecond!: (value: (typeof metadata)[]) => void;
    vi.mocked(secretsApi.listSecrets)
      .mockReturnValueOnce(new Promise((resolve) => (resolveFirst = resolve)))
      .mockReturnValueOnce(new Promise((resolve) => (resolveSecond = resolve)));
    const environmentId = ref("env_1");
    const c = mountController(() =>
      useSecretsPanelController(environmentId),
    ).controller;
    environmentId.value = "env_2";
    await nextTick();
    const current = { ...metadata, key: "CURRENT" };
    resolveSecond([current]);
    await vi.waitFor(() => expect(c.secrets.value).toEqual([current]));
    resolveFirst([{ ...metadata, key: "STALE" }]);
    await nextTick();
    expect(c.secrets.value).toEqual([current]);
    expect(c.loading.value).toBe(false);
  });

  it("drops the old environment's rows as soon as it changes", async () => {
    vi.mocked(secretsApi.listSecrets)
      .mockResolvedValueOnce([metadata])
      .mockReturnValueOnce(new Promise(() => {}));
    const environmentId = ref("env_1");
    const c = mountController(() =>
      useSecretsPanelController(environmentId),
    ).controller;
    await vi.waitFor(() => expect(c.secrets.value).toEqual([metadata]));

    environmentId.value = "env_2";
    await nextTick();

    expect(c.secrets.value).toBeNull();
  });

  it("loads metadata only, never values", async () => {
    vi.mocked(secretsApi.listSecrets).mockResolvedValueOnce([metadata]);
    const c = makeController();
    await vi.waitFor(() => expect(c.secrets.value).toEqual([metadata]));
    expect(c.secrets.value?.[0]).not.toHaveProperty("value");
  });

  it("surfaces listing failures", async () => {
    vi.mocked(secretsApi.listSecrets).mockRejectedValueOnce(new Error("down"));
    const c = makeController();
    await vi.waitFor(() =>
      expect(c.loadError.value).toBe("Could not load secrets."),
    );
  });

  it("reveal shows the plaintext and auto-hides after 30s", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(secretsApi.revealSecret).mockResolvedValueOnce({
        key: "DATABASE_URL",
        value: "postgres://secret",
        version: 2,
      });
      const c = makeController();
      await c.reveal("DATABASE_URL");
      expect(c.revealedKey.value).toBe("DATABASE_URL");
      expect(c.revealedValue.value).toBe("postgres://secret");
      expect(c.revealedValue.value).not.toContain("undefined");

      await vi.advanceTimersByTimeAsync((REVEAL_SECONDS - 1) * 1000);
      expect(c.revealedKey.value).toBe("DATABASE_URL");
      await vi.advanceTimersByTimeAsync(1000);
      expect(c.revealedKey.value).toBeNull();
      expect(c.revealedValue.value).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  it("hideRevealed drops the plaintext immediately", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(secretsApi.revealSecret).mockResolvedValueOnce({
        key: "DATABASE_URL",
        value: "postgres://secret",
        version: 2,
      });
      const c = makeController();
      await c.reveal("DATABASE_URL");
      c.hideRevealed();
      expect(c.revealedValue.value).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  it("drops revealed plaintext when the panel unmounts", async () => {
    vi.mocked(secretsApi.revealSecret).mockResolvedValueOnce({
      key: "DATABASE_URL",
      value: "postgres://secret",
      version: 2,
    });
    const { controller, unmount } = mountController(() =>
      useSecretsPanelController(ref("env_1")),
    );
    await controller.reveal("DATABASE_URL");
    expect(controller.revealedValue.value).toBe("postgres://secret");

    unmount();

    expect(controller.revealedKey.value).toBeNull();
    expect(controller.revealedValue.value).toBeNull();
  });

  it("parks reveal behind reauthentication and completes after confirming", async () => {
    vi.mocked(secretsApi.revealSecret)
      .mockRejectedValueOnce(
        new ApiError(403, {
          RECENT_AUTHENTICATION_REQUIRED: "confirm password",
        }),
      )
      .mockResolvedValueOnce({
        key: "DATABASE_URL",
        value: "postgres://secret",
        version: 2,
      });
    const c = makeController();
    const reveal = c.reveal("DATABASE_URL");
    await vi.waitFor(() =>
      expect(secretsApi.revealSecret).toHaveBeenCalledTimes(1),
    );
    // Parked: nothing revealed yet, exactly one attempted call.
    expect(c.revealedKey.value).toBeNull();
    expect(secretsApi.revealSecret).toHaveBeenCalledTimes(1);

    // Confirming the password re-runs the parked action exactly once.
    vi.mocked(authApi.reauthenticate).mockResolvedValueOnce();
    const reauth = useReauthentication();
    const ok = await reauth.submit("correct-password");
    await reveal;
    expect(ok).toBe(true);
    expect(secretsApi.revealSecret).toHaveBeenCalledTimes(2);
    expect(c.revealedKey.value).toBe("DATABASE_URL");
    expect(c.revealedValue.value).toBe("postgres://secret");
  });

  it("shows no save error behind the session-expired dialog", async () => {
    vi.mocked(secretsApi.setSecret).mockRejectedValue(
      new ApiError(401, { UNAUTHORIZED: "Sign in again." }),
    );
    const c = makeController();
    useAuthStore().sessionExpired = true;
    await expect(c.setSecret("DATABASE_URL", "new-value")).rejects.toThrow();
    expect(c.actionError.value).toBeNull();

    useAuthStore().sessionExpired = false;
    await expect(c.setSecret("DATABASE_URL", "new-value")).rejects.toThrow();
    expect(c.actionError.value).toBe("Could not save the secret.");
  });

  it("setSecret saves and reloads", async () => {
    vi.mocked(secretsApi.listSecrets).mockResolvedValue([metadata]);
    vi.mocked(secretsApi.setSecret).mockResolvedValueOnce(metadata);
    const c = makeController();
    await c.setSecret("DATABASE_URL", "new-value");
    expect(secretsApi.setSecret).toHaveBeenCalledWith(
      "env_1",
      "DATABASE_URL",
      "new-value",
    );
    await vi.waitFor(() => expect(c.secrets.value).toEqual([metadata]));
  });

  it("deleteSecret clears a revealed value for the same key", async () => {
    vi.mocked(secretsApi.revealSecret).mockResolvedValueOnce({
      key: "A",
      value: "v",
      version: 1,
    });
    vi.mocked(secretsApi.deleteSecret).mockResolvedValueOnce();
    const c = makeController();
    await c.reveal("A");
    expect(c.revealedKey.value).toBe("A");
    await c.deleteSecret("A");
    expect(c.revealedKey.value).toBeNull();
  });
});

describe("useSecretsPanelController — filter and copy", () => {
  it("filters keys without asking the server again", async () => {
    vi.mocked(secretsApi.listSecrets).mockResolvedValue([
      metadata,
      { ...metadata, key: "API_KEY" },
    ]);
    const c = makeController();
    await vi.waitFor(() => expect(c.secrets.value).toHaveLength(2));

    c.filter.value = "  api ";

    expect(c.visibleSecrets.value?.map((secret) => secret.key)).toEqual([
      "API_KEY",
    ]);
    expect(secretsApi.listSecrets).toHaveBeenCalledTimes(1);
  });

  it("copies a value without showing it", async () => {
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });
    vi.mocked(secretsApi.revealSecret).mockResolvedValue({
      key: "DATABASE_URL",
      value: "postgres://secret",
      version: 2,
    });
    const c = makeController();

    await c.copyValue("DATABASE_URL");

    expect(writeText).toHaveBeenCalledWith("postgres://secret");
    expect(c.copiedKey.value).toBe("DATABASE_URL");
    expect(c.revealedValue.value).toBeNull();
  });

  it("says so when the browser blocks the copy", async () => {
    Object.defineProperty(navigator, "clipboard", {
      value: {
        writeText: vi.fn(async () => {
          throw new Error("denied");
        }),
      },
      configurable: true,
    });
    Object.defineProperty(document, "execCommand", {
      value: vi.fn(() => false),
      configurable: true,
    });
    vi.mocked(secretsApi.revealSecret).mockResolvedValue({
      key: "DATABASE_URL",
      value: "postgres://secret",
      version: 2,
    });
    const c = makeController();

    await c.copyValue("DATABASE_URL");

    expect(c.copiedKey.value).toBeNull();
    expect(c.actionError.value).toBe(
      "The browser blocked the copy. Reveal the value and copy it from there.",
    );
  });
});

describe("useSecretsPanelController — .env editor", () => {
  it("openEditor merges the stored layout with the revealed values", async () => {
    mockEditorLoad();
    const c = makeController();
    const opening = c.openEditor();
    await vi.waitFor(() =>
      expect(secretsApi.exportSecrets).toHaveBeenCalledTimes(1),
    );
    await opening;
    expect(c.editorOpen.value).toBe(true);
    expect(c.editorContent.value).toBe(
      "# app\nDATABASE_URL=postgres://secret\nAPI_KEY=k-123\n",
    );
    expect(c.editorDirty.value).toBe(false);
  });

  it("openEditor generates a fresh layout when none is stored", async () => {
    vi.mocked(secretsApi.getEnvLayout).mockResolvedValueOnce({ layout: null });
    vi.mocked(secretsApi.exportSecrets).mockResolvedValueOnce(exported);
    const c = makeController();
    await c.openEditor();
    expect(c.editorContent.value).toBe(
      "DATABASE_URL=postgres://secret\nAPI_KEY=k-123",
    );
  });

  it("openEditor surfaces load failures and leaves the buffer empty", async () => {
    vi.mocked(secretsApi.getEnvLayout).mockRejectedValueOnce(new Error("down"));
    const c = makeController();
    await c.openEditor();
    expect(c.editorLoadError.value).toBe(
      "Could not load the secrets for editing.",
    );
    expect(c.editorContent.value).toBeNull();
  });

  it("parks the editor load behind reauthentication and completes after confirming", async () => {
    vi.mocked(secretsApi.getEnvLayout).mockResolvedValue({
      layout: storedLayout,
    });
    vi.mocked(secretsApi.exportSecrets)
      .mockRejectedValueOnce(
        new ApiError(403, {
          RECENT_AUTHENTICATION_REQUIRED: "confirm password",
        }),
      )
      .mockResolvedValueOnce(exported);
    const c = makeController();
    const opening = c.openEditor();
    await vi.waitFor(() =>
      expect(secretsApi.exportSecrets).toHaveBeenCalledTimes(1),
    );
    // Parked: nothing loaded yet, exactly one attempted export.
    expect(c.editorContent.value).toBeNull();
    expect(c.editorAwaitingReauth.value).toBe(true);
    expect(secretsApi.exportSecrets).toHaveBeenCalledTimes(1);

    // Confirming the password re-runs the parked load exactly once.
    vi.mocked(authApi.reauthenticate).mockResolvedValueOnce();
    const reauth = useReauthentication();
    const ok = await reauth.submit("correct-password");
    await opening;
    expect(ok).toBe(true);
    expect(secretsApi.exportSecrets).toHaveBeenCalledTimes(2);
    expect(c.editorContent.value).toBe(
      "# app\nDATABASE_URL=postgres://secret\nAPI_KEY=k-123\n",
    );
    expect(c.editorAwaitingReauth.value).toBe(false);
  });

  it("closeEditor wipes the plaintext buffer", async () => {
    mockEditorLoad();
    const c = makeController();
    await c.openEditor();
    c.closeEditor();
    expect(c.editorOpen.value).toBe(false);
    expect(c.editorContent.value).toBeNull();
  });

  it("changing the environment wipes the editor buffer", async () => {
    const environmentId = ref("env_1");
    const c = mountController(() =>
      useSecretsPanelController(environmentId),
    ).controller;
    mockEditorLoad();
    await c.openEditor();
    expect(c.editorContent.value).not.toBeNull();
    environmentId.value = "env_2";
    await vi.waitFor(() => expect(c.editorContent.value).toBeNull());
    expect(c.editorOpen.value).toBe(false);
  });
});

describe("useSecretsPanelController — .env editor save flow", () => {
  it("saveDraft is blocked while the buffer has issues", async () => {
    mockEditorLoad();
    const c = makeController();
    await c.openEditor();
    c.editorContent.value = "# broken\n1BAD=x\n";
    expect(c.editorIssues.value).toHaveLength(1);
    expect(c.editorCanSave.value).toBe(false);
    expect(await c.saveDraft()).toBe(false);
    expect(secretsApi.importSecrets).not.toHaveBeenCalled();
  });

  it("saveDraft is blocked while the buffer is clean", async () => {
    mockEditorLoad();
    const c = makeController();
    await c.openEditor();
    expect(c.editorDirty.value).toBe(false);
    expect(c.editorCanSave.value).toBe(false);
    expect(await c.saveDraft()).toBe(false);
    expect(secretsApi.importSecrets).not.toHaveBeenCalled();
  });

  it("saveDraft dry-runs and applyDraft submits replace mode with a value-free layout", async () => {
    mockEditorLoad();
    vi.mocked(secretsApi.importSecrets)
      .mockResolvedValueOnce({
        addedKeys: ["DATABASE_URL"],
        updatedKeys: [],
        unchangedKeys: ["API_KEY"],
        deletedKeys: [],
        dryRun: true,
        revision: "rev-1",
      })
      .mockResolvedValueOnce({
        addedKeys: ["DATABASE_URL"],
        updatedKeys: [],
        unchangedKeys: ["API_KEY"],
        deletedKeys: [],
        dryRun: false,
        revision: "rev-2",
      });
    const c = makeController();
    await c.openEditor();
    c.editorContent.value = "# app\nDATABASE_URL=new-value\nAPI_KEY=k-123\n";
    expect(await c.saveDraft()).toBe(true);
    expect(secretsApi.importSecrets).toHaveBeenNthCalledWith(
      1,
      "env_1",
      {
        mode: "replace",
        dryRun: true,
        entries: [
          { key: "DATABASE_URL", value: "new-value" },
          { key: "API_KEY", value: "k-123" },
        ],
      },
      expect.any(AbortSignal),
    );
    expect(c.editorDiff.value?.dryRun).toBe(true);

    expect(await c.applyDraft()).toBe(true);
    expect(secretsApi.importSecrets).toHaveBeenNthCalledWith(
      2,
      "env_1",
      {
        mode: "replace",
        dryRun: false,
        entries: [
          { key: "DATABASE_URL", value: "new-value" },
          { key: "API_KEY", value: "k-123" },
        ],
        envLayout: "# app\nDATABASE_URL=\nAPI_KEY=\n",
        expectedRevision: "rev-1",
      },
      expect.any(AbortSignal),
    );
    expect(c.editorDirty.value).toBe(false);
    expect(c.editorDiff.value).toBeNull();
  });

  it("applyDraft failures keep the diff pending and report the error", async () => {
    mockEditorLoad();
    vi.mocked(secretsApi.importSecrets)
      .mockResolvedValueOnce({
        addedKeys: [],
        updatedKeys: [],
        unchangedKeys: [],
        deletedKeys: [],
        dryRun: true,
        revision: "rev-1",
      })
      .mockRejectedValueOnce(new Error("down"));
    const c = makeController();
    await c.openEditor();
    c.editorContent.value = "DATABASE_URL=x\n";
    expect(await c.saveDraft()).toBe(true);
    expect(await c.applyDraft()).toBe(false);
    expect(c.editorError.value).toBe(
      "The changes could not be saved. Nothing may have changed — review and try again.",
    );
    expect(c.editorDiff.value).not.toBeNull();
    expect(c.editorDirty.value).toBe(true);
  });

  it("discardDraft reverts to the baseline and drops the diff", async () => {
    mockEditorLoad();
    const c = makeController();
    await c.openEditor();
    c.editorContent.value = "DATABASE_URL=changed\n";
    expect(c.editorDirty.value).toBe(true);
    c.discardDraft();
    expect(c.editorContent.value).toBe(
      "# app\nDATABASE_URL=postgres://secret\nAPI_KEY=k-123\n",
    );
    expect(c.editorDirty.value).toBe(false);
  });

  it("backToEditing returns from the confirmation summary", async () => {
    mockEditorLoad();
    const c = makeController();
    await c.openEditor();
    c.editorDiff.value = {
      addedKeys: [],
      updatedKeys: [],
      unchangedKeys: [],
      deletedKeys: [],
      dryRun: true,
      revision: "rev-1",
    };
    c.backToEditing();
    expect(c.editorDiff.value).toBeNull();
    expect(c.editorContent.value).not.toBeNull();
  });
});

describe("editor import password confirmation", () => {
  const preview = {
    addedKeys: [],
    updatedKeys: ["DATABASE_URL"],
    unchangedKeys: [],
    deletedKeys: [],
    dryRun: true,
    revision: "rev-1",
  };
  const challenge = () =>
    new ApiError(403, { RECENT_AUTHENTICATION_REQUIRED: "Confirm password." });

  it.each([true, false])(
    "retries a captured editor request with dryRun=%s",
    async (dryRun) => {
      mockEditorLoad();
      const c = makeController();
      await c.openEditor();
      c.editorContent.value = "DATABASE_URL=changed\n";
      c.editorDiff.value = dryRun ? null : preview;
      vi.mocked(secretsApi.importSecrets)
        .mockRejectedValueOnce(challenge())
        .mockResolvedValueOnce(preview);
      const saving = dryRun ? c.saveDraft() : c.applyDraft();
      await vi.waitFor(() =>
        expect(secretsApi.importSecrets).toHaveBeenCalledTimes(1),
      );
      expect(await (dryRun ? c.saveDraft() : c.applyDraft())).toBe(false);
      vi.mocked(authApi.reauthenticate).mockResolvedValueOnce();
      await useReauthentication().submit("fixture-password");
      expect(await saving).toBe(true);
      const calls = vi.mocked(secretsApi.importSecrets).mock.calls;
      expect(calls[1]).toEqual(calls[0]);
      expect(calls[1]?.[1].dryRun).toBe(dryRun);
      expect(c.editorSaving.value).toBe(false);
    },
  );

  it.each(["close", "change", "environment", "unmount", "dismiss", "back"])(
    "cancels a pending editor apply on %s",
    async (action) => {
      mockEditorLoad();
      const environment = ref("env_1");
      const { controller: c, unmount } = mountController(() =>
        useSecretsPanelController(environment),
      );
      await c.openEditor();
      c.editorContent.value = "DATABASE_URL=changed\n";
      c.editorDiff.value = preview;
      vi.mocked(secretsApi.importSecrets).mockRejectedValueOnce(challenge());
      const saving = c.applyDraft();
      await vi.waitFor(() =>
        expect(secretsApi.importSecrets).toHaveBeenCalledTimes(1),
      );
      await nextTick();
      if (action === "close") c.closeEditor();
      if (action === "change")
        c.editorContent.value = "DATABASE_URL=different\n";
      if (action === "environment") {
        environment.value = "env_2";
        await nextTick();
      }
      if (action === "unmount") unmount();
      if (action === "dismiss") useReauthentication().dismiss();
      if (action === "back") c.backToEditing();
      expect(await saving).toBe(false);
      vi.mocked(authApi.reauthenticate).mockResolvedValueOnce();
      await useReauthentication().submit("fixture-password");
      expect(secretsApi.importSecrets).toHaveBeenCalledTimes(1);
      expect(c.editorError.value).toBeNull();
    },
  );

  it("discards a stale preview after password confirmation", async () => {
    mockEditorLoad();
    const c = makeController();
    await c.openEditor();
    c.editorContent.value = "DATABASE_URL=changed\n";
    c.editorDiff.value = preview;
    vi.mocked(secretsApi.importSecrets)
      .mockRejectedValueOnce(challenge())
      .mockRejectedValueOnce(
        new ApiError(409, { IMPORT_PREVIEW_STALE: "Changed." }),
      );
    const saving = c.applyDraft();
    await vi.waitFor(() =>
      expect(secretsApi.importSecrets).toHaveBeenCalledTimes(1),
    );
    vi.mocked(authApi.reauthenticate).mockResolvedValueOnce();
    await useReauthentication().submit("fixture-password");
    expect(await saving).toBe(false);
    expect(c.editorDiff.value).toBeNull();
    expect(c.editorDirty.value).toBe(true);
    expect(c.editorError.value).toBe(
      "The secrets changed. Preview your changes again.",
    );
  });
});
