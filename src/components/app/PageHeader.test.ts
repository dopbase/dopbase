import { createApp, h } from "vue";
import { createMemoryHistory, createRouter } from "vue-router";
import { afterEach, describe, expect, it } from "vitest";
import PageHeader from "./PageHeader.vue";

let app: ReturnType<typeof createApp> | null = null;

async function mount(props: Record<string, unknown>): Promise<HTMLElement> {
  const host = document.createElement("div");
  document.body.append(host);
  const empty = { render: () => null };
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/agents", name: "agents", component: empty },
      { path: "/agents/:agentId", name: "agent", component: empty },
    ],
  });
  app = createApp({
    render: () => h(PageHeader, { title: "Give access", ...props }),
  });
  app.use(router);
  await router.push("/agents");
  await router.isReady();
  app.mount(host);
  return host;
}

function crumbs(host: HTMLElement) {
  return [
    ...host.querySelectorAll<HTMLAnchorElement>('[data-testid="breadcrumb"] a'),
  ].map((link) => [link.textContent?.trim(), link.getAttribute("href")]);
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
});

describe("PageHeader", () => {
  it("links every page in the trail and ends on the title", async () => {
    const host = await mount({
      trail: [
        { label: "AI agents", to: { name: "agents" } },
        {
          label: "review-bot",
          to: { name: "agent", params: { agentId: "usr_bot" } },
        },
      ],
    });
    expect(crumbs(host)).toEqual([
      ["AI agents", "/agents"],
      ["review-bot", "/agents/usr_bot"],
    ]);
    const nav = host.querySelector('[data-testid="breadcrumb"]');
    expect(nav?.getAttribute("aria-label")).toBe("Breadcrumb");
    expect(nav?.textContent?.replace(/\s+/g, " ").trim()).toBe(
      "AI agents / review-bot / Give access",
    );
  });

  it("shows a lone parent as a one-link trail", async () => {
    const host = await mount({
      parent: { label: "AI agents", to: { name: "agents" } },
    });
    expect(crumbs(host)).toEqual([["AI agents", "/agents"]]);
  });

  it("leaves the breadcrumb out with no parent", async () => {
    const host = await mount({});
    expect(host.querySelector('[data-testid="breadcrumb"]')).toBeNull();
  });
});
