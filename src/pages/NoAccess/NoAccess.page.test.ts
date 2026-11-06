/* eslint-disable vue/one-component-per-file */
import { createApp, defineComponent, h, nextTick, ref } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";

const state = vi.hoisted(() => ({
  description: "",
  docsUrl: null as string | null,
}));

vi.mock("./NoAccess.controller", () => ({
  useNoAccessController: () => ({
    title: ref("Single sign-on is turned off"),
    description: ref(state.description),
    docsUrl: ref(state.docsUrl),
  }),
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

import NoAccessPage from "./NoAccess.page.vue";

let app: ReturnType<typeof createApp> | null = null;

async function mountPage(): Promise<HTMLElement> {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({ render: () => h(NoAccessPage) });
  app.component(
    "RouterLink",
    defineComponent({
      setup(_, { slots }) {
        return () => h("a", { href: "#" }, slots.default?.());
      },
    }),
  );
  app.mount(host);
  await nextTick();
  return host;
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
});

describe("NoAccess page", () => {
  it("links the docs when the server has the feature off", async () => {
    state.description =
      "This feature is disabled on this server. Your server administrator can enable it by following the";
    state.docsUrl = "https://docs.dopbase.com";

    const host = await mountPage();
    const link = host.querySelector<HTMLAnchorElement>(
      '[data-testid="no-access-docs"]',
    );

    expect(link?.href).toBe("https://docs.dopbase.com/");
    expect(link?.target).toBe("_blank");
    expect(host.textContent?.replace(/\s+/g, " ")).toContain(
      "This feature is disabled on this server. Your server administrator can enable it by following the docs.",
    );
  });

  it("shows the plain description when the role is what's missing", async () => {
    state.description =
      "Your role doesn't include this page. Ask root if you need it.";
    state.docsUrl = null;

    const host = await mountPage();

    expect(host.querySelector('[data-testid="no-access-docs"]')).toBeNull();
    expect(host.textContent).toContain(
      "Your role doesn't include this page. Ask root if you need it.",
    );
  });
});
