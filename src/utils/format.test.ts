import { describe, expect, it } from "vitest";
import { formatDateTime, formatRelativeTime } from "./format";

const NOW = new Date("2026-08-28T12:00:00Z");

describe("formatRelativeTime", () => {
  it("renders just now for very fresh timestamps", () => {
    expect(formatRelativeTime("2026-08-28T11:59:50Z", NOW)).toBe("just now");
  });

  it("renders minutes and hours", () => {
    expect(formatRelativeTime("2026-08-28T11:30:00Z", NOW)).toBe("30m ago");
    expect(formatRelativeTime("2026-08-28T09:00:00Z", NOW)).toBe("3h ago");
  });

  it("renders days and months", () => {
    expect(formatRelativeTime("2026-08-20T12:00:00Z", NOW)).toBe("8d ago");
    expect(formatRelativeTime("2026-05-01T12:00:00Z", NOW)).toBe("3mo ago");
    expect(formatRelativeTime("2025-06-01T12:00:00Z", NOW)).toBe("1y ago");
  });

  it("falls back to a dash for unparseable input", () => {
    expect(formatRelativeTime("not-a-date", NOW)).toBe("—");
  });
});

describe("formatDateTime", () => {
  it("returns a non-empty localized string for valid input", () => {
    expect(formatDateTime("2026-08-28T12:00:00Z")).toMatch(/\d/);
  });

  it("returns the raw input when unparseable", () => {
    expect(formatDateTime("not-a-date")).toBe("not-a-date");
  });
});
