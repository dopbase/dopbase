import { describe, expect, it } from "vitest";
import { levelMeter } from "./level";

describe("levelMeter", () => {
  it.each([
    ["none", "No access", 0, false],
    ["metadata", "Names only", 1, false],
    ["read", "Read", 2, false],
    ["write", "Write", 3, false],
    ["manage", "Manage", 4, true],
    ["project_manager", "Project manager", 5, true],
  ] as const)("%s draws %s with %i bars", (level, label, filled, strong) => {
    expect(levelMeter(level)).toEqual({ label, filled, strong });
  });
});
