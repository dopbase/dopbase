/* eslint-disable vue/one-component-per-file */
import { createApp, defineComponent, h, nextTick } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import DashboardLayout from "./DashboardLayout.vue";

const { controller } = await vi.hoisted(async () => {
  const { ref } = await import("vue");
  return {
    controller: {
      email: ref("admin@example.com"),
      loggingOut: ref(false),
      sessionExpired: ref(false),
      logout: vi.fn(async () => undefined),
      signInAgain: vi.fn(async () => undefined),
    },
  };
});

vi.mock("./DashboardLayout.controller", () => ({
  useDashboardLayoutController: () => controller,
}));

vi.mock("~/stores/auth.store", () => ({
  useAuthStore: () => ({
    isAdmin: true,
    session: { role: "root" },
  }),
}));

vi.mock("vue-router", () => ({
  useRoute: () => ({ path: "/projects" }),
}));

vi.mock("~/components/app/ReauthModal.vue", () => ({
  default: defineComponent({ render: () => null }),
}));

let app: ReturnType<typeof createApp> | null = null;

afterEach(() => {
  app?.unmount();
  app = null;
  controller.sessionExpired.value = false;
  controller.signInAgain.mockReset();
  document.body.innerHTML = "";
});

describe("DashboardLayout", () => {
  it("asks the user to sign in again when the session expires", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    app = createApp({ render: () => h(DashboardLayout) });
    app.component(
      "RouterLink",
      defineComponent({
        setup(_, { slots }) {
          return () => h("a", slots.default?.());
        },
      }),
    );
    app.mount(host);

    expect(
      document.body.querySelector(
        '[role="dialog"][aria-label="Session expired"]',
      ),
    ).toBeNull();

    controller.sessionExpired.value = true;
    await nextTick();

    const dialog = document.body.querySelector<HTMLElement>(
      '[role="dialog"][aria-label="Session expired"]',
    );
    expect(dialog?.textContent).toContain(
      "Your session has expired. Sign in again to continue.",
    );
    const button = Array.from(dialog?.querySelectorAll("button") ?? []).find(
      (candidate) => candidate.textContent?.trim() === "Sign in again",
    );
    expect(button).toBeDefined();
    button?.click();
    expect(controller.signInAgain).toHaveBeenCalledTimes(1);
  });
});
