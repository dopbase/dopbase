import { beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick, ref } from "vue";
import { useEmptyEnvironmentController } from "./EmptyEnvironment.controller";
import * as environmentsApi from "~/services/environments.api";
import type { Environment } from "~/services";
import { mountController } from "~/tests/mount-controller";

vi.mock("~/services/environments.api");

function environment(
  id: string,
  name: string,
  extra: Partial<Environment> = {},
): Environment {
  return {
    id,
    projectId: "prj_pay",
    projectName: "payments",
    name,
    level: "manage",
    secretCount: 0,
    createdAt: "",
    updatedAt: "",
    ...extra,
  };
}

const staging = environment("env_staging", "staging");

beforeEach(() => {
  vi.resetAllMocks();
});

function mount(active = true) {
  const activeRef = ref(active);
  const { controller } = mountController(() =>
    useEmptyEnvironmentController(
      ref("env_staging"),
      ref("payments"),
      activeRef,
    ),
  );
  return { controller, active: activeRef };
}

describe("useEmptyEnvironmentController", () => {
  it("shows the first-run checklist when no other environment has secrets", async () => {
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
      staging,
      environment("env_dev", "dev"),
    ]);
    const { controller } = mount();
    expect(controller.view.value).toBe("loading");

    await vi.waitFor(() => expect(controller.view.value).toBe("first-run"));
    expect(environmentsApi.listEnvironments).toHaveBeenCalledWith(
      "payments",
      expect.any(AbortSignal),
    );
  });

  it("offers the environments with secrets, most first, and whether their values can be copied", async () => {
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
      staging,
      environment("env_prod", "prod", { secretCount: 3, level: "metadata" }),
      environment("env_dev", "dev", { secretCount: 14, level: "read" }),
      environment("env_qa", "qa", { secretCount: 3, level: undefined }),
    ]);
    const { controller } = mount();

    await vi.waitFor(() => expect(controller.view.value).toBe("start"));
    expect(controller.sources.value).toEqual([
      { id: "env_dev", name: "dev", secretCount: 14, canCopyValues: true },
      { id: "env_prod", name: "prod", secretCount: 3, canCopyValues: false },
      { id: "env_qa", name: "qa", secretCount: 3, canCopyValues: true },
    ]);
  });

  it("treats an older server without counts, or a failed load, as a new project", async () => {
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
      staging,
      environment("env_dev", "dev", { secretCount: undefined }),
    ]);
    const first = mount();
    await vi.waitFor(() =>
      expect(first.controller.view.value).toBe("first-run"),
    );

    vi.mocked(environmentsApi.listEnvironments).mockRejectedValue(
      new Error("offline"),
    );
    const second = mount();
    await vi.waitFor(() =>
      expect(second.controller.view.value).toBe("first-run"),
    );
  });

  it("loads only while the environment is empty, and again each time it is", async () => {
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([staging]);
    const { controller, active } = mount(false);
    await nextTick();
    expect(environmentsApi.listEnvironments).not.toHaveBeenCalled();

    active.value = true;
    await vi.waitFor(() => expect(controller.view.value).toBe("first-run"));
    active.value = false;
    await nextTick();
    active.value = true;
    await vi.waitFor(() =>
      expect(environmentsApi.listEnvironments).toHaveBeenCalledTimes(2),
    );
  });
});
