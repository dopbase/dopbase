import { afterEach, describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";
import { useMediaQuery } from "./media-query";
import { mountController } from "~/tests/mount-controller";

function fakeMediaList(matches: boolean) {
  const listeners = new Set<(event: MediaQueryListEvent) => void>();
  const list = {
    matches,
    addEventListener: vi.fn((_: string, listener) => listeners.add(listener)),
    removeEventListener: vi.fn((_: string, listener) =>
      listeners.delete(listener),
    ),
    change(next: boolean) {
      for (const listener of listeners)
        listener({ matches: next } as MediaQueryListEvent);
    },
    listeners,
  };
  vi.spyOn(window, "matchMedia").mockReturnValue(
    list as unknown as MediaQueryList,
  );
  return list;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("useMediaQuery", () => {
  it("follows the query as the screen changes", async () => {
    const list = fakeMediaList(true);
    const { controller: matches } = mountController(() =>
      useMediaQuery("(max-width: 39.99rem)"),
    );
    expect(matches.value).toBe(true);

    list.change(false);
    await nextTick();

    expect(matches.value).toBe(false);
  });

  it("stops listening when the component unmounts", () => {
    const list = fakeMediaList(false);
    const { unmount } = mountController(() => useMediaQuery("(hover: none)"));

    unmount();

    expect(list.listeners.size).toBe(0);
  });
});
