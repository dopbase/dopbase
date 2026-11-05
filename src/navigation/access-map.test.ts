import { describe, expect, it } from "vitest";
import { canOpen, visibleSections } from "./access-map";
import type { Capability } from "~/services/access.api";

function holding(...held: Capability[]) {
  return (...needed: Capability[]) => needed.some((cap) => held.includes(cap));
}

describe("canOpen", () => {
  it.each([
    ["projects", [], true],
    ["audit", [], true],
    ["status", [], true],
    ["users", [], false],
    ["users", ["users.view"], true],
    ["agents", ["agents.view"], true],
    ["agents", ["agents.view_all"], true],
    ["agents", ["users.view"], false],
    ["agents", ["agents.manage"], false],
    ["groups", ["groups.manage"], true],
    ["groups", ["users.view"], true],
    ["groups", ["users.manage"], false],
    ["roles", ["users.view"], false],
    ["backups", ["instance.backup"], true],
    ["sso", ["instance.backup"], false],
    // Routes outside the map are open to every signed-in account.
    ["account", [], true],
    ["project-access", [], true],
    [undefined, [], true],
  ] as const)("%s with %j is %s", (name, held, expected) => {
    expect(canOpen(name, holding(...held))).toBe(expected);
  });
});

describe("visibleSections", () => {
  it("shows every section in order to an account holding everything", () => {
    const sections = visibleSections(() => true);

    expect(sections.map((section) => section.label)).toEqual([
      "Workspace",
      "Access",
      "Instance",
    ]);
    expect(sections.flatMap((s) => s.items.map((item) => item.name))).toEqual([
      "projects",
      "audit",
      "users",
      "agents",
      "groups",
      "roles",
      "status",
      "backups",
      "sso",
    ]);
  });

  it("drops a section when the account can open none of its pages", () => {
    const sections = visibleSections(holding());

    expect(sections).toEqual([
      {
        label: "Workspace",
        items: [
          { name: "projects", label: "Projects" },
          { name: "audit", label: "Audit log" },
        ],
      },
      { label: "Instance", items: [{ name: "status", label: "Status" }] },
    ]);
  });

  it("keeps only the pages the account can open", () => {
    const people = visibleSections(holding("users.view")).find(
      (section) => section.label === "Access",
    );

    expect(people?.items.map((item) => item.name)).toEqual(["users", "groups"]);
  });

  it("shows a member who runs their own agents just AI agents", () => {
    const people = visibleSections(holding("agents.view")).find(
      (section) => section.label === "Access",
    );

    expect(people?.items.map((item) => item.name)).toEqual(["agents"]);
  });
});
