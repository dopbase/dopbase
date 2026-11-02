import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatDateTime,
  formatDuration,
  formatHosting,
  formatRelativeTime,
  formatLevel,
  formatList,
  formatRole,
} from "./format";

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
  it("returns the raw input when unparseable", () => {
    expect(formatDateTime("not-a-date")).toBe("not-a-date");
  });
});

describe("formatBytes", () => {
  it("formats zero and negative values", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(-10)).toBe("0 B");
  });

  it("formats bytes, kilobytes, and megabytes", () => {
    expect(formatBytes(500)).toBe("500 B");
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(20480)).toBe("20 KB");
    expect(formatBytes(1572864)).toBe("1.5 MB");
  });
});

describe("formatDuration", () => {
  it("formats zero and negative values as 0s", () => {
    expect(formatDuration(0)).toBe("0s");
    expect(formatDuration(-10)).toBe("0s");
    expect(formatDuration(NaN)).toBe("0s");
  });

  it("formats seconds", () => {
    expect(formatDuration(45)).toBe("45s");
  });

  it("formats minutes and seconds", () => {
    expect(formatDuration(60)).toBe("1m");
    expect(formatDuration(125)).toBe("2m 5s");
  });

  it("formats hours and minutes", () => {
    expect(formatDuration(3600)).toBe("1h");
    expect(formatDuration(7500)).toBe("2h 5m");
  });

  it("formats days and hours", () => {
    expect(formatDuration(86400)).toBe("1d");
    expect(formatDuration(93600)).toBe("1d 2h");
  });
});

describe("formatRole", () => {
  it("names each role and passes unknown ones through", () => {
    expect(formatRole("root")).toBe("Root Administrator");
    expect(formatRole("member")).toBe("Member");
    expect(formatRole("ai_agent")).toBe("AI Agent");
    expect(formatRole("auditor")).toBe("auditor");
  });
});

describe("formatLevel", () => {
  it("names each access level", () => {
    expect(formatLevel("metadata")).toBe("Names only");
    expect(formatLevel("project_manager")).toBe("Project manager");
    expect(formatLevel("none")).toBe("No access");
  });
});

describe("formatList", () => {
  it("joins names the way a sentence would", () => {
    expect(formatList([])).toBe("");
    expect(formatList(["staging"])).toBe("staging");
    expect(formatList(["staging", "prod"])).toBe("staging and prod");
    expect(formatList(["staging", "prod", "qa"])).toBe("staging, prod and qa");
  });
});

describe("formatHosting", () => {
  it("calls a dopbase.dev workspace Secure cloud", () => {
    expect(formatHosting("acme.dopbase.dev")).toBe("Secure cloud");
    expect(formatHosting("eu.acme.Dopbase.dev")).toBe("Secure cloud");
  });

  it("calls every other host Self-hosted", () => {
    expect(formatHosting("localhost")).toBe("Self-hosted");
    expect(formatHosting("dopbase.dev")).toBe("Self-hosted");
    expect(formatHosting("vault.acme.com")).toBe("Self-hosted");
    expect(formatHosting("dopbase.dev.evil.com")).toBe("Self-hosted");
    expect(formatHosting("notdopbase.dev")).toBe("Self-hosted");
  });
});
