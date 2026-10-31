import { createApp, h } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import DbRadioCard from "./DbRadioCard.vue";

let app: ReturnType<typeof createApp> | null = null;

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
});

function mount(props: InstanceType<typeof DbRadioCard>["$props"]): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({ render: () => h(DbRadioCard, props) });
  app.mount(host);
  return host;
}

describe("DbRadioCard", () => {
  it("shows its label and description and reports a pick", () => {
    const onSelect = vi.fn();
    const host = mount({
      name: "level",
      value: "write",
      checked: false,
      label: "Write",
      description: "Also add, change, import and roll back secrets.",
      onSelect,
    });

    expect(host.textContent).toContain("Write");
    expect(host.textContent).toContain("roll back secrets");
    host.querySelector("input")!.click();
    expect(onSelect).toHaveBeenCalledWith("write");
  });

  it("can't be picked while disabled", () => {
    const host = mount({
      name: "level",
      value: "project_manager",
      checked: false,
      label: "Project manager",
      disabled: true,
    });

    expect(host.querySelector("input")!.disabled).toBe(true);
  });
});
