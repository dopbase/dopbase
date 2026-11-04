import { createApp, h, nextTick, ref } from "vue";
import { createMemoryHistory, createRouter } from "vue-router";
import { afterEach, describe, expect, it } from "vitest";
import DbAvatar from "./DbAvatar.vue";
import DbFilterMenu from "./DbFilterMenu.vue";
import DbTabNav from "./DbTabNav.vue";

let app: ReturnType<typeof createApp> | null = null;

async function mount(
  render: () => ReturnType<typeof h>,
  withRouter = false,
): Promise<HTMLElement> {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({ render });
  if (withRouter) {
    const empty = { render: () => null };
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: "/users/:userId", name: "user", component: empty },
        {
          path: "/users/:userId/access",
          name: "user-access",
          component: empty,
        },
      ],
    });
    app.use(router);
    await router.push("/users/usr_maya");
    await router.isReady();
  }
  app.mount(host);
  return host;
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
});

describe("DbAvatar", () => {
  it("shows the first letter of an email in capitals", async () => {
    const host = await mount(() => h(DbAvatar, { name: "maya@example.com" }));
    expect(host.textContent?.trim()).toBe("M");
    expect(host.firstElementChild?.getAttribute("aria-hidden")).toBe("true");
  });

  it("falls back to a question mark for an empty name", async () => {
    const host = await mount(() => h(DbAvatar, { name: "   " }));
    expect(host.textContent?.trim()).toBe("?");
  });

  it("dims a disabled account", async () => {
    const host = await mount(() =>
      h(DbAvatar, { name: "sam@example.com", muted: true, size: "sm" }),
    );
    expect(host.firstElementChild?.className).toContain("opacity-50");
    expect(host.firstElementChild?.className).toContain("h-6");
  });
});

describe("DbFilterMenu", () => {
  const options = [
    { value: "rol_admin", label: "Admin" },
    { value: "rol_member", label: "Member" },
  ];

  it("offers the choices from an empty chip and reports the pick", async () => {
    const value = ref<string | null>(null);
    const host = await mount(() =>
      h(DbFilterMenu, {
        label: "Role",
        options,
        modelValue: value.value,
        "onUpdate:modelValue": (next: string | null) => (value.value = next),
      }),
    );
    const trigger = host.querySelector<HTMLButtonElement>(
      'button[aria-label="Filter by Role"]',
    )!;
    expect(trigger.textContent).toContain("Role");
    trigger.click();
    await nextTick();
    await nextTick();
    const items = Array.from(
      document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
    );
    expect(items.map((item) => item.textContent?.trim())).toEqual([
      "Admin",
      "Member",
    ]);
    items[1]!.click();
    await nextTick();
    expect(value.value).toBe("rol_member");
  });

  it("reads Role: Member once set, and × clears it", async () => {
    const value = ref<string | null>("rol_member");
    const host = await mount(() =>
      h(DbFilterMenu, {
        label: "Role",
        options,
        modelValue: value.value,
        "onUpdate:modelValue": (next: string | null) => (value.value = next),
      }),
    );
    const chip = host.querySelector('[data-testid="filter-chip"]')!;
    expect(chip.textContent?.replace(/\s+/g, " ").trim()).toBe("Role: Member");
    host
      .querySelector<HTMLButtonElement>(
        'button[aria-label="Clear Role filter"]',
      )!
      .click();
    await nextTick();
    expect(value.value).toBeNull();
    expect(host.querySelector('[data-testid="filter-chip"]')).toBeNull();
  });

  it("shows the empty chip when the value matches no option", async () => {
    const host = await mount(() =>
      h(DbFilterMenu, { label: "Role", options, modelValue: "rol_gone" }),
    );
    expect(host.querySelector('[data-testid="filter-chip"]')).toBeNull();
    expect(
      host.querySelector('button[aria-label="Filter by Role"]'),
    ).not.toBeNull();
  });
});

describe("DbTabNav", () => {
  const tabs = [
    {
      key: "overview",
      label: "Overview",
      to: { name: "user", params: { userId: "usr_maya" } },
    },
    {
      key: "access",
      label: "Access",
      to: { name: "user-access", params: { userId: "usr_maya" } },
    },
  ];

  it("renders each tab as a link with its own address", async () => {
    const host = await mount(
      () =>
        h(DbTabNav, { tabs, active: "overview", label: "Account sections" }),
      true,
    );
    const nav = host.querySelector("nav")!;
    expect(nav.getAttribute("aria-label")).toBe("Account sections");
    const links = Array.from(nav.querySelectorAll("a"));
    expect(links.map((link) => link.getAttribute("href"))).toEqual([
      "/users/usr_maya",
      "/users/usr_maya/access",
    ]);
  });

  it("marks only the active tab as the current page", async () => {
    const host = await mount(
      () => h(DbTabNav, { tabs, active: "access", label: "Account sections" }),
      true,
    );
    const links = Array.from(host.querySelectorAll("a"));
    expect(links.map((link) => link.getAttribute("aria-current"))).toEqual([
      null,
      "page",
    ]);
  });
});
