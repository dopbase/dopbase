import { createApp, h, nextTick } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { TokenRow } from "./TokensPanel.controller";

const { controller } = await vi.hoisted(async () => {
  const { ref } = await import("vue");
  return {
    controller: {
      rows: ref<TokenRow[] | null>([]),
      filter: ref(""),
      copiedId: ref<string | null>(null),
      copyName: vi.fn(),
      loading: ref(false),
      loadError: ref(null),
      actionError: ref(null),
      creating: ref(false),
      created: ref(null),
      create: vi.fn(async () => undefined),
      acknowledgeCreated: vi.fn(),
      revoke: vi.fn(),
    },
  };
});

vi.mock("./TokensPanel.controller", () => ({
  useTokensPanelController: () => controller,
}));

import TokensPanel from "./TokensPanel.vue";

const token = {
  id: "tok_1",
  environmentId: "env_1",
  name: "deploy-prod",
  createdAt: "2026-09-23T00:00:00Z",
  expiresAt: "2026-10-03T00:00:00Z",
  lastUsedAt: null,
  revokedAt: null,
};

let app: ReturnType<typeof createApp> | null = null;
let onActivity = vi.fn();

function mountPanel(): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  onActivity = vi.fn();
  app = createApp({
    render: () =>
      h(TokensPanel, {
        environmentId: "env_1",
        environmentName: "prod",
        projectName: "payments",
        protected: true,
        onActivity,
      }),
  });
  app.mount(host);
  return host;
}

function button(text: string): HTMLButtonElement | undefined {
  return Array.from(document.querySelectorAll("button")).find(
    (candidate) => candidate.textContent?.trim() === text,
  );
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  controller.rows.value = [];
  controller.created.value = null;
  vi.clearAllMocks();
});

describe("TokensPanel", () => {
  it("lists tokens with what they can do and when they expire", () => {
    controller.rows.value = [
      {
        token,
        lastUsed: "Never",
        expires: "Expires in 3 days",
        expiresTone: "warn",
        active: true,
      },
    ];
    const host = mountPanel();
    const table = host.querySelector('[data-testid="tokens-table"]');

    expect(table?.textContent).toContain("deploy-prod");
    expect(table?.textContent).toContain("Read");
    expect(table?.textContent).toContain("Expires in 3 days");
    expect(
      Array.from(table?.querySelectorAll("td") ?? []).find((cell) =>
        cell.textContent?.includes("Expires in 3 days"),
      )?.className,
    ).toContain("text-warn");
  });

  it("offers copy, activity and revoke for an active token", async () => {
    controller.rows.value = [
      {
        token,
        lastUsed: "Never",
        expires: "Jan 25, 2027",
        expiresTone: "default",
        active: true,
      },
    ];
    mountPanel();

    document
      .querySelector<HTMLButtonElement>(
        'button[aria-label="Actions for deploy-prod"]',
      )!
      .click();
    await nextTick();
    const items = Array.from(
      document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'),
    );
    expect(items.map((item) => item.textContent?.trim())).toEqual([
      "Copy name",
      "Activity",
      "Revoke token",
    ]);

    items[1]!.click();
    expect(onActivity).toHaveBeenCalledOnce();
  });

  it("creates a token with the chosen expiry and explains what it can do", async () => {
    mountPanel();
    button("New runner token")?.click();
    await nextTick();

    const name = document.querySelector<HTMLInputElement>(
      'input[name="tokenName"]',
    )!;
    name.value = "deploy-prod";
    name.dispatchEvent(new Event("input"));
    await nextTick();
    expect(
      document.querySelector('[data-testid="token-summary"]')?.textContent,
    ).toContain("deploy-prod can read every value in payments / prod until");
    const environment = document.querySelector<HTMLInputElement>(
      'input[name="tokenEnvironment"]',
    )!;
    expect(environment.value).toBe("prod (protected)");
    expect(environment.readOnly).toBe(true);

    document
      .querySelector<HTMLButtonElement>('[role="radio"][data-value="30d"]')!
      .click();
    document
      .querySelector<HTMLFormElement>("#new-runner-token")!
      .requestSubmit();
    await nextTick();

    expect(controller.create).toHaveBeenCalledWith("deploy-prod", "30d");
  });

  it("refuses a name an environment couldn't have", async () => {
    mountPanel();
    button("New runner token")?.click();
    await nextTick();

    const name = document.querySelector<HTMLInputElement>(
      'input[name="tokenName"]',
    )!;
    name.value = "Deploy prod";
    name.dispatchEvent(new Event("input"));
    document
      .querySelector<HTMLFormElement>("#new-runner-token")!
      .requestSubmit();
    await nextTick();

    expect(controller.create).not.toHaveBeenCalled();
    expect(document.body.textContent).toContain(
      "Use lowercase letters, numbers and single hyphens",
    );
  });

  it("puts the whole token in the usage lines", async () => {
    mountPanel();
    const plaintext = "dbs_0123456789abcdefghijklmnopqrstuvwxyz";
    controller.created.value = { token, plaintextToken: plaintext } as never;
    await nextTick();

    expect(
      document.querySelector('[data-testid="token-usage"]')?.textContent,
    ).toContain(`export DOPBASE_TOKEN=${plaintext}`);
  });
});
