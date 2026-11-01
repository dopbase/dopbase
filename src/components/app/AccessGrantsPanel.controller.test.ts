import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { ref } from "vue";
import { useAccessGrantsPanel } from "./AccessGrantsPanel.controller";
import type { GrantPrincipal } from "~/utils/level";
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
  readOnly = false,
) {
  return {
    panel: useAccessGrantsPanel(
      ref(principal),
      ref(grants),
      changed,
      ref(readOnly),
    ),
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

  it("lists an environment grant that overrides the project grant, with both levels", () => {
    const { panel: p } = panel([direct, prodOverride]);
    expect(p.overrides.value).toEqual([
      {
        key: "grt_prod",
        project: "payments",
        environment: "prod",
        from: expect.objectContaining({ label: "Write" }),
        to: expect.objectContaining({ label: "Names only" }),
      },
    ]);
  });

  it("compares against the highest project grant, direct or through a group", () => {
    const readViaGroup = grantFixture({
      ...viaBackend,
      id: "grt_pay_group",
      scopeId: direct.scopeId,
      scopeLabel: "payments",
      projectId: direct.projectId,
      level: "manage",
    });
    const { panel: p } = panel([direct, readViaGroup, prodOverride]);
    expect(p.overrides.value[0]?.from.label).toBe("Manage");
  });

  it("marks a None override as blocked, and stays quiet with no project grant", () => {
    const blocked = grantFixture({
      ...prodOverride,
      level: "none",
    });
    expect(panel([direct, blocked]).panel.overrides.value[0]?.to).toBeNull();
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

  it("opens the change dialog for a grant and closes it once saved", async () => {
    const { panel: p, changed } = panel([direct]);
    p.openChange(direct);
    expect(p.editing.value).toStrictEqual(direct);
    await p.saved();
    expect(p.editing.value).toBeNull();
    expect(changed).toHaveBeenCalledOnce();
  });

  it("gives a row a menu only when it has something in it", () => {
    const { panel: p } = panel([everywhere, direct, viaBackend]);
    expect(
      p.rows.value.map((row) => [row.grant.id, row.editable, row.hasActions]),
    ).toEqual([
      ["grt_all", false, false],
      ["grt_pay", true, true],
      ["grt_search", false, true],
    ]);
  });

  it("lets read-only viewers open projects but not change grants", () => {
    const { panel: p } = panel([everywhere, direct], maya, vi.fn(), true);
    expect(
      p.rows.value.map((row) => [row.grant.id, row.editable, row.hasActions]),
    ).toEqual([
      ["grt_all", false, false],
      ["grt_pay", false, true],
    ]);
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

  it("selects only grants you can remove, and the header box follows the filter", () => {
    const { panel: p } = panel([direct, prodOverride, viaBackend, everywhere]);
    expect(p.canSelect.value).toBe(true);
    const [, pay, prod, search] = p.rows.value;
    p.toggleRow(search!, true); // through a group, so not selectable
    expect(p.selectedRows.value).toEqual([]);

    p.toggleRow(prod!, true);
    expect(p.someVisibleSelected.value).toBe(true);
    expect(p.allVisibleSelected.value).toBe(false);

    p.filter.value = "prod";
    expect(p.allVisibleSelected.value).toBe(true);
    p.toggleAllVisible(false);
    expect(p.selectedRows.value).toEqual([]);

    p.filter.value = "";
    p.toggleAllVisible(true);
    expect(p.selectedRows.value.map((row) => row.grant.id)).toEqual([
      pay!.grant.id,
      prod!.grant.id,
    ]);
    p.clearSelection();
    expect(p.selectedRows.value).toEqual([]);
  });

  it("shows no checkboxes to read-only viewers", () => {
    const { panel: p } = panel([direct], maya, vi.fn(), true);
    expect(p.canSelect.value).toBe(false);
  });

  it("removes every selected grant and clears the selection", async () => {
    vi.mocked(accessApi.removeGrant).mockResolvedValue();
    const { panel: p, changed } = panel([direct, prodOverride]);
    p.toggleAllVisible(true);
    p.askRemoveSelected();
    expect(p.removeTitle.value).toBe("Remove 2 grants?");
    expect(p.removeDescription.value).toBe(
      "maya@example.com loses access to payments and payments/prod on their next request. Other grants they hold stay.",
    );
    await p.confirmRemove();
    expect(vi.mocked(accessApi.removeGrant).mock.calls).toEqual([
      ["grt_pay"],
      ["grt_prod"],
    ]);
    expect(p.removing.value).toBeNull();
    expect(p.selectedRows.value).toEqual([]);
    expect(changed).toHaveBeenCalledOnce();
  });

  it("says how far it got and keeps the rest when one removal fails", async () => {
    vi.mocked(accessApi.removeGrant)
      .mockResolvedValueOnce()
      .mockRejectedValueOnce(
        new ApiError(403, { ACCESS_DENIED: "You don't manage payments." }),
      );
    const { panel: p, changed } = panel([direct, prodOverride]);
    p.toggleAllVisible(true);
    p.askRemoveSelected();
    await p.confirmRemove();
    expect(p.removeError.value).toBe(
      "Removed 1 of 2. payments/prod failed: You don't manage payments.",
    );
    expect(p.removing.value?.map((row) => row.grant.id)).toEqual(["grt_prod"]);
    expect(p.removeTitle.value).toBe("Remove access to payments/prod?");
    expect(p.selectedRows.value.map((row) => row.grant.id)).toEqual([
      "grt_prod",
    ]);
    expect(changed).toHaveBeenCalledOnce();
  });

  it("drops what went from the confirm when sign-in is cancelled partway", async () => {
    vi.mocked(accessApi.removeGrant)
      .mockResolvedValueOnce()
      .mockRejectedValueOnce(new DOMException("Cancelled", "AbortError"));
    const { panel: p, changed } = panel([direct, prodOverride]);
    p.toggleAllVisible(true);
    p.askRemoveSelected();
    await p.confirmRemove();
    expect(p.removing.value?.map((row) => row.grant.id)).toEqual(["grt_prod"]);
    expect(p.selectedRows.value.map((row) => row.grant.id)).toEqual([
      "grt_prod",
    ]);
    expect(p.removeError.value).toBe(
      "Removed 1 of 2. Confirm again to remove the rest.",
    );
    expect(changed).toHaveBeenCalledOnce();

    // Confirming again only sends the one that's left.
    vi.mocked(accessApi.removeGrant).mockClear().mockResolvedValue();
    await p.confirmRemove();
    expect(vi.mocked(accessApi.removeGrant).mock.calls).toEqual([["grt_prod"]]);
  });

  it("stays quiet when sign-in is cancelled before anything went", async () => {
    vi.mocked(accessApi.removeGrant).mockRejectedValueOnce(
      new DOMException("Cancelled", "AbortError"),
    );
    const { panel: p, changed } = panel([direct]);
    p.askRemove(p.rows.value[0]!);
    await p.confirmRemove();
    expect(p.removeError.value).toBeNull();
    expect(p.removing.value?.map((row) => row.grant.id)).toEqual(["grt_pay"]);
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
