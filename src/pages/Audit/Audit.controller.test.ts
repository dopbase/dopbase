import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { shortHash, useAuditController } from "./Audit.controller";
import * as auditApi from "~/services/audit.api";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import * as usersApi from "~/services/users.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import { maya, reviewBot, settle, signInAs } from "~/tests/people";

const { route, router } = vi.hoisted(() => ({
  route: { query: {} as Record<string, string> },
  router: { replace: vi.fn() },
}));
vi.mock("vue-router", () => ({
  useRoute: () => route,
  useRouter: () => router,
}));

vi.mock("~/services/audit.api");
vi.mock("~/services/projects.api");
vi.mock("~/services/environments.api");
vi.mock("~/services/users.api");

const event: auditApi.AuditEvent = {
  id: "aud_1",
  actorType: "admin",
  actorId: "usr_1",
  actorLabel: "a@b.c",
  action: "secret.revealed",
  projectId: "prj_1",
  environmentId: "env_1",
  resourceType: "secret",
  resourceId: "DATABASE_URL",
  ipAddress: null,
  metadata: {},
  createdAt: "2026-08-28T00:00:00Z",
  seq: 4812,
  recordHash: "9f2c0000000000000000a71e",
};

const report: auditApi.ChainVerifyReport = {
  ok: true,
  records: 4812,
  headSeq: 4812,
  headHash: "9f2c0000000000000000a71e",
  legacy: 0,
  unsequenced: 0,
  prunedSeq: null,
  broken: null,
};

/** Runs the controller mounted, so its first load and options run too. */
async function mounted() {
  const { controller } = mountController(useAuditController);
  await settle();
  return controller;
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  route.query = {};
  signInAs(["audit.read"]);
  vi.mocked(auditApi.listAuditEvents).mockResolvedValue({
    items: [event],
    nextCursor: null,
  });
  vi.mocked(auditApi.verifyAuditChain).mockResolvedValue(report);
  vi.mocked(projectsApi.listProjects).mockResolvedValue([
    { id: "prj_1", name: "payments", createdAt: "", updatedAt: "" },
  ]);
  vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
    {
      id: "env_1",
      projectId: "prj_1",
      projectName: "payments",
      name: "prod",
      createdAt: "",
      updatedAt: "",
    },
  ]);
  vi.mocked(usersApi.fetchUsers).mockResolvedValue([maya, reviewBot]);
});

