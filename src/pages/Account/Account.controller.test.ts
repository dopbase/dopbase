import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick } from "vue";
import { useAccountController } from "./Account.controller";
import * as authApi from "~/services/auth.api";
import * as accessApi from "~/services/access.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";
import { browserSession } from "~/tests/browser-session";
import { mountController } from "~/tests/mount-controller";
import { accessFixture, settle, signInAs } from "~/tests/people";

const { routerPush, routerReplace, route } = vi.hoisted(() => ({
  routerPush: vi.fn(),
  routerReplace: vi.fn(),
  route: { name: "account" as string | undefined, query: {} },
}));

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: routerPush, replace: routerReplace }),
  useRoute: () => route,
}));

vi.mock("~/services/auth.api");
vi.mock("~/services/access.api");

/** maya with personal tokens that reach data, or read the audit log. */
function signInWithTokens(tokenScope: "data" | "audit"): void {
  signInAs(["tokens.personal"], { adminId: "usr_maya", role: "member" });
  useAuthStore().session!.tokenScope = tokenScope;
}

function signIn(store: ReturnType<typeof useAuthStore>): Promise<void> {
  vi.mocked(authApi.login).mockResolvedValueOnce(browserSession());
  return store.login("a@b.c", "oldpassword1");
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  route.name = "account";
});

describe("useAccountController", () => {
  it("validates lengths and confirmation", async () => {
    await signIn(useAuthStore());
    const c = useAccountController();
    c.currentPassword.value = "oldpassword1";
    c.newPassword.value = "short";
    c.confirmPassword.value = "different";
    await c.submit();
    expect(c.fieldErrors.value.newPassword).toContain("at least");
    expect(c.fieldErrors.value.confirmPassword).toBe("Passwords do not match.");
    expect(authApi.changePassword).not.toHaveBeenCalled();
  });

  it("rotates the password and routes back to login", async () => {
    await signIn(useAuthStore());
    vi.mocked(authApi.changePassword).mockResolvedValueOnce();
    const c = useAccountController();
    c.currentPassword.value = "oldpassword1";
    c.newPassword.value = "brandnewpassword";
    c.confirmPassword.value = "brandnewpassword";
    await c.submit();
    expect(authApi.changePassword).toHaveBeenCalledWith(
      "oldpassword1",
      "brandnewpassword",
    );
    expect(routerPush).toHaveBeenCalledWith({
      name: "login",
      query: { notice: "password-changed" },
    });
    const store = useAuthStore();
    expect(store.isAuthenticated).toBe(false);
  });

  it("maps 401 to the current-password field", async () => {
    await signIn(useAuthStore());
    vi.mocked(authApi.changePassword).mockRejectedValueOnce(
      new ApiError(401, { AUTHENTICATION_INVALID: "wrong" }),
    );
    const c = useAccountController();
    c.currentPassword.value = "wrongpassword";
    c.newPassword.value = "brandnewpassword";
    c.confirmPassword.value = "brandnewpassword";
    await c.submit();
    expect(c.fieldErrors.value.currentPassword).toBe(
      "The current password is incorrect.",
    );
  });

  it("exposes the signed-in identity with a formatted role and last login", async () => {
    await signIn(useAuthStore());
    const c = useAccountController();
    expect(c.email.value).toBe("a@b.c");
    expect(c.role.value).toBe("admin");
    expect(c.formattedRole.value).toBe("Administrator");

    vi.mocked(authApi.login).mockResolvedValueOnce(
      browserSession({
        adminId: "usr_root",
        email: "root@example.com",
        csrfToken: "csrf_root",
        role: "root",
        lastLoginAt: new Date(Date.now() - 60000).toISOString(),
      }),
    );
    await useAuthStore().login("root@example.com", "oldpassword1");

    const root = useAccountController();
    expect(root.role.value).toBe("root");
    expect(root.formattedRole.value).toBe("Root Administrator");
    expect(root.formattedLastLogin.value).toBe("1m ago");
  });

  it("sets a first password without asking for a current one", async () => {
    await signIn(useAuthStore());
    useAuthStore().session = {
      ...useAuthStore().session!,
      hasPassword: false,
    };
    vi.mocked(authApi.changePassword).mockResolvedValueOnce();
    const c = useAccountController();
    expect(c.needsCurrentPassword.value).toBe(false);
    c.newPassword.value = "brandnewpassword";
    c.confirmPassword.value = "brandnewpassword";
    await c.submit();
    expect(c.fieldErrors.value.currentPassword).toBeUndefined();
    expect(authApi.changePassword).toHaveBeenCalledWith("", "brandnewpassword");
  });

  it("opens the password dialog with an empty form", async () => {
    await signIn(useAuthStore());
    const c = useAccountController();
    c.newPassword.value = "leftover";
    c.fieldErrors.value = { newPassword: "Too short." };

    c.openPassword();

    expect(c.passwordOpen.value).toBe(true);
    expect(c.newPassword.value).toBe("");
    expect(c.fieldErrors.value).toEqual({});
  });

  it("shows Profile and Personal tokens for roles whose tokens reach data", () => {
    signInWithTokens("data");
    const c = useAccountController();

    expect(c.tabs.value).toEqual([
      { key: "profile", label: "Profile & sign-in", to: { name: "account" } },
      {
        key: "tokens",
        label: "Personal tokens",
        to: { name: "account-tokens" },
      },
    ]);
    expect(c.subtitle.value).toBe("Your profile, sign-in and personal tokens.");
  });

  it("shows Audit tokens in place of Personal tokens for audit roles", () => {
    signInWithTokens("audit");
    route.name = "account-audit-tokens";
    const c = useAccountController();

    expect(c.tabs.value.map((tab) => tab.label)).toEqual([
      "Profile & sign-in",
      "Audit tokens",
    ]);
    expect(c.activeTab.value).toBe("audit-tokens");
    expect(routerReplace).not.toHaveBeenCalled();
  });

  it("moves a personal tokens address to Audit tokens for audit roles", () => {
    signInWithTokens("audit");
    route.name = "account-tokens";
    useAccountController();

    expect(routerReplace).toHaveBeenCalledWith({
      name: "account-audit-tokens",
    });
  });

  it("keeps root and roles without tokens on the profile", async () => {
    signInAs([], { role: "root" });
    route.name = "account-tokens";
    const c = useAccountController();
    await nextTick();

    expect(c.tabs.value.map((tab) => tab.key)).toEqual(["profile"]);
    expect(c.activeTab.value).toBe("profile");
    expect(c.subtitle.value).toBe("Your profile and sign-in.");
    expect(routerReplace).toHaveBeenCalledWith({ name: "account" });
  });

  it("lists the groups you belong to", async () => {
    signInWithTokens("data");
    vi.mocked(accessApi.fetchUserAccess).mockResolvedValue(
      accessFixture({ groups: [{ id: "grp_backend", name: "Backend" }] }),
    );
    const { controller } = mountController(useAccountController);
    await settle();

    expect(accessApi.fetchUserAccess).toHaveBeenCalledWith("me");
    expect(controller.groups.value).toEqual([
      { id: "grp_backend", name: "Backend" },
    ]);
    expect(controller.canOpenGroups.value).toBe(false);
  });

  it("leaves groups out when your access can't be read", async () => {
    signInWithTokens("data");
    vi.mocked(accessApi.fetchUserAccess).mockRejectedValue(new Error("down"));
    const { controller } = mountController(useAccountController);
    await settle();

    expect(controller.groups.value).toBeNull();
  });
});
