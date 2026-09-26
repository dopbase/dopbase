/* eslint-disable vue/one-component-per-file */
import { createApp, h, nextTick } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";

const { controller } = await vi.hoisted(async () => {
  const { reactive, ref } = await import("vue");
  return {
    controller: {
      items: ref([
        {
          id: "aud_1",
          actorType: "admin",
          actorId: "usr_1",
          actorLabel: "admin@example.com",
          action: "secret.revealed",
          projectId: null,
          environmentId: null,
          resourceType: null,
          resourceId: null,
          metadata: {},
          createdAt: "2026-09-26T10:15:30.123Z",
        },
      ]),
      nextCursor: ref(null),
      loading: ref(false),
      loadingMore: ref(false),
      loadError: ref(null),
      hasLoaded: ref(true),
      filters: reactive({
        action: "",
        projectId: "",
        environmentId: "",
        actor: "",
      }),
      projectOptions: ref([]),
      environmentOptions: ref([]),
      projectName: () => "—",
      environmentName: () => "—",
      formatDateTime: () => "Sep 26, 2026",
      formatRelativeTime: () => "1h ago",
      load: vi.fn(),
      loadMore: vi.fn(),
    },
  };
});

vi.mock("./Audit.controller", () => ({
  useAuditController: () => controller,
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

import AuditPage from "./Audit.page.vue";

let app: ReturnType<typeof createApp> | null = null;

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
});

describe("AuditPage", () => {
  it("shows the raw timestamp when an event is expanded", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    app = createApp({ render: () => h(AuditPage) });
    app.mount(host);

    expect(host.textContent).not.toContain("2026-09-26T10:15:30.123Z");
    host
      .querySelector("[data-testid='audit-table'] tbody tr")
      ?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await nextTick();

    expect(host.textContent).toContain("timestamp:");
    expect(host.textContent).toContain("2026-09-26T10:15:30.123Z");
  });
});
