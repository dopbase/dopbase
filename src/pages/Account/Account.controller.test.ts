import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAccountController } from "./Account.controller";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";

const routerPush = vi.hoisted(() => vi.fn());

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: routerPush }),
  useRoute: () => ({ query: {} }),
}));

vi.mock("~/services/auth.api");

function signIn(store: ReturnType<typeof useAuthStore>): Promise<void> {
  vi.mocked(authApi.login).mockResolvedValueOnce({
    adminId: "usr_1",
    email: "a@b.c",
    sessionKind: "browser",
    token: null,
    csrfToken: "csrf_1",
  });
  return store.login("a@b.c", "oldpassword1");
}

beforeEach(() => {
  setActivePinia(createPinia());
  routerPush.mockReset();
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

  it("exposes the signed-in email", async () => {
    await signIn(useAuthStore());
    const c = useAccountController();
    expect(c.email.value).toBe("a@b.c");
  });
});
