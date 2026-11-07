import { describe, expect, it } from "vitest";
import type { AuditEvent } from "~/services/audit.api";
import { actionTone, eventDetail, eventOrigin } from "./audit";

function event(
  metadata: Record<string, unknown> = {},
  overrides: Partial<AuditEvent> = {},
): AuditEvent {
  return {
    id: "evt_1",
    actorType: "root",
    actorId: "usr_root",
    actorLabel: "root@dopbase.com",
    action: "login.succeeded",
    projectId: null,
    environmentId: null,
    resourceType: null,
    resourceId: null,
    ipAddress: null,
    metadata,
    createdAt: "2026-11-04T09:12:00Z",
    ...overrides,
  };
}

describe("actionTone", () => {
  it("marks failures and denials", () => {
    expect(actionTone("login.failed")).toBe("crit");
    expect(actionTone("cli_login.denied")).toBe("crit");
    expect(actionTone("login.succeeded")).toBe("accent");
  });
});

describe("eventOrigin", () => {
  it("joins the address and the client", () => {
    expect(
      eventOrigin(
        event({ client: "dopbase-cli 0.2.0" }, { ipAddress: "203.0.113.24" }),
      ),
    ).toBe("203.0.113.24 · dopbase-cli 0.2.0");
    expect(eventOrigin(event())).toBe("");
  });
});

describe("eventDetail", () => {
  it.each([
    [
      { principal: "maya@dopbase.com", level: "write" },
      "maya@dopbase.com · Write",
    ],
    [{ principal: "backend", level: "bogus" }, "backend"],
    [{ oldName: "staging", newName: "qa" }, "staging → qa"],
    [{ oldName: "Backend", name: "Platform" }, "Backend → Platform"],
    [
      { group: "Backend", email: "sam@dopbase.com" },
      "sam@dopbase.com in Backend",
    ],
    [{ group: "Backend", userId: "usr_sam" }, "Backend"],
    [{ email: "maya@dopbase.com", role: "rol_member" }, "maya@dopbase.com"],
    [{ key: "DATABASE_URL", version: 3 }, "DATABASE_URL v3"],
    [
      { key: "dopbase-2026-11-01.tar.gz", size: 12 },
      "dopbase-2026-11-01.tar.gz",
    ],
    [{ backup: "dopbase-2026-11-01.tar.gz" }, "dopbase-2026-11-01.tar.gz"],
    [{ name: "payments" }, "payments"],
    [{ issuer: "https://acme.okta.com" }, "https://acme.okta.com"],
    [{ hostname: "laptop" }, "laptop"],
    [{ count: 1 }, "1 key"],
    [{ entryCount: 12 }, "12 keys"],
    [{ kind: "cli" }, "CLI"],
    [{ kind: "browser" }, "browser"],
    [{ reason: "password_login_disabled" }, "password_login_disabled"],
    [{}, ""],
  ])("names %o as %s", (metadata, expected) => {
    expect(eventDetail(event(metadata))).toBe(expected);
  });
});
