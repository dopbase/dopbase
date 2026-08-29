import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAuditController } from "./Audit.controller";
import * as auditApi from "~/services/audit.api";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";

vi.mock("~/services/audit.api");
vi.mock("~/services/projects.api");
vi.mock("~/services/environments.api");

const event = {
  id: "aud_1",
  actorType: "admin",
  actorId: "usr_1",
  actorLabel: "a@b.c",
  action: "secret.revealed",
  projectId: "prj_1",
  environmentId: "env_1",
  resourceType: "secret",
  resourceId: "DATABASE_URL",
  metadata: {},
  createdAt: "2026-08-28T00:00:00Z",
};

beforeEach(() => {
  vi.mocked(projectsApi.listProjects).mockResolvedValue([]);
  vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([]);
});

describe("useAuditController", () => {
  it("loads the first page with a page-size limit", async () => {
    vi.mocked(auditApi.listAuditEvents).mockResolvedValueOnce({
      items: [event],
      nextCursor: "cursor_2",
    });
    const c = useAuditController();
    await c.load();
    expect(auditApi.listAuditEvents).toHaveBeenCalledWith(
      expect.objectContaining({ limit: 25 }),
    );
    expect(c.items.value).toEqual([event]);
    expect(c.nextCursor.value).toBe("cursor_2");
    expect(c.hasLoaded.value).toBe(true);
  });

  it("passes active filters to the API", async () => {
    vi.mocked(auditApi.listAuditEvents).mockResolvedValueOnce({
      items: [],
      nextCursor: null,
    });
    const c = useAuditController();
    c.filters.action = "secret.revealed";
    c.filters.projectId = "prj_1";
    await c.load();
    expect(auditApi.listAuditEvents).toHaveBeenCalledWith(
      expect.objectContaining({
        action: "secret.revealed",
        projectId: "prj_1",
      }),
    );
  });

  it("omits empty filters", async () => {
    vi.mocked(auditApi.listAuditEvents).mockResolvedValueOnce({
      items: [],
      nextCursor: null,
    });
    const c = useAuditController();
    await c.load();
    const query = vi.mocked(auditApi.listAuditEvents).mock.calls[0][0];
    expect(query.action).toBeUndefined();
    expect(query.projectId).toBeUndefined();
  });

  it("appends cursor pages on loadMore", async () => {
    vi.mocked(auditApi.listAuditEvents)
      .mockResolvedValueOnce({ items: [event], nextCursor: "cursor_2" })
      .mockResolvedValueOnce({
        items: [{ ...event, id: "aud_2" }],
        nextCursor: null,
      });
    const c = useAuditController();
    await c.load();
    await c.loadMore();
    expect(auditApi.listAuditEvents).toHaveBeenLastCalledWith(
      expect.objectContaining({ cursor: "cursor_2" }),
    );
    expect(c.items.value.map((item) => item.id)).toEqual(["aud_1", "aud_2"]);
    expect(c.nextCursor.value).toBeNull();
  });

  it("surfaces loading failures", async () => {
    vi.mocked(auditApi.listAuditEvents).mockRejectedValueOnce(
      new Error("down"),
    );
    const c = useAuditController();
    await c.load();
    expect(c.loadError.value).toBe("Could not load audit events.");
    expect(c.hasLoaded.value).toBe(false);
  });
});
