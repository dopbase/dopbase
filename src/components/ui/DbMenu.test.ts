import { createApp, h, nextTick, type VNode } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import DbMenu from "./DbMenu.vue";
import DbMenuItem from "./DbMenuItem.vue";

let app: ReturnType<typeof createApp> | null = null;

/** Mounts a menu labelled `label` around the given items. */
function mount(label: string, children: () => VNode[]): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp(() => h(DbMenu, { label }, { default: children }));
  app.mount(host);
  return host;
}

function mountMenu(onRename = vi.fn(), onDelete = vi.fn()) {
  const host = mount("Actions for API_KEY", () => [
    h(DbMenuItem, { onSelect: onRename }, () => "Rename"),
    h(DbMenuItem, { tone: "danger", onSelect: onDelete }, () => "Delete"),
  ]);
  const trigger = host.querySelector<HTMLButtonElement>(
    'button[aria-label="Actions for API_KEY"]',
  )!;
  return { trigger, onRename, onDelete };
}

const menu = () => document.querySelector<HTMLElement>('[role="menu"]');
const items = () =>
  Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"]'));

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
});

describe("DbMenu", () => {
  it("opens from its trigger and focuses the first item", async () => {
    const { trigger } = mountMenu();
    expect(menu()).toBeNull();

    trigger.click();
    await nextTick();
    await nextTick();

    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(menu()).not.toBeNull();
    expect(document.activeElement).toBe(items()[0]);
  });

  it("reports the chosen item and closes", async () => {
    const { trigger, onDelete } = mountMenu();
    trigger.click();
    await nextTick();

    items()[1]!.click();
    await nextTick();

    expect(onDelete).toHaveBeenCalledOnce();
    expect(menu()).toBeNull();
  });

  it("moves between items with the arrow keys", async () => {
    const { trigger } = mountMenu();
    trigger.click();
    await nextTick();
    await nextTick();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
    expect(document.activeElement).toBe(items()[1]);
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
    expect(document.activeElement).toBe(items()[0]);
  });

  it("closes on Escape and returns focus to the trigger", async () => {
    const { trigger } = mountMenu();
    trigger.click();
    await nextTick();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await nextTick();

    expect(menu()).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it("closes on an outside click", async () => {
    const { trigger, onRename } = mountMenu();
    trigger.click();
    await nextTick();

    document.body.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    await nextTick();

    expect(menu()).toBeNull();
    expect(onRename).not.toHaveBeenCalled();
  });

  it("styles destructive items as danger", async () => {
    const { trigger } = mountMenu();
    trigger.click();
    await nextTick();

    expect(items()[1]!.className).toContain("text-crit");
    expect(items()[0]!.className).not.toContain("text-crit");
  });

  it("marks the current choice in a pick-one menu", async () => {
    const host = mount("Environment", () => [
      h(DbMenuItem, { selected: false }, () => "dev"),
      h(DbMenuItem, { selected: true }, () => "prod"),
    ]);
    host.querySelector<HTMLButtonElement>("button")!.click();
    await nextTick();
    await nextTick();

    const choices = Array.from(
      document.querySelectorAll<HTMLElement>('[role="menuitemradio"]'),
    );
    expect(choices.map((item) => item.getAttribute("aria-checked"))).toEqual([
      "false",
      "true",
    ]);
    expect(choices[1]!.className).toContain("bg-accent-soft");
    // It opens on the current choice.
    expect(document.activeElement).toBe(choices[1]);
  });
});
