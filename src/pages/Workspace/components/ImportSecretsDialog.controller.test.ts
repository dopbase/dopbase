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
    const controller = useImportSecretsDialogController(ref("env_1"), ref(true));
    const file = new File(["DATABASE_URL=postgres://db"], "production.env");

    await expect(controller.processFile(file)).resolves.toBe(true);
    expect(useImportStore().pending).toMatchObject({
      environmentId: "env_1",
      fileName: "production.env",
      entries: [{ key: "DATABASE_URL", value: "postgres://db" }],
    });
    expect(push).toHaveBeenCalledWith({
      name: "environment-import",
      params: { projectRef: "project", environmentId: "env_1" },
    });
  });

  it("keeps the dialog open when the file contains no valid entries", async () => {
    const controller = useImportSecretsDialogController(ref("env_1"), ref(true));
    const file = new File(["not an assignment"], "broken.env");

    await expect(controller.processFile(file)).resolves.toBe(false);
    expect(controller.parseErrors.value).toEqual([
      "Line 1: expected KEY=value.",
    ]);
    expect(push).not.toHaveBeenCalled();
  });
});
