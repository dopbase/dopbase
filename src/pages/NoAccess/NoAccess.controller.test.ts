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
      } | null,
    },
  };
});

const routeNames: Record<string, string> = {
  "/backups": "backups",
  "/groups": "groups",
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
});
