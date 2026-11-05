import { describe, expect, it } from "vitest";
import { roleSummary, type RoleDraft } from "./role-summary";
import { roleFixture } from "~/tests/people";

// The built-in roles as the server seeds them.
const admin = roleFixture({
  id: "rol_admin",
  name: "Admin",
  rank: 10,
  maxLevel: "project_manager",
  capabilities: [
    "users.view",
    "users.manage",
    "groups.manage",
    "grants.manage",
    "projects.create",
    "audit.read",
    "audit.verify",
    "access.report",
    "instance.view",
    "tokens.personal",
    "tokens.revoke_others",
    "agents.view",
    "agents.view_all",
    "agents.manage",
  ],
});
const member = roleFixture({
  id: "rol_member",
  name: "Member",
  rank: 20,
  maxLevel: "manage",
  capabilities: ["tokens.personal", "agents.view"],
});
const auditor = roleFixture({
  id: "rol_auditor",
  name: "Auditor",
  rank: 30,
  maxLevel: "metadata",
  tokenScope: "audit",
  capabilities: [
    "users.view",
    "audit.read",
    "audit.read_all",
    "audit.verify",
    "access.report",
    "instance.view",
    "tokens.personal",
    "agents.view_all",
  ],
});
const agent = roleFixture({
  id: "rol_ai_agent",
  name: "AI agent",
  rank: 40,
  maxLevel: "write",
  signIn: "token_only",
  capabilities: ["tokens.personal"],
});
const custom = roleFixture({
  id: "rol_viewer",
  name: "Viewer",
  builtin: false,
  maxLevel: "write",
  capabilities: ["tokens.personal", "agents.view", "audit.read"],
});
const ROLES = [admin, member, auditor, agent, custom];

function draft(overrides: Partial<RoleDraft> = {}): RoleDraft {
  return {
    name: "Developer",
    capabilities: ["tokens.personal", "agents.view", "audit.read"],
    maxLevel: "write",
    tokenScope: "data",
    tokenMaxLevel: "read",
    tokenMaxTtlDays: 90,
    ...overrides,
  };
}

describe("roleSummary", () => {
  it("sums up a Developer the way the design does", () => {
    const summary = roleSummary(draft(), ROLES);

    expect(summary.maxLevel).toBe("write");
    expect([summary.held, summary.total]).toEqual([3, 14]);
    expect(summary.heldList).toBe(
      "Audit for their projects, tokens, own AI agents",
    );
    expect(summary.tokens).toBe("Read · 90 days");
    expect(summary.tokensDetail).toBe("Project data only");
    expect(summary.givers).toBe("Root and Admins");
    expect(summary.closest?.name).toBe("Member");
    expect(summary.closestDetail).toBe("+ audit.read, Write not Manage");
    expect(summary.warnings).toEqual([]);
    expect(summary.sentence).toBe(
      "Developer reaches Write and holds 3 of 14 capabilities.",
    );
  });

  it("counts only capabilities the editor lists, so grants.manage stays out", () => {
    const summary = roleSummary(
      draft({ capabilities: ["tokens.personal", "grants.manage"] }),
      ROLES,
    );

    expect(summary.held).toBe(1);
    expect(summary.heldList).toBe("Tokens");
  });

  it("names a role that holds nothing outside project data", () => {
    const summary = roleSummary(
      draft({ capabilities: [], tokenScope: "none" }),
      ROLES,
    );

    expect(summary.heldList).toBe("Nothing outside project data");
    expect(summary.tokens).toBe("None");
    expect(summary.tokensDetail).toBe("No personal tokens");
  });

  it("shortens a long list after three", () => {
    const summary = roleSummary(
      draft({
        capabilities: [
          "users.view",
          "groups.manage",
          "projects.create",
          "audit.read",
          "tokens.personal",
        ],
      }),
      ROLES,
    );

    expect(summary.heldList).toBe(
      "See people, manage groups, create projects and 2 more",
    );
  });

  it("describes audit tokens", () => {
    const summary = roleSummary(
      draft({ tokenScope: "audit", tokenMaxTtlDays: 30 }),
      ROLES,
    );

    expect(summary.tokens).toBe("Audit log · 30 days");
    expect(summary.tokensDetail).toBe("No project data");
  });

  it("lists the built-in roles that cover it and manage people, root first", () => {
    const summary = roleSummary(
      draft({ capabilities: ["tokens.personal"], maxLevel: "metadata" }),
      ROLES,
    );

    // Members and Auditors cover it but can't give anyone a role.
    expect(summary.givers).toBe("Root and Admins");
    expect(summary.giversDetail).toBe("Anyone whose role covers it");
  });

  it("lets anyone who runs agents give a token-only role", () => {
    const summary = roleSummary(
      draft({
        capabilities: ["tokens.personal"],
        maxLevel: "write",
        signIn: "token_only",
      }),
      ROLES,
    );

    expect(summary.givers).toBe("Root, Admins and Members");
  });

  it("leaves it to root when no built-in role covers it", () => {
    const summary = roleSummary(
      draft({
        capabilities: ["users.manage", "audit.read_all", "tokens.personal"],
        maxLevel: "manage",
      }),
      ROLES,
    );

    expect(summary.givers).toBe("Only root");
    expect(summary.giversDetail).toBe("No built-in role covers it");
  });

  it("matches a built-in role exactly, and never a token-only one", () => {
    const summary = roleSummary(
      draft({
        capabilities: ["tokens.personal", "agents.view"],
        maxLevel: "manage",
      }),
      ROLES,
    );

    expect(summary.closest?.name).toBe("Member");
    expect(summary.closestDetail).toBe("Same as Member");
  });

  it("never names the role being viewed as its own closest match", () => {
    const summary = roleSummary(
      draft({
        capabilities: ["tokens.personal", "agents.view"],
        maxLevel: "manage",
      }),
      ROLES,
      "rol_member",
    );

    expect(summary.closest?.name).not.toBe("Member");
  });

  it("shows two differences and counts the rest", () => {
    const summary = roleSummary(
      draft({
        capabilities: [
          "tokens.personal",
          "agents.view",
          "audit.read",
          "audit.verify",
          "instance.view",
        ],
        maxLevel: "read",
      }),
      [member],
    );

    expect(summary.closestDetail).toBe("+ audit.read, + audit.verify, 2 more");
  });

  it("warns about powers over other people's accounts and tokens", () => {
    const summary = roleSummary(
      draft({
        capabilities: ["users.manage", "tokens.revoke_others", "agents.manage"],
      }),
      ROLES,
    );

    expect(summary.warnings.map((warning) => warning.title)).toEqual([
      "Holders can manage people",
      "Holders can revoke other people's tokens",
      "Holders can manage any AI agent",
    ]);
  });

  it("names an unnamed role plainly in the sentence", () => {
    expect(roleSummary(draft({ name: "  " }), ROLES).sentence).toBe(
      "This role reaches Write and holds 3 of 14 capabilities.",
    );
  });
});
