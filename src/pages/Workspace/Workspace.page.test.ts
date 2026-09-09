/* eslint-disable vue/one-component-per-file */
import { createApp, h, nextTick } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";

const { controller } = await vi.hoisted(async () => {
  const { ref } = await import("vue");
  return {
    controller: {
      projects: ref([
        { id: "prj_1", name: "payments", createdAt: "", updatedAt: "" },
      ]),
      project: ref({
        id: "prj_1",
        name: "payments",
        createdAt: "",
        updatedAt: "",
      }),
      selectedEnvironment: ref({
        id: "env_482731",
        projectId: "prj_1",
        projectName: "payments",
        name: "production",
        createdAt: "",
        updatedAt: "",
      }),
      activeTab: ref("secrets"),
      selectProject: vi.fn(),
      switchTab: vi.fn(),
      createProject: vi.fn(async () => undefined),
    },
  };
});

vi.mock("./Workspace.controller", () => ({
  useWorkspaceController: () => controller,
}));

vi.mock("~/layouts", async () => {
  const { defineComponent, h } = await import("vue");
  return {
    DashboardLayout: defineComponent({
      setup(_, { slots }) {
        return () => h("div", slots.default?.());
      },
    }),
  };
});

vi.mock("./components/ProjectRail.vue", async () => {
  const { defineComponent } = await import("vue");
  return { default: defineComponent({ render: () => null }) };
});
vi.mock("./components/SecretsPanel.vue", async () => {
  const { defineComponent } = await import("vue");
  return { default: defineComponent({ render: () => null }) };
});
vi.mock("./components/TokensPanel.vue", async () => {
  const { defineComponent } = await import("vue");
  return { default: defineComponent({ render: () => null }) };
});
vi.mock("./components/NameDialog.vue", async () => {
  const { defineComponent } = await import("vue");
  return { default: defineComponent({ render: () => null }) };
});

import WorkspacePage from "./Workspace.page.vue";

let app: ReturnType<typeof createApp> | null = null;

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
});

describe("WorkspacePage", () => {
  it("shows and copies the selected environment ID", async () => {
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });
    const host = document.createElement("div");
    document.body.append(host);
    app = createApp({ render: () => h(WorkspacePage) });
    app.mount(host);

    expect(
      host.querySelector('[data-testid="environment-id"]')?.textContent?.trim(),
    ).toBe("env_482731");
    const copyButton = Array.from(host.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Copy ID",
    );
    copyButton?.click();
    await nextTick();
    expect(writeText).toHaveBeenCalledWith("env_482731");
  });
});
