import { createApp, h } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import EnvironmentId from "./EnvironmentId.vue";

let app: ReturnType<typeof createApp> | null = null;

function mountId(
  clipboard: { writeText: (value: string) => Promise<void> } | undefined,
): HTMLElement {
  Object.defineProperty(navigator, "clipboard", {
    value: clipboard,
    configurable: true,
  });
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({
    render: () => h(EnvironmentId, { id: "env_918204", variant: "chip" }),
  });
  app.mount(host);
  return host;
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  window.getSelection()?.removeAllRanges();
  vi.restoreAllMocks();
});

describe("EnvironmentId", () => {
  it("shows the ID and copies it when clicked", async () => {
    const writeText = vi.fn(async () => undefined);
    const host = mountId({ writeText });
    const button = host.querySelector("button")!;

    expect(button.textContent).toContain("env_918204");
    expect(button.getAttribute("aria-label")).toBe(
      "Copy environment ID env_918204",
    );

    button.click();

    await vi.waitFor(() => expect(button.textContent).toContain("Copied"));
    expect(writeText).toHaveBeenCalledWith("env_918204");
  });

  it("selects the ID for a manual copy when the clipboard is blocked", async () => {
    Object.defineProperty(document, "execCommand", {
      value: vi.fn(() => false),
      configurable: true,
    });
    const host = mountId({
      writeText: vi.fn(async () => {
        throw new Error("denied");
      }),
    });
    const button = host.querySelector("button")!;

    button.click();

    await vi.waitFor(() =>
      expect(button.textContent).toContain("Couldn’t copy"),
    );
    expect(window.getSelection()?.toString()).toBe("env_918204");
  });
});
