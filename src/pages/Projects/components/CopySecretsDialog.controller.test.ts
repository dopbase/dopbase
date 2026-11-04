import { beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick, ref } from "vue";
import { useCopySecretsDialogController } from "./CopySecretsDialog.controller";
import type { CopySource } from "./EmptyEnvironment.controller";
import * as secretsApi from "~/services/secrets.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";

vi.mock("~/services/secrets.api", async (original) => ({
  ...(await original<typeof secretsApi>()),
  listSecrets: vi.fn(),
  copySecrets: vi.fn(),
}));

const dev: CopySource = {
  id: "env_dev",
  name: "dev",
  secretCount: 3,
  canCopyValues: true,
};

function secret(key: string, version = 1) {
  return { key, version, createdAt: "", updatedAt: "" };
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(secretsApi.listSecrets).mockResolvedValue([
    secret("LOG_LEVEL"),
    secret("API_KEY", 4),
    secret("DATABASE_URL"),
  ]);
});

function mount(source: CopySource = dev, withValues = false) {
  const open = ref(false);
  const { controller } = mountController(() =>
    useCopySecretsDialogController(
      ref("env_staging"),
      ref(source),
      open,
      ref(withValues),
    ),
  );
  return { controller, open };
}

async function opened(source?: CopySource, withValues?: boolean) {
  const mounted = mount(source, withValues);
  mounted.open.value = true;
  await vi.waitFor(() => expect(mounted.controller.keys.value).not.toBeNull());
  return mounted;
}

describe("useCopySecretsDialogController", () => {
  it("lists the source's keys A to Z, all ticked, when it opens", async () => {
    const { controller, open } = mount();
    await nextTick();
    expect(secretsApi.listSecrets).not.toHaveBeenCalled();

    open.value = true;
    await vi.waitFor(() => expect(controller.keys.value).not.toBeNull());

    expect(secretsApi.listSecrets).toHaveBeenCalledWith(
      "env_dev",
      expect.any(AbortSignal),
    );
    expect(controller.keys.value?.map((row) => row.key)).toEqual([
      "API_KEY",
      "DATABASE_URL",
      "LOG_LEVEL",
    ]);
    expect(controller.selectedCount.value).toBe(3);
    expect(controller.allSelected.value).toBe(true);
    expect(controller.values.value).toBe(false);
  });

  it("unticks one key, then all, then ticks them again", async () => {
    const { controller } = await opened();

    controller.toggle("LOG_LEVEL", false);
    expect(controller.selectedCount.value).toBe(2);
    expect(controller.someSelected.value).toBe(true);
    expect(controller.allSelected.value).toBe(false);

    controller.toggleAll(false);
    expect(controller.selectedCount.value).toBe(0);
    expect(controller.someSelected.value).toBe(false);
    controller.toggleAll(true);
    expect(controller.allSelected.value).toBe(true);
  });

  it("copies the ticked keys, with values when asked", async () => {
    vi.mocked(secretsApi.copySecrets).mockResolvedValue({
      copiedKeys: ["API_KEY", "DATABASE_URL"],
      skippedKeys: [],
    });
    const { controller } = await opened(dev, true);
    expect(controller.values.value).toBe(true);
    controller.toggle("LOG_LEVEL", false);

    const result = await controller.submit();

    expect(secretsApi.copySecrets).toHaveBeenCalledWith(
      "env_staging",
      "env_dev",
      ["API_KEY", "DATABASE_URL"],
      true,
    );
    expect(result?.copiedKeys).toEqual(["API_KEY", "DATABASE_URL"]);
  });

  it("never asks for values from a source you can't read", async () => {
    const { controller } = await opened({ ...dev, canCopyValues: false }, true);

    expect(controller.values.value).toBe(false);
  });

  it("shows why a copy was refused and keeps the dialog's choices", async () => {
    vi.mocked(secretsApi.copySecrets).mockRejectedValue(
      new ApiError(422, {
        SECRET_LIMIT_EXCEEDED:
          "An environment may contain at most 500 secrets.",
      }),
    );
    const { controller } = await opened();

    await expect(controller.submit()).resolves.toBeNull();

    expect(controller.submitError.value).toBe(
      "An environment may contain at most 500 secrets.",
    );
    expect(controller.selectedCount.value).toBe(3);
    expect(controller.submitting.value).toBe(false);
  });

  it("says when the keys could not be loaded", async () => {
    vi.mocked(secretsApi.listSecrets).mockRejectedValue(new Error("offline"));
    const { controller, open } = mount();
    open.value = true;

    await vi.waitFor(() =>
      expect(controller.loadError.value).toBe(
        "Could not load the keys in dev.",
      ),
    );
  });
});
