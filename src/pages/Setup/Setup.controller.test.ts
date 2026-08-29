import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { MIN_PASSWORD_LENGTH, useSetupController } from "./Setup.controller";
import * as bootstrapApi from "~/services/bootstrap.api";
import { ApiError } from "~/services/http.client";

const routerPush = vi.hoisted(() => vi.fn());

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: routerPush }),
  useRoute: () => ({ query: {} }),
}));

vi.mock("~/services/bootstrap.api");

const VALID = {
  setupToken: "dbs_setup",
  email: "a@b.c",
  password: "longenoughpw1",
  confirmPassword: "longenoughpw1",
};

function fill(c: ReturnType<typeof useSetupController>): void {
  c.setupToken.value = VALID.setupToken;
  c.email.value = VALID.email;
  c.password.value = VALID.password;
  c.confirmPassword.value = VALID.confirmPassword;
}

beforeEach(() => {
  setActivePinia(createPinia());
  routerPush.mockReset();
});

describe("useSetupController", () => {
  it("blocks empty submissions with field errors", async () => {
    const c = useSetupController();
    await c.submit();
    expect(c.fieldErrors.value.setupToken).toBeTruthy();
    expect(c.fieldErrors.value.email).toBeTruthy();
    expect(c.fieldErrors.value.password).toBeTruthy();
    expect(bootstrapApi.bootstrapAdmin).not.toHaveBeenCalled();
  });

  it(`requires at least ${MIN_PASSWORD_LENGTH} characters`, async () => {
    const c = useSetupController();
    fill(c);
    c.password.value = "short";
    c.confirmPassword.value = "short";
    await c.submit();
    expect(c.fieldErrors.value.password).toContain("at least");
  });

  it("requires matching password confirmation", async () => {
    const c = useSetupController();
    fill(c);
    c.confirmPassword.value = "different-password";
    await c.submit();
    expect(c.fieldErrors.value.confirmPassword).toBe("Passwords do not match.");
    expect(c.passwordsMatch.value).toBe(false);
  });

  it("claims the server and routes to the workspace", async () => {
    vi.mocked(bootstrapApi.bootstrapAdmin).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      csrfToken: "csrf_1",
    });
    const c = useSetupController();
    fill(c);
    c.email.value = "  A@B.C  ";
    await c.submit();
    expect(bootstrapApi.bootstrapAdmin).toHaveBeenCalledWith({
      setupToken: "dbs_setup",
      email: "a@b.c",
      password: VALID.password,
    });
    expect(routerPush).toHaveBeenCalledWith({ name: "workspace" });
  });

  it("maps SETUP_TOKEN_INVALID to the token field", async () => {
    vi.mocked(bootstrapApi.bootstrapAdmin).mockRejectedValueOnce(
      new ApiError(401, { SETUP_TOKEN_INVALID: "bad token" }),
    );
    const c = useSetupController();
    fill(c);
    await c.submit();
    expect(c.fieldErrors.value.setupToken).toBeTruthy();
    expect(c.formError.value).toBeNull();
  });

  it("maps conflicts to an already-set-up message", async () => {
    vi.mocked(bootstrapApi.bootstrapAdmin).mockRejectedValueOnce(
      new ApiError(409, { BOOTSTRAP_ALREADY_CLOSED: "done" }),
    );
    const c = useSetupController();
    fill(c);
    await c.submit();
    expect(c.formError.value).toContain("already been set up");
  });
});
