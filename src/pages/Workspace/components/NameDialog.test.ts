import { createApp, h, nextTick, reactive } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import NameDialog from "./NameDialog.vue";

const mountedApps: Array<ReturnType<typeof createApp>> = [];

async function waitForFocus(): Promise<void> {
  await nextTick();
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
}

afterEach(() => {
  for (const app of mountedApps.splice(0)) app.unmount();
  document.body.innerHTML = "";
});

describe("NameDialog", () => {
  it("focuses the name input whenever the dialog opens", async () => {
    const state = reactive({ open: false, initialName: "" });
    const host = document.createElement("div");
    document.body.append(host);
    const app = createApp({
      render: () =>
        h(NameDialog, {
          open: state.open,
          initialName: state.initialName,
          title: "New project",
          label: "Project name",
          action: vi.fn(async () => undefined),
        }),
    });
    mountedApps.push(app);
    app.mount(host);

    state.open = true;
    await waitForFocus();
    const input = document.body.querySelector<HTMLInputElement>(
      'input[name="resource-name"]',
    );
    expect(document.activeElement).toBe(input);

    state.open = false;
    await nextTick();
    state.initialName = "payments";
    state.open = true;
    await waitForFocus();
    const reopenedInput = document.body.querySelector<HTMLInputElement>(
      'input[name="resource-name"]',
    );
    expect(document.activeElement).toBe(reopenedInput);
    expect(reopenedInput?.value).toBe("payments");
  });
});
