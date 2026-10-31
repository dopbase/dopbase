/* eslint-disable vue/one-component-per-file */
import { createApp, h } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import DbCopyButton from "./DbCopyButton.vue";

let app: ReturnType<typeof createApp> | null = null;

function mountButton(
  clipboard: { writeText: (value: string) => Promise<void> } | undefined,
  onCopied = vi.fn(),
  onCopyError = vi.fn(),
) {
  Object.defineProperty(navigator, "clipboard", {
    value: clipboard,
    configurable: true,
  });
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({
    render: () =>
      h(DbCopyButton, {
        value: "dbs_secret",
        onCopied,
        onCopyError,
      }),
  });
  app.mount(host);
  return { host, onCopied, onCopyError };
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  vi.restoreAllMocks();
});

describe("DbCopyButton", () => {
  it("uses the Clipboard API and confirms success", async () => {
    const writeText = vi.fn(async () => undefined);
    const { host, onCopied } = mountButton({ writeText });

    host.querySelector("button")?.click();

    await vi.waitFor(() => expect(host.textContent).toContain("Copied"));
    expect(writeText).toHaveBeenCalledWith("dbs_secret");
    expect(onCopied).toHaveBeenCalledOnce();
  });

  it("falls back to selection copying when the Clipboard API is unavailable", async () => {
    const execCommand = vi.fn(() => true);
    Object.defineProperty(document, "execCommand", {
      value: execCommand,
      configurable: true,
    });
    const { host, onCopied } = mountButton(undefined);

    host.querySelector("button")?.click();

    await vi.waitFor(() => expect(onCopied).toHaveBeenCalledOnce());
    expect(execCommand).toHaveBeenCalledWith("copy");
  });

  it("reports failure when both copy methods fail", async () => {
    Object.defineProperty(document, "execCommand", {
      value: vi.fn(() => false),
      configurable: true,
    });
    const writeText = vi.fn(async () => {
      throw new Error("denied");
    });
    const { host, onCopyError } = mountButton({ writeText });

    host.querySelector("button")?.click();

    await vi.waitFor(() => expect(onCopyError).toHaveBeenCalledOnce());
  });

  it("draws an icon button that keeps its label for screen readers", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    app = createApp({
      render: () =>
        h(DbCopyButton, {
          value: "dopbase run",
          label: "Copy command",
          iconOnly: true,
        }),
    });
    app.mount(host);

    const button = host.querySelector("button")!;
    expect(button.getAttribute("aria-label")).toBe("Copy command");
    expect(button.getAttribute("title")).toBe("Copy command");
    expect(button.textContent?.trim()).toBe("");
  });
});
