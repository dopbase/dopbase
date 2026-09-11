import { createApp, h, nextTick } from "vue";
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
    await nextTick();
    await nextTick();

    expect(writeText).toHaveBeenCalledWith("dbs_secret");
    expect(onCopied).toHaveBeenCalledOnce();
    expect(host.textContent).toContain("Copied");
  });

  it("falls back to selection copying when the Clipboard API is unavailable", async () => {
    const execCommand = vi.fn(() => true);
    Object.defineProperty(document, "execCommand", {
      value: execCommand,
      configurable: true,
    });
    const { host, onCopied } = mountButton(undefined);

    host.querySelector("button")?.click();
    await nextTick();

    expect(execCommand).toHaveBeenCalledWith("copy");
    expect(onCopied).toHaveBeenCalledOnce();
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
    await nextTick();
    await nextTick();

    expect(onCopyError).toHaveBeenCalledOnce();
  });
});
