import { beforeEach, describe, expect, it, vi } from "vitest";
import { useNoAccessController } from "./NoAccess.controller";

const { route, session } = await vi.hoisted(async () => {
  const { reactive } = await import("vue");
  return {
    route: reactive({ path: "/backups" }),
    session: {
      value: { role: "member", roleName: "Member" } as {
        role: string;
        roleName?: string;
        features?: string[];
      } | null,
    },
  };
});

const routeNames: Record<string, string> = {
  "/backups": "backups",
  "/groups": "groups",
  "/roles": "roles",
  "/sso": "sso",
  "/projects/p/billing/access": "project-access",
};

vi.mock("vue-router", () => ({
  useRoute: () => route,
  useRouter: () => ({
    resolve: (path: string) => ({ name: routeNames[path] ?? "not-found" }),
  }),
}));

vi.mock("~/stores/auth.store", () => ({
  useAuthStore: () => ({
    get session() {
      return session.value;
    },
    hasFeature: (feature: string) =>
      session.value?.features === undefined ||
      session.value.features.includes(feature),
  }),
}));

beforeEach(() => {
  route.path = "/backups";
  session.value = { role: "member", roleName: "Member" };
});

describe("useNoAccessController", () => {
  it("names the page and the capability it needs", () => {
    const { title, description } = useNoAccessController();

    expect(title.value).toBe("You don't have access to Backups");
    expect(description.value).toBe(
      "Backups needs the instance.backup capability, which your role (Member) doesn't have. Ask root if you need it.",
    );
  });

  it("lists every capability that would open the page", () => {
    route.path = "/groups";
    session.value = { role: "member", roleName: "Viewer" };

    const { description } = useNoAccessController();

    expect(description.value).toBe(
      "Groups needs users.view or groups.manage, and your role (Viewer) has neither. Ask root if you need it.",
    );
  });

  it("falls back to the built-in role name", () => {
    session.value = { role: "ai_agent" };

    const { description } = useNoAccessController();

    expect(description.value).toContain("your role (AI Agent)");
  });

  it("stays generic for a page outside the capability map", () => {
    route.path = "/projects/p/billing/access";

    const { title, description } = useNoAccessController();

    expect(title.value).toBe("You don't have access to this page");
    expect(description.value).toBe(
      "Your role doesn't include this page. Ask root if you need it.",
    );
  });

  it("points to the docs, not a setting, when the server has a page off", () => {
    route.path = "/sso";
    session.value = { role: "root", features: ["rbac"] };

    const { title, description, docsUrl } = useNoAccessController();

    expect(title.value).toBe("Single sign-on is turned off");
    expect(description.value).toBe(
      "This feature is disabled on this server. Your server administrator can enable it by following the",
    );
    expect(description.value).not.toContain("DOPBASE_");
    expect(docsUrl.value).toBe("https://docs.dopbase.com");
  });

  it("says groups come with people management", () => {
    route.path = "/groups";
    session.value = { role: "root", features: ["sso"] };

    const { description } = useNoAccessController();

    expect(description.value).toBe(
      "Groups require people management, which is disabled on this server. Your server administrator can enable it by following the",
    );
  });

  it("says custom roles need people management too", () => {
    route.path = "/roles";
    session.value = { role: "root", features: ["sso"] };

    const { description } = useNoAccessController();

    expect(description.value).toBe(
      "Custom roles require people management. Your server administrator can enable them by following the",
    );
  });

  it("blames only custom roles when people management is on", () => {
    route.path = "/roles";
    session.value = { role: "root", features: ["rbac"] };

    const { description } = useNoAccessController();

    expect(description.value).toBe(
      "Custom roles are disabled on this server. Your server administrator can enable them by following the",
    );
  });

  it("links no docs when the role is what's missing", () => {
    const { docsUrl } = useNoAccessController();

    expect(docsUrl.value).toBeNull();
  });
});
