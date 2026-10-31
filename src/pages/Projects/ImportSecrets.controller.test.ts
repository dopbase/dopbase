import { beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick, reactive } from "vue";
import { createPinia, setActivePinia } from "pinia";
import { useImportSecretsController } from "./ImportSecrets.controller";
import { useImportStore } from "~/stores/import.store";
import { useReauthentication } from "~/composable";
import * as secretsApi from "~/services/secrets.api";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";

vi.mock("~/services/secrets.api");
vi.mock("~/services/auth.api");
const navigation = vi.hoisted(() => ({ push: vi.fn(), replace: vi.fn() }));
let route: { params: { projectRef: string; environmentName: string } };
vi.mock("vue-router", () => ({
  useRoute: () => route,
  useRouter: () => navigation,
}));

const preview = {
  addedKeys: [],
  updatedKeys: ["APP_SECRETS"],
  unchangedKeys: [],
  deletedKeys: [],
  dryRun: true,
  revision: "revision-1",
};
function challenge() {
  return new ApiError(403, {
    RECENT_AUTHENTICATION_REQUIRED: "Confirm your password.",
  });
}

beforeEach(() => {
  setActivePinia(createPinia());
  useReauthentication().dismiss();
  vi.resetAllMocks();
  route = reactive({
    params: { projectRef: "project", environmentName: "dev" },
  });
  useImportStore().begin({
    environmentId: "env_1",
    environmentName: "dev",
    fileName: "test.env",
    entries: [{ key: "APP_SECRETS", value: "fixture-value" }],
    errors: [],
  });
  vi.mocked(authApi.reauthenticate).mockResolvedValue();
  vi.mocked(secretsApi.importSecrets).mockResolvedValue(preview);
});

describe("import password confirmation", () => {
  it("retries preview and apply with the captured request and prevents duplicate submissions", async () => {
    const { controller: c } = mountController(useImportSecretsController);
    vi.mocked(secretsApi.importSecrets).mockRejectedValueOnce(challenge());
    const validating = c.validate();
    await vi.waitFor(() =>
      expect(secretsApi.importSecrets).toHaveBeenCalledTimes(1),
    );
    await c.validate();
    expect(secretsApi.importSecrets).toHaveBeenCalledTimes(1);
    await useReauthentication().submit("fixture-password");
    await validating;
    expect(c.stage.value).toBe("dry");
    expect(vi.mocked(secretsApi.importSecrets).mock.calls[1]).toEqual(
      vi.mocked(secretsApi.importSecrets).mock.calls[0],
    );
    vi.mocked(secretsApi.importSecrets).mockRejectedValueOnce(challenge());
    const applying = c.apply();
    await vi.waitFor(() =>
      expect(secretsApi.importSecrets).toHaveBeenCalledTimes(3),
    );
    await c.apply();
    expect(secretsApi.importSecrets).toHaveBeenCalledTimes(3);
    await useReauthentication().submit("fixture-password");
    await applying;
    const calls = vi.mocked(secretsApi.importSecrets).mock.calls;
    expect(calls[3]).toEqual(calls[2]);
    expect(calls[3]?.[1]).toMatchObject({
      dryRun: false,
      expectedRevision: "revision-1",
    });
    expect(navigation.push).toHaveBeenCalledTimes(1);
    expect(c.working.value).toBe(false);
  });

  it.each([
    "cancel",
    "unmount",
    "environment",
    "draft",
    "entries",
    "mode",
    "dismiss",
  ])("does not resume a preview after %s", async (action) => {
    const { controller: c, unmount } = mountController(
      useImportSecretsController,
    );
    vi.mocked(secretsApi.importSecrets).mockRejectedValueOnce(challenge());
    const validating = c.validate();
    await vi.waitFor(() =>
      expect(secretsApi.importSecrets).toHaveBeenCalledTimes(1),
    );
    await nextTick();
    if (action === "cancel") c.cancel();
    if (action === "unmount") unmount();
    if (action === "environment") route.params.environmentName = "staging";
    if (action === "draft")
      useImportStore().begin({
        environmentId: "env_1",
        environmentName: "dev",
        fileName: "new.env",
        entries: [{ key: "OTHER", value: "new" }],
        errors: [],
      });
    if (action === "entries")
      useImportStore().pending!.entries[0]!.value = "changed";
    if (action === "mode") c.mode.value = "replace";
    if (action === "dismiss") useReauthentication().dismiss();
    await validating;
    await useReauthentication().submit("fixture-password");
    expect(secretsApi.importSecrets).toHaveBeenCalledTimes(1);
    expect(c.stage.value).toBe("review");
    expect(c.actionError.value).toBeNull();
  });

  it("does not apply a replaced draft after confirmation", async () => {
    const { controller: c } = mountController(useImportSecretsController);
    await c.validate();
    vi.mocked(secretsApi.importSecrets).mockRejectedValueOnce(challenge());
    const applying = c.apply();
    await vi.waitFor(() =>
      expect(secretsApi.importSecrets).toHaveBeenCalledTimes(2),
    );
    useImportStore().pending!.entries[0]!.value = "changed";
    await applying;
    await useReauthentication().submit("fixture-password");
    expect(secretsApi.importSecrets).toHaveBeenCalledTimes(2);
    expect(navigation.push).not.toHaveBeenCalled();
  });

  it("ignores a preview response that arrives after changing environments", async () => {
    const { controller: c } = mountController(useImportSecretsController);
    let finish!: (value: typeof preview) => void;
    vi.mocked(secretsApi.importSecrets).mockReturnValueOnce(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    const validating = c.validate();
    route.params.environmentName = "staging";
    finish(preview);
    await validating;
    expect(c.stage.value).toBe("review");
    expect(c.effectGroups.value).toEqual([]);
  });

  it("requires another preview when the revision becomes stale during confirmation", async () => {
    const { controller: c } = mountController(useImportSecretsController);
    await c.validate();
    vi.mocked(secretsApi.importSecrets)
      .mockRejectedValueOnce(challenge())
      .mockRejectedValueOnce(
        new ApiError(409, { IMPORT_PREVIEW_STALE: "Changed." }),
      );
    const applying = c.apply();
    await vi.waitFor(() =>
      expect(secretsApi.importSecrets).toHaveBeenCalledTimes(2),
    );
    await useReauthentication().submit("fixture-password");
    await applying;
    expect(c.stage.value).toBe("review");
    expect(c.actionError.value).toBe(
      "The secrets changed. Preview the import again.",
    );
    await c.apply();
    expect(secretsApi.importSecrets).toHaveBeenCalledTimes(3);
    expect(navigation.push).not.toHaveBeenCalled();
  });
});
