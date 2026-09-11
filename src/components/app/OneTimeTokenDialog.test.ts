import { createApp, h, nextTick } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import OneTimeTokenDialog from "./OneTimeTokenDialog.vue";

let app: ReturnType<typeof createApp> | null = null;

function mountDialog(onAcknowledge = vi.fn()) {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({
    render: () =>
      h(OneTimeTokenDialog, {
        open: true,
        title: "Runner token created",
        token: "dbs_secret",
        name: "production",
        id: "tok_123",
        detail: "This token expires in 30 days.",
        onAcknowledge,
      }),
  });
  app.mount(host);
  return { onAcknowledge };
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  vi.restoreAllMocks();
});

describe("OneTimeTokenDialog", () => {
  it("shows the token, metadata, warning, and acknowledgment action", () => {
    const { onAcknowledge } = mountDialog();

    const input = document.querySelector<HTMLInputElement>(
      '[data-testid="one-time-token"]',
    );
    expect(input?.value).toBe("dbs_secret");
    expect(document.body.textContent).toContain("production");
    expect(document.body.textContent).toContain("tok_123");
    expect(document.body.textContent).toContain("expires in 30 days");

    const acknowledge = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.includes("stored it safely"),
    );
    acknowledge?.click();
    expect(onAcknowledge).toHaveBeenCalledOnce();
  });

  it("selects the token and shows manual copy guidance on failure", async () => {
    Object.defineProperty(navigator, "clipboard", {
      value: undefined,
      configurable: true,
    });
    Object.defineProperty(document, "execCommand", {
      value: vi.fn(() => false),
      configurable: true,
    });
    mountDialog();

    const copy = document.querySelector<HTMLButtonElement>(
      'button[aria-label="Copy"]',
    );
    expect(copy).not.toBeNull();
    copy?.click();
    await nextTick();
    await nextTick();

    const input = document.querySelector<HTMLInputElement>(
      '[data-testid="one-time-token"]',
    );
    expect(document.activeElement).toBe(input);
    expect(input?.selectionStart).toBe(0);
    expect(input?.selectionEnd).toBe("dbs_secret".length);
    expect(document.body.textContent).toContain(
      "Press Ctrl+C or Command+C",
    );
  });
});
