import { describe, expect, it } from "vitest";
import { tokenExpiresIn } from "./token-expiry";

describe("tokenExpiresIn", () => {
  it("accepts the maximum custom duration in either unit", () => {
    expect(tokenExpiresIn("custom", "26280", "h")).toBe("26280h");
    expect(tokenExpiresIn("custom", "1095", "d")).toBe("1095d");
    expect(tokenExpiresIn("never", "", "h")).toBe("never");
    expect(() => tokenExpiresIn("10y", "", "h")).toThrow();
  });

  it("rejects invalid custom durations", () => {
    for (const amount of [
      "",
      "0",
      "-1",
      "1.5",
      "1096",
      "999999999999999999999",
    ]) {
      expect(() => tokenExpiresIn("custom", amount, "d")).toThrow();
    }
  });
});
