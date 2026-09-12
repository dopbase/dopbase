import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { MIN_PASSWORD_LENGTH, useSetupController } from "./Setup.controller";
import * as bootstrapApi from "~/services/bootstrap.api";
import { ApiError } from "~/services/http.client";

const routerPush = vi.hoisted(() => vi.fn());
const routerReplace = vi.hoisted(() => vi.fn());
const routeQuery = vi.hoisted(() => ({}) as Record<string, unknown>);

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: routerPush, replace: routerReplace }),
  useRoute: () => ({ query: routeQuery }),
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
  routerReplace.mockReset();
  for (const key of Object.keys(routeQuery)) delete routeQuery[key];
});

describe("useSetupController", () => {
  it("pre-fills the setup token from the ?token= query and strips it from the URL", () => {
    routeQuery.token = "setup_first-run-token";
    routeQuery.other = "kept";
    const c = useSetupController();
    expect(c.setupToken.value).toBe("setup_first-run-token");
    expect(routerReplace).toHaveBeenCalledTimes(1);
    expect(routerReplace).toHaveBeenCalledWith({ query: { other: "kept" } });
  });

  it("ignores a missing or non-string token query param", () => {
    routeQuery.token = ["setup_array"];
    const c = useSetupController();
    expect(c.setupToken.value).toBe("");
    expect(routerReplace).not.toHaveBeenCalled();
  });
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

  it("claims the server and routes to projects", async () => {
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
    expect(routerPush).toHaveBeenCalledWith({ name: "projects" });
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

  describe("restore mode", () => {
    it("validates file extension on selection", () => {
      const c = useSetupController();
      const invalidFile = new File(["dummy"], "backup.zip", {
        type: "application/zip",
      });
      c.onFileSelected(invalidFile);
      expect(c.selectedFile.value).toBeNull();
      expect(c.restoreError.value).toContain(".dop");

      const validFile = new File(["dummy"], "backup.dop", {
        type: "application/octet-stream",
      });
      c.onFileSelected(validFile);
      expect(c.selectedFile.value?.name).toBe("backup.dop");
      expect(c.restoreError.value).toBeNull();
    });

    it("requires a file before submitting restore", async () => {
      const c = useSetupController();
      await c.submitRestore();
      expect(c.restoreError.value).toContain("select a .dop");
      expect(bootstrapApi.bootstrapRestore).not.toHaveBeenCalled();
    });

    it("restores backup successfully and routes to login with notice", async () => {
      vi.mocked(bootstrapApi.bootstrapRestore).mockResolvedValueOnce({
        message: "Backup restored successfully.",
        restored: true,
        key: "backup.dop",
        size: 1024,
      });
      vi.mocked(bootstrapApi.fetchBootstrapStatus).mockResolvedValueOnce({
        state: "ready",
      });

      const c = useSetupController();
      const file = new File(["dummy"], "backup.dop", {
        type: "application/octet-stream",
      });
      c.onFileSelected(file);
      c.setupToken.value = "dbs_test-token";
      await c.submitRestore();

      expect(bootstrapApi.bootstrapRestore).toHaveBeenCalledWith(
        file,
        "dbs_test-token",
      );
      expect(routerPush).toHaveBeenCalledWith({
        name: "login",
        query: { notice: "backup-restored" },
      });
    });

    it("handles decryption failure with clear message", async () => {
      vi.mocked(bootstrapApi.bootstrapRestore).mockRejectedValueOnce(
        new ApiError(400, { BACKUP_DECRYPT_FAILED: "decryption failed" }),
      );

      const c = useSetupController();
      const file = new File(["dummy"], "backup.dop", {
        type: "application/octet-stream",
      });
      c.onFileSelected(file);
      c.setupToken.value = "dbs_test-token";
      await c.submitRestore();

      expect(c.restoreError.value).toContain("master key");
    });

    it("restores backup with master key file", async () => {
      vi.mocked(bootstrapApi.bootstrapRestore).mockResolvedValueOnce({
        message: "Restored",
        restored: true,
        key: "backup.dop",
        size: 1024,
      });
      vi.mocked(bootstrapApi.fetchBootstrapStatus).mockResolvedValueOnce({
        state: "ready",
      });

      const c = useSetupController();
      const file = new File(["dummy"], "backup.dop", {
        type: "application/octet-stream",
      });
      const keyFile = new File(["keydata"], "master.key", {
        type: "application/octet-stream",
      });
      c.onFileSelected(file);
      c.setupToken.value = "dbs_test-token";
      c.onMasterKeyFileSelected(keyFile);
      await c.submitRestore();

      expect(bootstrapApi.bootstrapRestore).toHaveBeenCalledWith(
        file,
        "dbs_test-token",
        keyFile,
      );
      expect(routerPush).toHaveBeenCalledWith({
        name: "login",
        query: { notice: "backup-restored" },
      });
    });
  });
});
