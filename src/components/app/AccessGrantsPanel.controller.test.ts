import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { ref } from "vue";
import { useAccessGrantsPanel } from "./AccessGrantsPanel.controller";
import type { GrantPrincipal } from "./GiveAccessDialog.controller";
import * as accessApi from "~/services/access.api";
import { ApiError } from "~/services/http.client";
import { grantFixture } from "~/tests/people";

vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  removeGrant: vi.fn(),
}));

const maya: GrantPrincipal = {
  type: "user",
  id: "usr_maya",
  label: "maya@example.com",
  maxLevel: "project_manager",
};

const direct = grantFixture({ id: "grt_pay", level: "write" });
const prodOverride = grantFixture({
  id: "grt_prod",
  scopeType: "environment",
  scopeId: "env_prod",
  scopeLabel: "payments/prod",
  level: "metadata",
});
const viaBackend = grantFixture({
  id: "grt_search",
  principalType: "group",
  principalId: "grp_backend",
  principalLabel: "Backend",
  scopeId: "prj_search",
  scopeLabel: "search",
  projectId: "prj_search",
  level: "read",
});
const everywhere = grantFixture({
  id: "grt_all",
  scopeType: "all",
  scopeId: "",
  scopeLabel: "",
  projectId: "",
  level: "metadata",
});
const auditOnly = grantFixture({
  id: "grt_audit",
  scopeType: "audit",
  scopeLabel: "",
});

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
});

function panel(
  grants: accessApi.Grant[],
  principal: GrantPrincipal = maya,
  changed = vi.fn(),
) {
  return {
    panel: useAccessGrantsPanel(ref(principal), ref(grants), changed),
    changed,
  };
}

describe("useAccessGrantsPanel", () => {
  it("lists all-projects first, then by project and environment, never audit scopes", () => {
    const { panel: p } = panel([
      viaBackend,
      prodOverride,
      auditOnly,
      direct,
      everywhere,
    ]);
    expect(p.rows.value.map((row) => row.grant.id)).toEqual([
      "grt_all",
      "grt_pay",
      "grt_prod",
      "grt_search",
    ]);
  });

  it("splits an environment grant into project and environment with links", () => {
    const { panel: p } = panel([direct, prodOverride, everywhere]);
    const [all, project, env] = p.rows.value;
    expect(all).toMatchObject({
      project: "All projects",
      allProjects: true,
      projectLink: null,
    });
    expect(project).toMatchObject({
      project: "payments",
      environment: null,
      projectLink: { name: "project", params: { projectRef: "payments" } },
    });
    expect(env).toMatchObject({
      project: "payments",
      environment: "prod",
      projectLink: {
        name: "environment",
        params: { projectRef: "payments", environmentName: "prod" },
      },
    });
  });

  it("says which grants come through a group", () => {
    const { panel: p } = panel([direct, viaBackend]);
    expect(p.rows.value.map((row) => row.viaGroup)).toEqual([
      null,
      { id: "grp_backend", name: "Backend" },
    ]);
  });

  it("treats a group's own grants as direct on the group's page", () => {
    const { panel: p } = panel([viaBackend], {
      type: "group",
      id: "grp_backend",
      label: "Backend",
    });
    expect(p.rows.value[0]?.viaGroup).toBeNull();
  });

  it("explains an environment grant that overrides the project grant", () => {
    const { panel: p } = panel([direct, prodOverride]);
    expect(p.overrides.value).toEqual([
      "An environment grant overrides the project grant, so maya@example.com gets Names only in payments/prod.",
    ]);
  });

  it("says can't reach for a None override, and stays quiet with no project grant", () => {
    const blocked = grantFixture({
      ...prodOverride,
      level: "none",
    });
    expect(panel([direct, blocked]).panel.overrides.value[0]).toContain(
      "maya@example.com can't reach payments/prod",
    );
    expect(panel([prodOverride, everywhere]).panel.overrides.value).toEqual([]);
  });

  it("filters by project or environment", () => {
    const { panel: p } = panel([direct, prodOverride, viaBackend]);
    p.filter.value = "PROD";
    expect(p.visibleRows.value.map((row) => row.grant.id)).toEqual([
      "grt_prod",
    ]);
    p.filter.value = "search";
    expect(p.visibleRows.value.map((row) => row.grant.id)).toEqual([
      "grt_search",
    ]);
  });

  it("opens the dialog to give new access or change a level", async () => {
    const { panel: p, changed } = panel([direct]);
    p.openGive();
    expect(p.dialogOpen.value).toBe(true);
    expect(p.editing.value).toBeNull();
    p.openChange(direct);
    expect(p.editing.value).toStrictEqual(direct);
    await p.saved();
    expect(p.dialogOpen.value).toBe(false);
    expect(changed).toHaveBeenCalledOnce();
  });

  it("removes a direct grant and tells the page", async () => {
    vi.mocked(accessApi.removeGrant).mockResolvedValueOnce();
    const { panel: p, changed } = panel([direct]);
    p.askRemove(p.rows.value[0]!);
    await p.confirmRemove();
    expect(accessApi.removeGrant).toHaveBeenCalledWith("grt_pay");
    expect(p.removing.value).toBeNull();
    expect(changed).toHaveBeenCalledOnce();
  });

  it("keeps the confirm open with the reason when removal fails", async () => {
    vi.mocked(accessApi.removeGrant).mockRejectedValueOnce(
      new ApiError(403, { ACCESS_DENIED: "You don't manage payments." }),
    );
    const { panel: p, changed } = panel([direct]);
    p.askRemove(p.rows.value[0]!);
    await p.confirmRemove();
    expect(p.removing.value).not.toBeNull();
    expect(p.removeError.value).toBe("You don't manage payments.");
    expect(changed).not.toHaveBeenCalled();
  });

  it("won't close the confirm while removing", async () => {
    let finish!: () => void;
    vi.mocked(accessApi.removeGrant).mockReturnValueOnce(
      new Promise((resolve) => (finish = resolve)),
    );
    const { panel: p } = panel([direct]);
    p.askRemove(p.rows.value[0]!);
    const pending = p.confirmRemove();
    p.cancelRemove();
    expect(p.removing.value).not.toBeNull();
    finish();
    await pending;
    expect(p.removing.value).toBeNull();
  });
});
