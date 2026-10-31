/* eslint-disable vue/one-component-per-file */
import { createApp, defineComponent, h, nextTick } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";

const { controller } = await vi.hoisted(async () => {
  const { ref } = await import("vue");
  const payments = {
    id: "prj_1",
    name: "payments",
    createdAt: "",
    updatedAt: "",
  };
  const prod = {
    id: "env_1",
    projectId: "prj_1",
    projectName: "payments",
    name: "prod",
    protected: true,
    createdAt: "",
    updatedAt: "",
  };
  return {
    controller: {
      projects: ref<unknown[] | null>([payments]),
      loadError: ref<string | null>(null),
      load: vi.fn(),
      filter: ref(""),
      rows: ref<unknown[]>([
        {
          project: payments,
          environmentCount: 8,
          expanded: true,
          environments: [
            {
              environment: prod,
              meter: { label: "Names only", filled: 1, strong: false },
              updated: "4m ago",
              canManage: true,
            },
          ],
          hiddenCount: 2,
          canManage: true,
        },
      ]),
      toggleProject: vi.fn(),
      showAllEnvironments: vi.fn(),
      openEnvironment: vi.fn(),
      openProjectAccess: vi.fn(),
      canCreateProjects: ref(true),
      nameDialog: ref<unknown>(null),
      nameDialogCopy: ref({
        title: "New project",
        label: "Project name",
        initialName: "",
        submitLabel: "Create",
        hint: "",
        placeholder: "",
      }),
      submitName: vi.fn(async () => undefined),
      deleting: ref(null),
      deleteCounts: ref([]),
      deleteLoading: ref(false),
      deleteError: ref(null),
      startDelete: vi.fn(),
      cancelDelete: vi.fn(),
      confirmDelete: vi.fn(),
    },
  };
});

vi.mock("./Projects.controller", () => ({
  useProjectsController: () => controller,
}));

vi.mock("~/layouts", async () => {
  const { defineComponent: define, h: render } = await import("vue");
  return {
    DashboardLayout: define({
      setup(_, { slots }) {
        return () => render("div", slots.default?.());
      },
    }),
  };
});

import ProjectsPage from "./Projects.page.vue";

let app: ReturnType<typeof createApp> | null = null;
const initialRows = controller.rows.value;
const initialProjects = controller.projects.value;

function mountPage(): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({ render: () => h(ProjectsPage) });
  app.component(
    "RouterLink",
    defineComponent({
      props: { to: { type: Object, required: true } },
      setup(_, { slots, attrs }) {
        return () => h("a", { ...attrs, href: "#" }, slots.default?.());
      },
    }),
  );
  app.mount(host);
  return host;
}

function button(text: string): HTMLButtonElement | undefined {
  return Array.from(document.querySelectorAll("button")).find(
    (candidate) => candidate.textContent?.trim() === text,
  );
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  controller.projects.value = initialProjects;
  controller.rows.value = initialRows;
  controller.canCreateProjects.value = true;
  controller.filter.value = "";
  controller.nameDialog.value = null;
  vi.clearAllMocks();
});

describe("Projects page", () => {
  it("shows projects as a tree with each environment's access", () => {
    const host = mountPage();

    const tree = host.querySelector('[data-testid="project-tree"]');
    expect(tree?.textContent).toContain("payments");
    expect(tree?.textContent).toContain("8 environments");
    expect(tree?.textContent).toContain("prod");
    expect(tree?.textContent).toContain("protected");
    expect(tree?.textContent).toContain("Names only");
    expect(tree?.textContent).toContain("Show all 8 environments");
  });

  it("opens an environment from its row", () => {
    const host = mountPage();

    host
      .querySelector<HTMLAnchorElement>(
        'a[aria-label="Open secrets in payments / prod"]',
      )!
      .click();

    expect(controller.openEnvironment).toHaveBeenCalledWith(
      expect.objectContaining({ id: "env_1" }),
    );
  });

  it("toggles a project and shows all of its environments", () => {
    mountPage();

    // The project row's toggle is the button that reports aria-expanded.
    Array.from(document.querySelectorAll("button[aria-expanded]"))
      .find((candidate) => candidate.textContent?.includes("payments"))
      ?.dispatchEvent(new MouseEvent("click"));
    button("Show all 8 environments")?.click();

    expect(controller.toggleProject).toHaveBeenCalledWith("prj_1");
    expect(controller.showAllEnvironments).toHaveBeenCalledWith("prj_1");
  });

  it("starts a new project from the header", async () => {
    mountPage();

    button("New project")?.click();
    await nextTick();

    expect(controller.nameDialog.value).toEqual({ kind: "new-project" });
  });

  it("says when the filter matches nothing", () => {
    controller.rows.value = [];
    controller.filter.value = "billing";
    const host = mountPage();

    expect(
      host.querySelector('[data-testid="projects-no-match"]')?.textContent,
    ).toContain("Nothing matches “billing”.");
  });

  it("invites creators to make the first project, with the CLI command", () => {
    controller.projects.value = [];
    controller.rows.value = [];
    const host = mountPage();

    expect(host.textContent).toContain("Create your first project");
    expect(host.textContent).toContain("dopbase project create payments");
    expect(button("New project")).toBeDefined();
  });

  it("tells everyone else to ask for access", () => {
    controller.projects.value = [];
    controller.rows.value = [];
    controller.canCreateProjects.value = false;
    const host = mountPage();

    expect(host.textContent).toContain("No projects shared with you");
    expect(host.textContent).not.toContain("dopbase project create");
    expect(button("New project")).toBeUndefined();
  });
});
