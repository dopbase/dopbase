import { createApp, h, nextTick } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";

const { controller } = await vi.hoisted(async () => {
  const { ref } = await import("vue");
  return {
    controller: {
      tokens: ref<import("~/services/tokens.api").RunnerToken[]>([]),
      loading: ref(false),
      loadError: ref(null),
      actionError: ref(null),
      creating: ref(false),
      created: ref(null),
      create: vi.fn(),
      acknowledgeCreated: vi.fn(),
      revoke: vi.fn(),
    },
  };
});

vi.mock("./TokensPanel.controller", () => ({
  useTokensPanelController: () => controller,
}));

import TokensPanel from "./TokensPanel.vue";

let app: ReturnType<typeof createApp> | null = null;

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  vi.useRealTimers();
});

describe("TokensPanel", () => {
  it("marks a token expired while the page stays open", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-09-24T00:00:00Z"));
    controller.tokens.value = [
      {
        id: "tok_1",
        environmentId: "env_1",
        name: "deploy",
        createdAt: "2026-09-23T00:00:00Z",
        expiresAt: "2026-09-24T00:00:01Z",
        lastUsedAt: null,
        revokedAt: null,
      },
    ];
    const host = document.createElement("div");
    document.body.append(host);
    app = createApp({
      render: () => h(TokensPanel, { environmentId: "env_1" }),
    });
    app.mount(host);
    expect(
      document.querySelector('[data-testid="tokens-table"]')?.textContent,
    ).toContain("active");

    await vi.advanceTimersByTimeAsync(1001);
    await nextTick();
    expect(
      document.querySelector('[data-testid="tokens-table"]')?.textContent,
    ).toContain("expired");
  });
});
