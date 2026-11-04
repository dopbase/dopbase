import { describe, expect, it } from "vitest";
import type { AuditEvent } from "~/services/audit.api";
import { actionTone, eventOrigin } from "./audit";

function event(
  metadata: Record<string, unknown> = {},
  overrides: Partial<AuditEvent> = {},
): AuditEvent {
  return {
    id: "evt_1",
    actorType: "root",
    actorId: "usr_root",
    actorLabel: "root@example.com",
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
