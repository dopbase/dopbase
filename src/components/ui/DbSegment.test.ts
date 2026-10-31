import { createApp, h, nextTick, ref } from "vue";
import { afterEach, describe, expect, it } from "vitest";
import DbSegment from "./DbSegment.vue";
import DbSearchField from "./DbSearchField.vue";
import DbLevelBadge from "./DbLevelBadge.vue";

let app: ReturnType<typeof createApp> | null = null;

function mount(render: () => ReturnType<typeof h>): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({ render });
  app.mount(host);
  return host;
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
});

describe("DbSegment", () => {
  const options = [
    { value: "table", label: "Table" },
    { value: "editor", label: ".env" },
  ];

  it("marks the chosen option and switches on click", async () => {
    const view = ref("table");
    const host = mount(() =>
      h(DbSegment, {
        modelValue: view.value,
        options,
        label: "Show secrets as",
        "onUpdate:modelValue": (value: string) => (view.value = value),
      }),
    );
    const [table, env] = Array.from(
      host.querySelectorAll<HTMLButtonElement>('[role="radio"]'),
    );
    expect(
      host.querySelector('[role="radiogroup"]')?.getAttribute("aria-label"),
    ).toBe("Show secrets as");
    expect(table?.getAttribute("aria-checked")).toBe("true");

    env?.click();
    await nextTick();

    expect(view.value).toBe("editor");
    expect(env?.getAttribute("aria-checked")).toBe("true");
  });

  it("moves the choice with the arrow keys", async () => {
    const view = ref("editor");
    const host = mount(() =>
      h(DbSegment, {
        modelValue: view.value,
        options,
        label: "View",
        "onUpdate:modelValue": (value: string) => (view.value = value),
      }),
    );

    host
      .querySelector('[aria-checked="true"]')
      ?.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));

    expect(view.value).toBe("table");
  });
});

describe("DbSearchField", () => {
  it("reports typing and clears on Escape", async () => {
    const query = ref("");
    const host = mount(() =>
      h(DbSearchField, {
        modelValue: query.value,
        placeholder: "Filter keys",
        "onUpdate:modelValue": (value: string) => (query.value = value),
      }),
    );
    const input = host.querySelector("input")!;
    expect(input.getAttribute("aria-label")).toBe("Filter keys");

    input.value = "API";
    input.dispatchEvent(new Event("input"));
    expect(query.value).toBe("API");
    await nextTick();

    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    expect(query.value).toBe("");
  });
});

describe("DbLevelBadge", () => {
  it("lights the given number of bars", () => {
    const host = mount(() =>
      h(DbLevelBadge, { label: "Write", filled: 3, strong: false }),
    );
    const bars = Array.from(host.querySelectorAll('[data-testid="level-bar"]'));

    expect(host.textContent?.trim()).toBe("Write");
    expect(bars).toHaveLength(5);
    expect(
      bars.filter((bar) => bar.className.includes("bg-ink-strong")),
    ).toHaveLength(3);
  });

  it("uses the accent colour for strong levels", () => {
    const host = mount(() =>
      h(DbLevelBadge, { label: "Manage", filled: 4, strong: true }),
    );
    const bars = Array.from(host.querySelectorAll('[data-testid="level-bar"]'));

    expect(
      bars.filter((bar) => bar.className.includes("bg-accent")),
    ).toHaveLength(4);
  });
});
