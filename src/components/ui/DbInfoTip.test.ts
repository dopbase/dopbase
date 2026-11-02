import { createApp, h, nextTick } from "vue";
import { afterEach, describe, expect, it } from "vitest";
import DbInfoTip from "./DbInfoTip.vue";

let app: ReturnType<typeof createApp> | null = null;

function mount(): HTMLButtonElement {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp(() =>
    h(DbInfoTip, {
      text: "Live tokens held by AI agents.",
      label: "About AI agent tokens",
    }),
  );
  app.mount(host);
  return host.querySelector<HTMLButtonElement>(
    'button[aria-label="About AI agent tokens"]',
  )!;
}

const tip = () => document.querySelector<HTMLElement>('[role="tooltip"]');

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
});

describe("DbInfoTip", () => {
  it("shows the tip on focus and links it to the button", async () => {
    const button = mount();
    expect(tip()).toBeNull();

    button.dispatchEvent(new FocusEvent("focus"));
    await nextTick();

    expect(tip()?.textContent).toContain("Live tokens held by AI agents.");
    expect(button.getAttribute("aria-describedby")).toBe(tip()?.id);
  });

  it("shows on hover and hides when the pointer leaves", async () => {
    const button = mount();
    const anchor = button.parentElement!;

    anchor.dispatchEvent(new MouseEvent("mouseenter"));
    await nextTick();
    expect(tip()).not.toBeNull();

    anchor.dispatchEvent(new MouseEvent("mouseleave"));
    await nextTick();
    expect(tip()).toBeNull();
  });

  it("opens on a tap and hides on Escape or a tap elsewhere", async () => {
    const button = mount();

    button.click();
    await nextTick();
    expect(tip()).not.toBeNull();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await nextTick();
    expect(tip()).toBeNull();

    button.click();
    await nextTick();
    document.body.dispatchEvent(new Event("pointerdown", { bubbles: true }));
    await nextTick();
    expect(tip()).toBeNull();
  });
});
