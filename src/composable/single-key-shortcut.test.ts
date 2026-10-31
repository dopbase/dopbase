import { afterEach, describe, expect, it, vi } from "vitest";
import { useSingleKeyShortcut } from "./single-key-shortcut";
import { mountController } from "~/tests/mount-controller";

function press(key: string, init: KeyboardEventInit = {}, target?: Element) {
  (target ?? document.body).dispatchEvent(
    new KeyboardEvent("keydown", { key, bubbles: true, ...init }),
  );
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("useSingleKeyShortcut", () => {
  it("runs on the bare key", () => {
    const handler = vi.fn();
    mountController(() => useSingleKeyShortcut("e", handler));

    press("e");
    press("E");

    expect(handler).toHaveBeenCalledTimes(2);
  });

  it("ignores modifiers, other keys and typing in a field", () => {
    const handler = vi.fn();
    mountController(() => useSingleKeyShortcut("e", handler));
    const input = document.createElement("input");
    document.body.append(input);

    press("e", { metaKey: true });
    press("e", { ctrlKey: true });
    press("r");
    press("e", {}, input);

    expect(handler).not.toHaveBeenCalled();
  });

  it("pauses while a dialog or menu is open", () => {
    const handler = vi.fn();
    mountController(() => useSingleKeyShortcut("e", handler));
    const dialog = document.createElement("div");
    dialog.setAttribute("role", "dialog");
    document.body.append(dialog);

    press("e");

    expect(handler).not.toHaveBeenCalled();
  });

  it("stops listening when its owner unmounts", () => {
    const handler = vi.fn();
    const { unmount } = mountController(() =>
      useSingleKeyShortcut("e", handler),
    );

    unmount();
    press("e");

    expect(handler).not.toHaveBeenCalled();
  });
});
