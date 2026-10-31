import { createApp, h } from "vue";
import { afterEach, describe, expect, it } from "vitest";
import DbInput from "./DbInput.vue";

let app: ReturnType<typeof createApp> | null = null;

function mountInput(props: Record<string, unknown>): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp(() => h(DbInput, { modelValue: "staging", ...props }));
  app.mount(host);
  return host;
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
});

describe("DbInput", () => {
  it("shows a read-only value with an outline and a lock", () => {
    const host = mountInput({ label: "Environment", readonly: true });

    const input = host.querySelector("input")!;
    const box = host.querySelector<HTMLElement>("[data-readonly]")!;
    expect(input.readOnly).toBe(true);
    expect(box.className).not.toContain("bg-raised");
    expect(box.querySelector("svg")).not.toBeNull();
  });

  it("keeps editable fields filled", () => {
    const host = mountInput({ label: "Name" });

    expect(host.querySelector("input")!.readOnly).toBe(false);
    expect(host.querySelector("[data-readonly]")).toBeNull();
    expect(host.querySelector(".bg-raised")).not.toBeNull();
  });
});
