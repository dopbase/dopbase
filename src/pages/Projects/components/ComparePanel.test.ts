import { createApp, defineComponent, h, nextTick } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as compareApi from "~/services/compare.api";
import type { Environment } from "~/services";

vi.mock("~/services/compare.api", async (original) => ({
  ...(await original<typeof compareApi>()),
  compareEnvironments: vi.fn(),
  copySecret: vi.fn(),
}));

import ComparePanel from "./ComparePanel.vue";

function environment(id: string, name: string): Environment {
  return {
    id,
    projectId: "prj_pay",
    projectName: "payments",
    name,
    level: "manage",
    createdAt: "",
    updatedAt: "",
  };
}

const [prod, staging, dev, qa] = [
  environment("env_prod", "prod"),
  environment("env_staging", "staging"),
  environment("env_dev", "dev"),
  environment("env_qa", "qa"),
];
const column = (env: Environment) => ({
  id: env.id,
  name: env.name,
  protected: false,
  level: "manage" as const,
  valuesCompared: true,
});

let app: ReturnType<typeof createApp> | null = null;

/** Renders a link's text and keeps where it points, without a router. */
const RouterLinkStub = defineComponent({
  props: { to: { type: Object, required: true } },
  setup(props, { slots }) {
    return () =>
      h("a", { "data-to": JSON.stringify(props.to) }, slots.default?.());
  },
});
const onNewEnvironment = vi.fn();

function mountPanel(environments: Environment[]): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp(() =>
    h(ComparePanel, {
      environment: prod,
      environments,
      canCreateEnvironment: true,
      onNewEnvironment,
    }),
  );
  app.component("RouterLink", RouterLinkStub);
  app.mount(host);
  return host;
}

beforeEach(() => {
  localStorage.clear();
  vi.mocked(compareApi.compareEnvironments).mockImplementation(
    async (_base, ids) => ({
      base: column(prod),
      environments: ids.map((id) =>
        column([staging, dev, qa].find((env) => env.id === id)!),
      ),
      keys: [
        {
          key: "FEATURE_FLAGS",
          base: null,
          others: ids.map(() => ({ version: 1, sameAsBase: null })),
        },
      ],
    }),
  );
});

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  vi.clearAllMocks();
});

const add = () => document.querySelector('[data-testid="compare-add"]');

describe("ComparePanel", () => {
  it("starts with one environment and hides Add environment at three", async () => {
    mountPanel([prod, staging, dev, qa, environment("env_uat", "uat")]);
    await vi.waitFor(() =>
      expect(
        document.querySelectorAll('[data-testid="compare-chip"]'),
      ).toHaveLength(1),
    );
    expect(add()).not.toBeNull();

    for (const name of ["dev", "qa"]) {
      (add() as HTMLButtonElement).click();
      await nextTick();
      Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"]'))
        .find((item) => item.textContent?.includes(name))!
        .click();
      await nextTick();
    }

    expect(
      document.querySelectorAll('[data-testid="compare-chip"]'),
    ).toHaveLength(3);
    expect(add()).toBeNull();
    expect(
      document.querySelector('[data-testid="compare-limit"]')?.textContent,
    ).toContain("3 at a time");
  });

  it("offers New environment when there's nothing to compare", async () => {
    const host = mountPanel([prod]);
    await nextTick();

    expect(host.textContent).toContain("Nothing to compare yet");
    Array.from(host.querySelectorAll("button"))
      .find((button) => button.textContent?.includes("New environment"))!
      .click();
    expect(onNewEnvironment).toHaveBeenCalledOnce();
    expect(compareApi.compareEnvironments).toHaveBeenCalledWith(
      "env_prod",
      [],
      expect.any(AbortSignal),
    );
  });

  it("opens the fill dialog from a Not set cell", async () => {
    mountPanel([prod, staging, dev]);
    const cell = () =>
      document.querySelector<HTMLButtonElement>(
        'button[aria-label="Set FEATURE_FLAGS in prod"]',
      );
    await vi.waitFor(() => expect(cell()).not.toBeNull());

    cell()!.click();
    await nextTick();

    const dialog = document.querySelector('[data-testid="fill-dialog"]');
    expect(dialog?.textContent).toContain("Match staging");
    expect(dialog?.textContent).toContain("Use its own value");
  });

  it("says above the table that set values stay in Secrets", async () => {
    mountPanel([prod, staging]);
    const policy = () =>
      document.querySelector('[data-testid="compare-policy"]');
    await vi.waitFor(() => expect(policy()).not.toBeNull());

    expect(policy()?.textContent).toContain(
      "Compare doesn't show or change values that are already set.",
    );
    const link = policy()!.querySelector("a")!;
    expect(link.textContent).toContain("Open prod's Secrets");
    expect(JSON.parse(link.dataset.to!)).toEqual({
      name: "environment",
      params: { projectRef: "payments", environmentName: "prod" },
    });
  });
});
