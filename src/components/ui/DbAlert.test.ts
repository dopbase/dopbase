import { createApp, h } from "vue";
import { afterEach, describe, expect, it } from "vitest";
import DbAlert from "./DbAlert.vue";

let app: ReturnType<typeof createApp> | null = null;

function mountAlert(props: Record<string, unknown>, body: string): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp(() => h(DbAlert, props, () => body));
  app.mount(host);
  return host.firstElementChild as HTMLElement;
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
});

describe("DbAlert", () => {
  it("shows an icon, a title and the description below it", () => {
    const alert = mountAlert(
      { tone: "info", title: "Values stay in Secrets." },
      "Open Secrets to see them.",
    );

    expect(alert.querySelector("svg")).not.toBeNull();
    expect(alert.querySelector("p")?.textContent?.trim()).toBe(
      "Values stay in Secrets.",
    );
    expect(alert.textContent).toContain("Open Secrets to see them.");
  });

  it("stays a plain line without a title", () => {
    const alert = mountAlert({}, "Could not load.");

    expect(alert.querySelector("svg")).toBeNull();
    expect(alert.textContent?.trim()).toBe("Could not load.");
  });

  it("interrupts screen readers only for errors", () => {
    expect(mountAlert({}, "Failed.").getAttribute("role")).toBe("alert");
    app?.unmount();
    expect(mountAlert({ tone: "info" }, "Note.").getAttribute("role")).toBe(
      "note",
    );
    app?.unmount();
    expect(mountAlert({ tone: "success" }, "Saved.").getAttribute("role")).toBe(
      "status",
    );
  });
});
