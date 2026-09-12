import { createApp, h, nextTick, ref } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectsController } from "~/pages/Projects/Projects.controller";
import ProjectRail from "./ProjectRail.vue";

const mountedApps: Array<ReturnType<typeof createApp>> = [];

const billing = {
  id: "prj_billing",
  name: "billing",
  createdAt: "",
  updatedAt: "",
};

const payment = {
  id: "prj_payment",
  name: "payment",
  createdAt: "",
  updatedAt: "",
};

function createController() {
  return {
    projects: ref([billing, payment]),
    projectsError: ref(null),
    environments: ref([]),
    environmentsLoading: ref(false),
    environmentsError: ref(null),
    projectRef: ref("billing"),
    environmentId: ref(null),
    selectProject: vi.fn(),
    selectEnvironment: vi.fn(),
    createProject: vi.fn(async () => undefined),
    renameProject: vi.fn(async () => undefined),
    deleteProject: vi.fn(async () => ({
      projects: 1,
      environments: 0,
      secrets: 0,
      tokens: 0,
    })),
    createEnvironment: vi.fn(async () => undefined),
    renameEnvironment: vi.fn(async () => undefined),
    deleteEnvironment: vi.fn(async () => ({
      projects: 0,
      environments: 1,
      secrets: 0,
      tokens: 0,
    })),
    describeEnvironmentDeletion: vi.fn(async () => []),
  } as unknown as ProjectsController;
}

function mountProjectRail(controller: ProjectsController): void {
  const host = document.createElement("div");
  document.body.append(host);
  const app = createApp({
    render: () => h(ProjectRail, { controller }),
  });
  mountedApps.push(app);
  app.mount(host);
}

afterEach(() => {
  for (const app of mountedApps.splice(0)) app.unmount();
  document.body.innerHTML = "";
});

describe("ProjectRail", () => {
  it("renames the project whose action button was clicked", async () => {
    const controller = createController();
    mountProjectRail(controller);

    document.body
      .querySelector<HTMLButtonElement>('[aria-label="Rename payment"]')
      ?.click();
    await nextTick();

    const input = document.body.querySelector<HTMLInputElement>(
      'input[name="resource-name"]',
    );
    expect(input?.value).toBe("payment");
    if (!input) throw new Error("Rename input was not rendered.");
    input.value = "payment-service";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.closest("form")?.dispatchEvent(
      new Event("submit", { bubbles: true, cancelable: true }),
    );

    await vi.waitFor(() =>
      expect(controller.renameProject).toHaveBeenCalledWith(
        "prj_payment",
        "payment-service",
      ),
    );
    expect(controller.selectProject).not.toHaveBeenCalled();
  });

  it("deletes the project whose action button was clicked", async () => {
    const controller = createController();
    mountProjectRail(controller);

    document.body
      .querySelector<HTMLButtonElement>('[aria-label="Delete payment"]')
      ?.click();
    await nextTick();

    const dialog = document.body.querySelector<HTMLElement>(
      '[role="dialog"][aria-label="Delete project"]',
    );
    expect(dialog?.textContent).toContain("'payment'");
    if (!dialog) throw new Error("Delete project dialog was not rendered.");
    const input = dialog.querySelector<HTMLInputElement>("input");
    if (!input) throw new Error("Delete confirmation input was not rendered.");
    input.value = "payment";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await nextTick();
    Array.from(dialog.querySelectorAll("button"))
      .find((button) => button.textContent?.trim() === "Delete project")
      ?.click();

    await vi.waitFor(() =>
      expect(controller.deleteProject).toHaveBeenCalledWith("prj_payment"),
    );
    expect(controller.selectProject).not.toHaveBeenCalled();
  });
});
