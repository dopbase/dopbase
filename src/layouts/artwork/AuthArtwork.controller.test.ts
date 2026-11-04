import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ARTWORKS,
  pickArtwork,
  useAuthArtworkController,
} from "./AuthArtwork.controller";

const names = ARTWORKS.map((artwork) => artwork.name);

afterEach(() => {
  vi.restoreAllMocks();
});

describe("pickArtwork", () => {
  it("draws from every artwork when none was shown before", () => {
    const seen = new Set(
      names.map((_, i) => pickArtwork(null, () => i / names.length).name),
    );
    expect([...seen].sort()).toEqual([...names].sort());
  });

  it("never repeats the last artwork", () => {
    for (const last of names) {
      for (const roll of [0, 0.25, 0.5, 0.75, 0.999]) {
        expect(pickArtwork(last, () => roll).name).not.toBe(last);
      }
    }
  });

  it("treats an unknown stored name as nothing shown", () => {
    expect(pickArtwork("retired-art", () => 0).name).toBe(names[0]);
  });
});

describe("useAuthArtworkController", () => {
  it("remembers the pick so the next visit shows another", () => {
    const first = useAuthArtworkController(() => 0).artwork;
    expect(localStorage.getItem("dopbase.auth-artwork")).toBe(first.name);

    const second = useAuthArtworkController(() => 0).artwork;
    expect(second.name).not.toBe(first.name);
    expect(localStorage.getItem("dopbase.auth-artwork")).toBe(second.name);
  });

  it("still picks one when storage is refused", () => {
    vi.spyOn(localStorage, "getItem").mockImplementation(() => {
      throw new Error("denied");
    });
    vi.spyOn(localStorage, "setItem").mockImplementation(() => {
      throw new Error("denied");
    });

    expect(names).toContain(useAuthArtworkController(() => 0.5).artwork.name);
  });
});
