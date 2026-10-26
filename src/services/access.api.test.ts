import { describe, expect, it } from "vitest";
import { reaches } from "./access.api";

describe("reaches", () => {
  it("compares levels in order, with no level reaching nothing", () => {
    expect(reaches("write", "read")).toBe(true);
    expect(reaches("read", "read")).toBe(true);
    expect(reaches("metadata", "read")).toBe(false);
    expect(reaches("project_manager", "manage")).toBe(true);
    expect(reaches(undefined, "metadata")).toBe(false);
  });
});