describe("useAuditController", () => {
  it("describes the audit scope the viewer's capabilities allow", () => {
    signInAs([]);
    expect(useAuditController().scopeDescription.value).toBe(
      "Your own actions on this server. Values never appear here.",
    );

    signInAs(["audit.read"]);
    expect(useAuditController().scopeDescription.value).toBe(
      "Events on this server's settings and on the projects you reach. Values never appear here.",
    );

    signInAs(["audit.read_all"]);
    expect(useAuditController().scopeDescription.value).toBe(
      "Every event on this server. Values never appear here.",
    );
  });

  it("loads the first page with a page-size limit and no empty filters", async () => {
    vi.mocked(auditApi.listAuditEvents).mockResolvedValueOnce({
      items: [event],
      nextCursor: "cursor_2",
    });
    const c = await mounted();

    expect(auditApi.listAuditEvents).toHaveBeenCalledWith({
      limit: 25,
      action: undefined,
      projectId: undefined,
      environmentId: undefined,
      actor: undefined,
    });
    expect(c.items.value).toEqual([event]);
    expect(c.nextCursor.value).toBe("cursor_2");
    expect(c.hasLoaded.value).toBe(true);
  });

  it("puts a chosen action in the address and reloads with it", async () => {
    const c = await mounted();

    c.setAction("secret.revealed");
    await settle();

    expect(router.replace).toHaveBeenLastCalledWith({
      query: { action: "secret.revealed" },
    });
    expect(auditApi.listAuditEvents).toHaveBeenLastCalledWith(
      expect.objectContaining({ action: "secret.revealed" }),
    );
  });

  it("filters by an environment along with its project, and clears both", async () => {
    const c = await mounted();

    c.setWhere("environment:env_1");
    await settle();
    expect(c.whereValue.value).toBe("environment:env_1");
    expect(auditApi.listAuditEvents).toHaveBeenLastCalledWith(
      expect.objectContaining({ projectId: "prj_1", environmentId: "env_1" }),
    );
    expect(router.replace).toHaveBeenLastCalledWith({
      query: { projectId: "prj_1", environmentId: "env_1" },
    });

    c.setWhere("project:prj_1");
    expect(c.filters).toMatchObject({ projectId: "prj_1", environmentId: "" });

    c.setWhere(null);
    await settle();
    expect(router.replace).toHaveBeenLastCalledWith({ query: {} });
    expect(c.whereValue.value).toBeNull();
  });

  it("offers each project followed by its environments under Where", async () => {
    const c = await mounted();

    expect(c.whereOptions.value).toEqual([
      { value: "project:prj_1", label: "payments" },
      { value: "environment:env_1", label: "payments/prod" },
    ]);
  });

  it("preselects the Actor chip from ?actor= with the person's email", async () => {
    signInAs(["audit.read", "users.view"]);
    route.query = { actor: "usr_bot" };
    const c = await mounted();

    expect(auditApi.listAuditEvents).toHaveBeenCalledWith(
      expect.objectContaining({ actor: "usr_bot" }),
    );
    expect(c.actorOptions.value).toContainEqual({
      value: "usr_bot",
      label: "review-bot",
    });
  });

  it("keeps an unknown ?actor= as a chip when people can't be listed", async () => {
    route.query = { actor: "usr_gone" };
    const c = await mounted();

    expect(usersApi.fetchUsers).not.toHaveBeenCalled();
    expect(c.actorOptions.value).toEqual([
      { value: "usr_1", label: "a@b.c" },
      { value: "usr_gone", label: "usr_gone" },
    ]);
  });

  it("starts from a project page's Activity link", async () => {
    route.query = { projectId: "prj_1", environmentId: "env_1" };
    const c = await mounted();

    expect(c.whereValue.value).toBe("environment:env_1");
    expect(c.where(event)).toBe("payments/prod");
  });

  it("appends cursor pages on loadMore", async () => {
    vi.mocked(auditApi.listAuditEvents)
      .mockResolvedValueOnce({ items: [event], nextCursor: "cursor_2" })
      .mockResolvedValueOnce({
        items: [{ ...event, id: "aud_2" }],
        nextCursor: null,
      });
    const c = await mounted();
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
    const c = await mounted();

    expect(c.loadError.value).toBe("Could not load audit events.");
    expect(c.hasLoaded.value).toBe(false);
  });

  it("ignores a failure from a request a newer filter replaced", async () => {
    let failFirst!: (cause: Error) => void;
    vi.mocked(auditApi.listAuditEvents)
      .mockReturnValueOnce(new Promise((_, reject) => (failFirst = reject)))
      .mockResolvedValueOnce({ items: [event], nextCursor: null });
    const c = useAuditController();
    const first = c.load();
    await c.load();

    failFirst(new Error("stale"));
    await first;

    expect(c.loadError.value).toBeNull();
    expect(c.loading.value).toBe(false);
    expect(c.items.value).toHaveLength(1);
  });

  it("names the credential when a token made the request", async () => {
    const c = await mounted();

    expect(c.credential({ ...event, actorType: "personal_token" })).toBe(
      "personal token",
    );
    expect(c.credential({ ...event, actorType: "runner" })).toBe(
      "runner token",
    );
    expect(c.credential(event)).toBeNull();
  });

  it("shows where a request came from, with the client when recorded", async () => {
    const c = await mounted();

    expect(
      c.origin({
        ...event,
        ipAddress: "203.0.113.24",
        metadata: { client: "dopbase-cli 0.2.0" },
      }),
    ).toBe("203.0.113.24 · dopbase-cli 0.2.0");
    expect(c.origin(event)).toBe("");
  });

  it("opens an event and checks the chain once for it", async () => {
    signInAs(["audit.read_all", "audit.verify"]);
    const c = await mounted();

    c.openEvent(event);
    expect(c.selected.value).toEqual(event);
    expect(c.chainLine(event)).toEqual({
      tone: "neutral",
      text: "Checking the chain…",
    });
    await settle();

    expect(c.chainLine(event)).toEqual({
      tone: "ok",
      text: "Chain verified up to #4812 · sha256 9f2c…a71e",
    });
    c.closeEvent();
    c.openEvent({ ...event, id: "aud_2" });
    await settle();
    expect(auditApi.verifyAuditChain).toHaveBeenCalledTimes(1);
    expect(c.selected.value?.id).toBe("aud_2");
  });

  it("says where the chain breaks when it breaks before the event", async () => {
    signInAs(["audit.verify"]);
    vi.mocked(auditApi.verifyAuditChain).mockResolvedValue({
      ...report,
      ok: false,
      records: 4000,
      broken: { seq: 4001, id: "aud_x", reason: "record hash mismatch" },
    });
    const c = await mounted();
    await c.verifyChain();

    expect(c.chainLine(event)).toEqual({
      tone: "crit",
      text: "Chain broken at #4001: record hash mismatch",
    });
    expect(c.chainLine({ ...event, seq: 12, recordHash: null })).toEqual({
      tone: "ok",
      text: "Chain verified up to #4000",
    });
    expect(c.chainSummary.value).toEqual({
      tone: "crit",
      text: "The chain breaks at #4001: record hash mismatch",
    });
  });

  it("says the chain can't vouch for events from before it", async () => {
    signInAs(["audit.verify"]);
    const c = await mounted();
    await c.verifyChain();

    expect(c.chainLine({ ...event, seq: null })).toEqual({
      tone: "neutral",
      text: "This event is older than the hash chain, so the chain can't vouch for it.",
    });
    expect(c.chainSummary.value).toEqual({
      tone: "ok",
      text: "Chain verified up to #4812. 4812 records checked.",
    });
  });

  it("leaves the chain alone for roles without audit.verify", async () => {
    const c = await mounted();

    c.openEvent(event);
    await c.verifyChain();

    expect(auditApi.verifyAuditChain).not.toHaveBeenCalled();
    expect(c.chainLine(event)).toBeNull();
  });

  it("shows why the chain check failed", async () => {
    signInAs(["audit.verify"]);
    vi.mocked(auditApi.verifyAuditChain).mockRejectedValue(
      new ApiError(403, { AUTHORIZATION_DENIED: "Not for this token." }),
    );
    const c = await mounted();
    await c.verifyChain();

    expect(c.verifyError.value).toBe("Not for this token.");
    expect(c.chainLine(event)).toEqual({
      tone: "crit",
      text: "Not for this token.",
    });
  });

  it("puts the event's fields in the code block, without empty ones", async () => {
    const c = await mounted();

    expect(
      JSON.parse(
        c.eventPayload({
          ...event,
          environmentId: null,
          metadata: { count: 3 },
        }),
      ),
    ).toEqual({
      id: "aud_1",
      action: "secret.revealed",
      actorType: "admin",
      actor: "usr_1",
      project: "prj_1",
      resourceType: "secret",
      resource: "DATABASE_URL",
      metadata: { count: 3 },
      createdAt: "2026-08-28T00:00:00Z",
    });
  });

  it("shortens hashes to their ends", () => {
    expect(shortHash("abcdef0123456789")).toBe("abcd…6789");
    expect(shortHash("abc")).toBe("abc");
  });
});
