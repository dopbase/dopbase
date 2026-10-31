import { createApp, nextTick, type Component } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Environment, Project } from "~/services";

const { narrow } = await vi.hoisted(async () => {
  const { ref } = await import("vue");
  return { narrow: ref(false) };
});

vi.mock("~/composable", async (original) => ({
  ...(await original<typeof import("~/composable")>()),
  useMediaQuery: () => narrow,
}));

import ContextBar from "./ContextBar.vue";

const payments: Project = {
  id: "prj_1",
  name: "payments",
  level: "project_manager",
  createdAt: "",
  updatedAt: "",
};

function environment(
  id: string,
  name: string,
  extra: Partial<Environment> = {},
): Environment {
  return {
    id,
    projectId: "prj_1",
    projectName: "payments",
    name,
    level: "manage",
    createdAt: "",
    updatedAt: "",
    ...extra,
  };
}

const dev = environment("env_1", "dev");
const prod = environment("env_2", "prod", { protected: true, level: "read" });

let app: ReturnType<typeof createApp> | null = null;
const onSwitchEnvironment = vi.fn();
const onSwitchProject = vi.fn();
const onNewEnvironment = vi.fn();

/** Renders a component into the page and keeps it for `afterEach`. */
function mount(component: Component, props: Record<string, unknown>) {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp(component, props);
  app.mount(host);
  return host;
}

function mountBar(): HTMLElement {
  return mount(ContextBar, {
    projectName: "payments",
    projectId: "prj_1",
    projects: [payments, { ...payments, id: "prj_2", name: "search" }],
    environments: [dev, prod],
    environment: dev,
    meter: { label: "Manage", filled: 4, strong: true },
    showRun: true,
    canCreateEnvironment: true,
    onSwitchEnvironment,
    onSwitchProject,
    onNewEnvironment,
  });
}

function sheetRow(name: string): HTMLButtonElement | undefined {
  return Array.from(
    document.querySelectorAll<HTMLButtonElement>(
      '[data-testid="switcher-sheet"] button',
    ),
  ).find((row) => row.textContent?.includes(name));
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  narrow.value = false;
  vi.clearAllMocks();
});

describe("ContextBar", () => {
  it("opens a dropdown on wider screens", async () => {
    const host = mountBar();

    host
      .querySelector<HTMLButtonElement>('[data-testid="environment-switcher"]')!
      .click();
    await nextTick();

    expect(document.querySelector('[role="menu"]')).not.toBeNull();
    expect(document.querySelector('[data-testid="switcher-sheet"]')).toBeNull();
    expect(
      document
        .querySelector('[role="menuitemradio"][aria-checked="true"]')
        ?.textContent?.trim(),
    ).toContain("dev");
  });

  it("offers New environment from the switcher", async () => {
    const host = mountBar();
    host
      .querySelector<HTMLButtonElement>('[data-testid="environment-switcher"]')!
      .click();
    await nextTick();

    document
      .querySelector<HTMLButtonElement>('[data-testid="new-environment"]')!
      .click();

    expect(onNewEnvironment).toHaveBeenCalledOnce();
  });

  it("offers New environment under the list on phones", async () => {
    narrow.value = true;
    const host = mountBar();
    host
      .querySelector<HTMLButtonElement>('[data-testid="environment-switcher"]')!
      .click();
    await nextTick();

    document
      .querySelector<HTMLButtonElement>('[data-testid="switcher-action"]')!
      .click();
    await nextTick();

    expect(onNewEnvironment).toHaveBeenCalledOnce();
  });

  it("opens a bottom sheet on phones and switches environment", async () => {
    narrow.value = true;
    const host = mountBar();
    const trigger = host.querySelector<HTMLButtonElement>(
      '[data-testid="environment-switcher"]',
    )!;

    trigger.click();
    await nextTick();

    expect(document.querySelector('[role="menu"]')).toBeNull();
    expect(trigger.getAttribute("aria-haspopup")).toBe("dialog");
    const row = sheetRow("prod")!;
    expect(row.textContent).toContain("Protected");
    expect(row.textContent).toContain("Read");
    expect(sheetRow("dev")?.getAttribute("aria-current")).toBe("true");

    row.click();
    await nextTick();

    expect(onSwitchEnvironment).toHaveBeenCalledWith(prod);
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
  });

  it("switches project from the sheet on phones", async () => {
    narrow.value = true;
    const host = mountBar();

    host
      .querySelector<HTMLButtonElement>('[data-testid="project-switcher"]')!
      .click();
    await nextTick();
    sheetRow("search")!.click();

    expect(onSwitchProject).toHaveBeenCalledWith(
      expect.objectContaining({ id: "prj_2" }),
    );
  });
});

describe("SwitcherSheet", () => {
  it("filters long lists", async () => {
    const { default: SwitcherSheet } = await import("./SwitcherSheet.vue");
    const items = Array.from({ length: 9 }, (_, index) => ({
      id: `env_${index}`,
      name: index === 4 ? "preview-checkout" : `env-${index}`,
    }));
    mount(SwitcherSheet, {
      open: true,
      title: "Switch environment",
      kind: "environment",
      items,
      currentId: null,
    });
    await nextTick();

    const input = document.querySelector<HTMLInputElement>(
      'input[type="search"]',
    )!;
    input.value = "checkout";
    input.dispatchEvent(new Event("input"));
    await nextTick();

    expect(
      document.querySelectorAll('[data-testid="switcher-sheet"] button'),
    ).toHaveLength(1);
  });
});
