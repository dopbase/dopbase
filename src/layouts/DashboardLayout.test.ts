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
      navOpen: ref(false),
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
    can: () => true,
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
  controller.navOpen.value = false;
  controller.signInAgain.mockReset();
  document.body.innerHTML = "";
});

function mountLayout(): void {
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
}

describe("DashboardLayout", () => {
  it("asks the user to sign in again when the session expires", async () => {
    mountLayout();

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

  it("opens the navigation in a sheet from the top bar menu", async () => {
    mountLayout();
    const sheetSelector = '[role="dialog"][aria-label="Dopbase"]';
    expect(document.body.querySelector(sheetSelector)).toBeNull();

    document.body
      .querySelector<HTMLButtonElement>('[data-testid="nav-open"]')
      ?.click();
    await nextTick();

    const sheet = document.body.querySelector<HTMLElement>(sheetSelector);
    expect(sheet?.textContent).toContain("Projects");
    expect(sheet?.textContent).toContain("admin@example.com");

    sheet
      ?.querySelector<HTMLButtonElement>('button[aria-label="Close"]')
      ?.click();
    await nextTick();
    expect(controller.navOpen.value).toBe(false);
  });
});
