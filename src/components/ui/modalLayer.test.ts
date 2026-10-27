import { describe, expect, it, afterEach } from "vitest";
import { acquireModalLayer, releaseModalLayer } from "./modalLayer";

describe("modalLayer", () => {
  afterEach(() => {
    for (let i = 0; i < 8; i += 1) releaseModalLayer();
  });

  it("stacks layers above the mobile project rail", () => {
    const first = acquireModalLayer();
    const second = acquireModalLayer();
    expect(first).toBeGreaterThanOrEqual(70);
    expect(second).toBeGreaterThan(first);
    releaseModalLayer();
    releaseModalLayer();
  });

  it("keeps priority dialogs above routine stacks", () => {
    acquireModalLayer();
    const priority = acquireModalLayer(true);
    expect(priority).toBeGreaterThanOrEqual(100);
    releaseModalLayer();
    releaseModalLayer();
  });
});
