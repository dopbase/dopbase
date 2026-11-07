import { createApp, h, nextTick } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";

const { controller } = await vi.hoisted(async () => {
  const { ref } = await import("vue");
  const versions = [
    {
      version: 3,
      changeType: "rolled_back",
      sourceVersion: 1,
      actorType: "admin",
      actorLabel: "admin@dopbase.com",
      createdAt: "2026-10-23T00:00:00Z",
      current: true,
      restorable: true,
    },
    {
      version: 2,
      changeType: "updated",
      sourceVersion: null,
      actorType: "admin",
      actorLabel: "admin@dopbase.com",
      createdAt: "2026-10-22T00:00:00Z",
      current: false,
      restorable: true,
    },
    {
      version: 1,
      changeType: "legacy",
      sourceVersion: null,
      actorType: "system",
      actorLabel: null,
      createdAt: "2026-09-01T00:00:00Z",
      current: false,
      restorable: false,
    },
  ];
  return {
    controller: {
      versions: ref(versions),
      current: ref(versions[0]),
      retainedVersions: ref(20),
      nextBefore: ref<number | null>(null),
      loading: ref(false),
      loadingMore: ref(false),
      loadError: ref(null),
      actionError: ref(null),
      notice: ref<string | null>(null),
      revealedVersion: ref<number | null>(null),
      revealedValue: ref<string | null>(null),
      revealCountdown: ref("0:30"),
      rollbackTarget: ref<(typeof versions)[number] | null>(null),
      rollingBack: ref(false),
      rollbackError: ref(null),
      load: vi.fn(),
      loadMore: vi.fn(),
      reveal: vi.fn(),
      hideRevealed: vi.fn(),
      askRollback: vi.fn(),
      cancelRollback: vi.fn(),
      confirmRollback: vi.fn(),
    },
  };
});

vi.mock("./SecretHistory.controller", async (original) => ({
  ...(await original<typeof import("./SecretHistory.controller")>()),
  useSecretHistoryController: () => controller,
}));

import SecretHistoryDialog from "./SecretHistoryDialog.vue";

let app: ReturnType<typeof createApp> | null = null;
const onRolledBack = vi.fn();

async function mountDialog() {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({
    render: () =>
      h(SecretHistoryDialog, {
        environmentId: "env_1",
        secretKey: "API_KEY",
        onRolledBack,
      }),
  });
  app.mount(host);
  await nextTick();
}

function button(label: string): HTMLButtonElement | undefined {
  return (
    document.querySelector<HTMLButtonElement>(
      `button[aria-label="${label}"]`,
    ) ?? undefined
  );
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  controller.rollbackTarget.value = null;
  controller.revealedVersion.value = null;
  controller.revealedValue.value = null;
  vi.clearAllMocks();
});

describe("SecretHistoryDialog", () => {
  it("lists versions with their change, status, and actions only where they apply", async () => {
    await mountDialog();
    const text = document.body.textContent ?? "";
    expect(text).toContain("Secrets history");
    expect(text).toContain("Current version v3.");
    expect(text).toContain("The last 20 versions keep their value");

    const current = document.querySelector('[data-testid="history-row-3"]')!;
    expect(current.textContent).toContain("current");
    expect(current.textContent).toContain("Rolled back from v1");
    const expired = document.querySelector('[data-testid="history-row-1"]')!;
    expect(expired.textContent).toContain("expired");
    expect(expired.textContent).toContain("Before history");

    expect(button("Roll back to v2")).toBeDefined();
    expect(button("Reveal v2")).toBeDefined();
    expect(button("Roll back to v3")).toBeUndefined();
    expect(button("Roll back to v1")).toBeUndefined();
    expect(button("Reveal v1")).toBeUndefined();
  });

  it("reveals an earlier value in its own row", async () => {
    await mountDialog();
    button("Reveal v2")!.click();
    expect(controller.reveal).toHaveBeenCalledWith(2);

    controller.revealedVersion.value = 2;
    controller.revealedValue.value = "old-value";
    await nextTick();
    expect(document.body.textContent).toContain("old-value");
    expect(document.body.textContent).toContain("hides in 0:30");
    expect(button("Hide v2")).toBeDefined();
  });

  it("asks before rolling back and reports the result", async () => {
    await mountDialog();
    button("Roll back to v2")!.click();
    expect(controller.askRollback).toHaveBeenCalledWith(
      expect.objectContaining({ version: 2 }),
    );

    controller.rollbackTarget.value = controller.versions.value[1]!;
    await nextTick();
    const confirm = document.querySelector('[data-testid="rollback-confirm"]')!;
    expect(confirm.textContent).toContain(
      "Save the value from v2 as a new version, v4?",
    );
    expect(confirm.textContent).toContain("Nothing in the history changes.");

    const result = { key: "API_KEY", version: 4, sourceVersion: 2 };
    controller.confirmRollback.mockResolvedValueOnce(result);
    document
      .querySelector<HTMLButtonElement>(
        '[data-testid="rollback-confirm-button"]',
      )!
      .click();
    await vi.waitFor(() => expect(onRolledBack).toHaveBeenCalledWith(result));
  });
});
