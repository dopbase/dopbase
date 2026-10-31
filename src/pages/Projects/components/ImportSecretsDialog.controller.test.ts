import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { ref } from "vue";
import { useImportStore } from "~/stores/import.store";
import { useImportSecretsDialogController } from "./ImportSecretsDialog.controller";

const push = vi.fn();
vi.mock("vue-router", () => ({
  useRoute: () => ({ params: { projectRef: "project" } }),
  useRouter: () => ({ push }),
}));

beforeEach(() => {
  setActivePinia(createPinia());
  push.mockReset();
});

describe("useImportSecretsDialogController", () => {
  it("stores a parsed file and navigates to its review route", async () => {
    const controller = useImportSecretsDialogController(
      ref("env_1"),
      ref("production"),
      ref(true),
    );
    const file = new File(["DATABASE_URL=postgres://db"], "production.env");

    await expect(controller.processFile(file)).resolves.toBe(true);
    expect(useImportStore().pending).toMatchObject({
      environmentId: "env_1",
      environmentName: "production",
      fileName: "production.env",
      entries: [{ key: "DATABASE_URL", value: "postgres://db" }],
    });
    expect(push).toHaveBeenCalledWith({
      name: "environment-import",
      params: { projectRef: "project", environmentName: "production" },
    });
  });

  it("imports into the environment open when the file was picked", async () => {
    const environmentId = ref("env_1");
    const environmentName = ref("production");
    const controller = useImportSecretsDialogController(
      environmentId,
      environmentName,
      ref(true),
    );
    const file = new File(["A=1"], "a.env");
    const result = controller.processFile(file);
    environmentId.value = "env_2";
    environmentName.value = "staging";

    await result;

    expect(useImportStore().pending?.environmentId).toBe("env_1");
    expect(push).toHaveBeenCalledWith(
      expect.objectContaining({
        params: { projectRef: "project", environmentName: "production" },
      }),
    );
  });

  it("drops the staged file when the review screen doesn't open", async () => {
    push.mockResolvedValueOnce({ type: 4 });
    const controller = useImportSecretsDialogController(
      ref("env_1"),
      ref("production"),
      ref(true),
    );

    await expect(
      controller.processFile(new File(["A=1"], "a.env")),
    ).resolves.toBe(false);
    expect(useImportStore().pending).toBeNull();
  });

  it("keeps the dialog open when the file contains no valid entries", async () => {
    const controller = useImportSecretsDialogController(
      ref("env_1"),
      ref("production"),
      ref(true),
    );
    const file = new File(["not an assignment"], "broken.env");

    await expect(controller.processFile(file)).resolves.toBe(false);
    expect(controller.parseErrors.value).toEqual([
      "Line 1: expected KEY=value.",
    ]);
    expect(push).not.toHaveBeenCalled();
  });
});
