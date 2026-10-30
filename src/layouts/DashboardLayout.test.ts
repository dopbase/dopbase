/* eslint-disable vue/one-component-per-file */
import { createApp, defineComponent, h, nextTick } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import DashboardLayout from "./DashboardLayout.vue";

const { controller } = await vi.hoisted(async () => {
  const { ref } = await import("vue");
  return {
    controller: {
      email: ref("admin@example.com"),
      roleName: ref("Admin"),
      sections: ref([
        {
          label: "Workspace",
          items: [
            { name: "projects", label: "Projects" },
            { name: "audit", label: "Audit log" },
          ],
        },
        { label: "Instance", items: [{ name: "status", label: "Status" }] },
      ]),
      activePage: ref("projects"),
      showPersonalTokens: ref(true),
      instanceHost: "secrets.acme.dev",
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

vi.mock("~/components/app/ReauthModal.vue", () => ({
  default: defineComponent({ render: () => null }),
}));

let app: ReturnType<typeof createApp> | null = null;

afterEach(() => {
  app?.unmount();
  app = null;
  controller.sessionExpired.value = false;
  controller.navOpen.value = false;
  controller.showPersonalTokens.value = true;
  controller.logout.mockClear();
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
      props: { to: { type: [String, Object], required: true } },
      setup(props, { slots }) {
        return () =>
          h("a", { "data-to": JSON.stringify(props.to) }, slots.default?.());
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
    expect(sheet?.textContent).toContain("Admin");
    expect(sheet?.textContent).toContain("admin@example.com");

    sheet
      ?.querySelector<HTMLButtonElement>('button[aria-label="Close"]')
      ?.click();
    await nextTick();
    expect(controller.navOpen.value).toBe(false);
  });

  it("groups the sidebar into the sections it is given", () => {
    mountLayout();
    const aside = document.body.querySelector<HTMLElement>("aside");
    const sections = Array.from(
      aside?.querySelectorAll<HTMLElement>("nav section") ?? [],
    );

    expect(
      sections.map((section) => section.getAttribute("aria-label")),
    ).toEqual(["Workspace", "Instance"]);
    expect(sections[0]?.textContent).toContain("Audit log");
    expect(
      aside?.querySelector('[aria-current="page"]')?.textContent,
    ).toContain("Projects");
    expect(aside?.textContent).toContain("secrets.acme.dev");
    expect(aside?.textContent).not.toContain("admin console");
  });

  it("opens the account menu with its links and signs out from it", async () => {
    mountLayout();
    const aside = document.body.querySelector<HTMLElement>("aside");
    const button = aside?.querySelector<HTMLButtonElement>(
      '[data-testid="account-menu-button"]',
    );
    expect(button?.textContent).toContain("admin@example.com");
    expect(aside?.querySelector('[data-testid="account-menu"]')).toBeNull();

    button?.click();
    await nextTick();

    const menu = aside?.querySelector<HTMLElement>(
      '[data-testid="account-menu"]',
    );
    expect(button?.getAttribute("aria-expanded")).toBe("true");
    expect(
      Array.from(menu?.querySelectorAll("a, button") ?? []).map((item) =>
        item.textContent?.trim(),
      ),
    ).toEqual(["Account", "Personal tokens", "Docs", "Sign out"]);

    menu
      ?.querySelector<HTMLButtonElement>('[data-testid="logout-button"]')
      ?.click();
    expect(controller.logout).toHaveBeenCalledTimes(1);
  });

  it("leaves Personal tokens out when the controller hides it", async () => {
    controller.showPersonalTokens.value = false;
    mountLayout();
    const aside = document.body.querySelector<HTMLElement>("aside");

    aside
      ?.querySelector<HTMLButtonElement>('[data-testid="account-menu-button"]')
      ?.click();
    await nextTick();

    expect(
      aside?.querySelector('[data-testid="account-menu"]')?.textContent,
    ).not.toContain("Personal tokens");
  });

  it("closes the account menu on Escape", async () => {
    mountLayout();
    const aside = document.body.querySelector<HTMLElement>("aside");
    aside
      ?.querySelector<HTMLButtonElement>('[data-testid="account-menu-button"]')
      ?.click();
    await nextTick();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await nextTick();

    expect(aside?.querySelector('[data-testid="account-menu"]')).toBeNull();
  });
});
