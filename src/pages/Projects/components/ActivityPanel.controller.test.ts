import { beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick, ref } from "vue";
import { useActivityPanelController } from "./ActivityPanel.controller";
import * as auditApi from "~/services/audit.api";
import type { AuditEvent } from "~/services/audit.api";
import type { Environment } from "~/services";
import { mountController } from "~/tests/mount-controller";

vi.mock("~/services/audit.api");

const prod: Environment = {
  id: "env_prod",
  projectId: "prj_pay",
  projectName: "payments",
  name: "prod",
  protected: true,
  createdAt: "",
  updatedAt: "",
};

function event(overrides: Partial<AuditEvent>): AuditEvent {
  return {
    id: "aud_1",
    actorType: "admin",
    actorId: "usr_1",
    actorLabel: "maya@example.com",
    action: "secret.revealed",
    projectId: "prj_pay",
    environmentId: "env_prod",
    resourceType: "secret",
    resourceId: "STRIPE_WEBHOOK_SECRET",
    ipAddress: null,
    metadata: {},
    createdAt: new Date().toISOString(),
    ...overrides,
  };
}

function panel() {
  return mountController(() =>
    useActivityPanelController(ref("prj_pay"), ref(prod), ref([prod])),
  ).controller;
}

beforeEach(() => {
  vi.mocked(auditApi.listAuditEvents).mockReset();
  vi.mocked(auditApi.listAuditEvents).mockResolvedValue({
    items: [event({})],
    nextCursor: null,
  });
});

describe("useActivityPanelController", () => {
  it("lists this environment's events first", async () => {
    const c = panel();

    await vi.waitFor(() => expect(c.rows.value).toHaveLength(1));
    expect(auditApi.listAuditEvents).toHaveBeenCalledWith(
      {
        limit: 25,
        projectId: "prj_pay",
        environmentId: "env_prod",
        action: undefined,
        actor: undefined,
      },
      expect.any(AbortSignal),
    );
    expect(c.rows.value[0]).toMatchObject({
      time: "just now",
      actor: "maya@example.com",
      where: "prod",
      whereProtected: true,
      details: "STRIPE_WEBHOOK_SECRET",
    });
  });

  it("widens to the whole project and narrows by action and actor", async () => {
    const c = panel();
    await vi.waitFor(() => expect(c.rows.value).toHaveLength(1));

    c.wholeProject.value = true;
    c.action.value = "secret.updated";
    c.actor.value = " lead@example.com ";
    await nextTick();

    await vi.waitFor(() =>
      expect(auditApi.listAuditEvents).toHaveBeenLastCalledWith(
        {
          limit: 25,
          projectId: "prj_pay",
          environmentId: undefined,
          action: "secret.updated",
          actor: "lead@example.com",
        },
        expect.any(AbortSignal),
      ),
    );
    expect(c.auditLink.value).toEqual({
      name: "audit",
      query: {
        projectId: "prj_pay",
        action: "secret.updated",
        actor: "lead@example.com",
      },
    });
  });

  it("appends the next page", async () => {
    vi.mocked(auditApi.listAuditEvents)
      .mockResolvedValueOnce({ items: [event({})], nextCursor: "c2" })
      .mockResolvedValueOnce({
        items: [event({ id: "aud_2", environmentId: null })],
        nextCursor: null,
      });
    const c = panel();
    await vi.waitFor(() => expect(c.nextCursor.value).toBe("c2"));

    await c.loadMore();

    expect(c.rows.value.map((row) => row.where)).toEqual(["prod", null]);
    expect(c.nextCursor.value).toBeNull();
  });

  it("cancels the older request when a filter changes", async () => {
    const c = panel();
    await vi.waitFor(() => expect(c.rows.value).toHaveLength(1));
    const first = vi.mocked(auditApi.listAuditEvents).mock.calls[0]![1]!;

    c.wholeProject.value = true;
    await nextTick();

    expect(first.aborted).toBe(true);
  });

  it("explains a failed load", async () => {
    vi.mocked(auditApi.listAuditEvents).mockRejectedValue(new Error(""));
    const c = panel();

    await vi.waitFor(() =>
      expect(c.loadError.value).toBe("Could not load the activity."),
    );
  });
});
