/* eslint-disable vue/one-component-per-file */
import { createApp, h, nextTick } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";

const { controller } = await vi.hoisted(async () => {
  const { computed, reactive, ref } = await import("vue");
  const event = {
    id: "aud_1",
    actorType: "personal_token",
    actorId: "usr_maya",
    actorLabel: "maya@example.com",
    action: "secret.revealed",
    projectId: "prj_1",
    environmentId: "env_1",
    resourceType: "secret",
    resourceId: "STRIPE_WEBHOOK_SECRET",
    ipAddress: "203.0.113.24",
    metadata: {},
    createdAt: "2026-09-26T10:15:30.123Z",
    seq: 4812,
    recordHash: "9f2c00a71e",
  };
  const selected = ref<typeof event | null>(null);
  return {
    controller: {
      items: ref([event]),
      nextCursor: ref(null),
      loading: ref(false),
      loadingMore: ref(false),
      loadError: ref(null),
      hasLoaded: ref(true),
      scopeDescription: ref("Every event on this server."),
      filters: reactive({
        action: "",
        projectId: "",
        environmentId: "",
        actor: "usr_maya",
      }),
      actionOptions: ref([]),
      actorOptions: ref([{ value: "usr_maya", label: "maya@example.com" }]),
      whereOptions: ref([]),
      whereValue: ref(null),
      where: () => "payments/prod",
      actorName: (item: typeof event) => item.actorLabel,
      credential: () => "personal token",
      formatDateTime: () => "Sep 26, 2026",
      formatRelativeTime: () => "2m ago",
      canVerify: ref(true),
      verifying: ref(false),
      verifyError: ref(null),
      chainSummary: computed(() => null),
      chainLine: () => ({
        tone: "ok",
        text: "Chain verified up to #4812 · sha256 9f2c…a71e",
      }),
      selected,
      openEvent: vi.fn((item: typeof event) => (selected.value = item)),
      closeEvent: vi.fn(() => (selected.value = null)),
      eventPayload: () => '{\n  "action": "secret.revealed"\n}',
      origin: () => "203.0.113.24",
      setAction: vi.fn(),
      setActor: vi.fn(),
      setWhere: vi.fn(),
      load: vi.fn(),
      loadMore: vi.fn(),
      verifyChain: vi.fn(),
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

function mount(): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({ render: () => h(AuditPage) });
  app.mount(host);
  return host;
}

afterEach(() => {
  app?.unmount();
  app = null;
  controller.selected.value = null;
  document.body.innerHTML = "";
});

describe("AuditPage", () => {
  it("shows the actor with the credential and where, row by row", () => {
    const host = mount();
    const row = host.querySelector("[data-testid='audit-row']");

    expect(row?.textContent).toContain("2m ago");
    expect(row?.textContent).toContain("secret.revealed");
    expect(row?.textContent).toContain("maya@example.com");
    expect(row?.textContent).toContain("via personal token");
    expect(row?.textContent).toContain("payments/prod");
  });

  it("shows the actor from the address as an applied chip", () => {
    const host = mount();
    const chip = host.querySelector("[data-testid='filter-chip']");

    expect(chip?.textContent).toContain("Actor:");
    expect(chip?.textContent).toContain("maya@example.com");
  });

  it("opens the event in a drawer with its details and the chain line", async () => {
    const host = mount();
    host
      .querySelector("[data-testid='audit-row']")
      ?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await nextTick();

    expect(controller.openEvent).toHaveBeenCalled();
    const drawer = document.body.querySelector("[data-testid='audit-event']");
    expect(drawer?.textContent).toContain("Event #4812");
    expect(drawer?.textContent).toContain("Credential");
    expect(drawer?.textContent).toContain("STRIPE_WEBHOOK_SECRET");
    expect(drawer?.textContent).toContain("203.0.113.24");
    expect(
      document.body.querySelector("[data-testid='audit-event-payload']")
        ?.textContent,
    ).toContain('"action": "secret.revealed"');
    expect(
      document.body.querySelector("[data-testid='audit-chain']")?.textContent,
    ).toContain("Chain verified up to #4812 · sha256 9f2c…a71e");
  });

  it("offers Verify chain to roles that may verify it", async () => {
    const host = mount();
    host
      .querySelector<HTMLButtonElement>("[data-testid='verify-chain']")
      ?.click();
    await nextTick();

    expect(controller.verifyChain).toHaveBeenCalled();
  });
});
