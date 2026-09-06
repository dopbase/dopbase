import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useUsersController } from "./Users.controller";
import * as usersApi from "~/services/users.api";
import * as serviceAccountsApi from "~/services/service-accounts.api";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: vi.fn() }),
  useRoute: () => ({ query: {} }),
}));

vi.mock("~/services/users.api");
vi.mock("~/services/service-accounts.api");
vi.mock("~/services/auth.api");

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(usersApi.fetchUsers).mockResolvedValue([
    {
      id: "usr_root",
      email: "root@example.com",
      role: "root",
      createdAt: "2026-08-01T00:00:00Z",
      updatedAt: "2026-08-01T00:00:00Z",
    },
  ]);
  vi.mocked(serviceAccountsApi.fetchServiceAccounts).mockResolvedValue([]);
  vi.mocked(authApi.reauthenticate).mockReset();
});

describe("useUsersController", () => {
  it("computes canSave correctly for create and edit modes", () => {
    const c = useUsersController();

    // Create mode: initially empty email and password
    c.openCreate();
    expect(c.canSave.value).toBe(false);

    c.email.value = "admin@example.com";
    expect(c.canSave.value).toBe(false);

    c.password.value = "securepassword123";
    expect(c.canSave.value).toBe(true);

    // Edit mode: password is optional
    c.openEdit({
      id: "usr_2",
      email: "member@example.com",
      role: "member",
      createdAt: "",
      updatedAt: "",
    });
    expect(c.canSave.value).toBe(true);

    c.email.value = "   ";
    expect(c.canSave.value).toBe(false);
  });

  it("validates email format and password length client-side", async () => {
    const c = useUsersController();
    c.openCreate();
    c.email.value = "invalid-email";
    c.password.value = "short";

    await c.save();

    expect(c.fieldErrors.value.email).toBe("Please enter a valid email address.");
    expect(c.fieldErrors.value.password).toContain("at least 12 characters");
    expect(usersApi.createUser).not.toHaveBeenCalled();
    expect(c.error.value).toBeNull(); // Page error is untouched
  });

  it("maps server ApiError validation to fieldErrors without polluting page error", async () => {
    const c = useUsersController();
    c.openCreate();
    c.email.value = "valid@example.com";
    c.password.value = "validpassword123";

    vi.mocked(usersApi.createUser).mockRejectedValueOnce(
      new ApiError(400, {
        EMAIL_INVAILD: "Please enter a valid email address.",
      })
    );

    await c.save();

    expect(c.fieldErrors.value.email).toBe("Please enter a valid email address.");
    expect(c.error.value).toBeNull(); // Did not leak to background page
    expect(c.showCreate.value).toBe(true); // Modal stayed open
  });

  it("maps password errors to fieldErrors.password", async () => {
    const c = useUsersController();
    c.openCreate();
    c.email.value = "valid@example.com";
    c.password.value = "validpassword123";

    vi.mocked(usersApi.createUser).mockRejectedValueOnce(
      new ApiError(400, {
        PASSWORD_TOO_SHORT: "Password must contain at least 12 characters.",
      })
    );

    await c.save();

    expect(c.fieldErrors.value.password).toBe("Password must contain at least 12 characters.");
    expect(c.error.value).toBeNull();
  });

  it("maps unhandled server errors to formError inside the modal", async () => {
    const c = useUsersController();
    c.openCreate();
    c.email.value = "valid@example.com";
    c.password.value = "validpassword123";

    vi.mocked(usersApi.createUser).mockRejectedValueOnce(
      new ApiError(500, {
        INTERNAL_ERROR: "Database write failed.",
      })
    );

    await c.save();

    expect(c.formError.value).toBe("Database write failed.");
    expect(c.error.value).toBeNull();
  });

  it("resets form errors on close and openCreate", () => {
    const c = useUsersController();
    c.openCreate();
    c.fieldErrors.value = { email: "Error" };
    c.formError.value = "Form error";

    c.close();
    expect(c.fieldErrors.value).toEqual({});
    expect(c.formError.value).toBeNull();
  });

  it("handles user delete flow via confirmation dialog without browser dialog", async () => {
    window.confirm = vi.fn();
    const c = useUsersController();

    const targetUser: usersApi.User = {
      id: "usr_to_delete",
      email: "delete-me@example.com",
      role: "member",
      createdAt: "2026-08-01T00:00:00Z",
      updatedAt: "2026-08-01T00:00:00Z",
    };

    // Prompt sets target
    c.promptDelete(targetUser);
    expect(c.userToDelete.value).toEqual(targetUser);
    expect(c.deleteUserError.value).toBeNull();

    // Close cancels
    c.closeDeleteUser();
    expect(c.userToDelete.value).toBeNull();

    // Prompt again and confirm
    c.promptDelete(targetUser);
    vi.mocked(usersApi.deleteUser).mockResolvedValueOnce(undefined);

    await c.confirmDeleteUser();

    expect(usersApi.deleteUser).toHaveBeenCalledWith("usr_to_delete");
    expect(c.userToDelete.value).toBeNull();
    expect(window.confirm).not.toHaveBeenCalled();
  });

  it("surfaces delete user error inside confirmation dialog", async () => {
    const c = useUsersController();
    const targetUser: usersApi.User = {
      id: "usr_fail",
      email: "fail@example.com",
      role: "member",
      createdAt: "",
      updatedAt: "",
    };

    c.promptDelete(targetUser);
    vi.mocked(usersApi.deleteUser).mockRejectedValueOnce(
      new ApiError(400, { ERROR: "Cannot delete user." })
    );

    await c.confirmDeleteUser();

    expect(c.deleteUserError.value).toBe("Cannot delete user.");
    expect(c.userToDelete.value).toEqual(targetUser); // Dialog stays open
  });

  it("handles AI agent delete flow via confirmation dialog without browser dialog", async () => {
    window.confirm = vi.fn();
    const c = useUsersController();

    const targetAgent: serviceAccountsApi.ServiceAccount = {
      id: "sa_to_delete",
      name: "build-agent",
      role: "ai_agent",
      createdAt: "2026-08-01T00:00:00Z",
      updatedAt: "2026-08-01T00:00:00Z",
    };

    // Prompt sets target
    c.promptDeleteAgent(targetAgent);
    expect(c.agentToDelete.value).toEqual(targetAgent);
    expect(c.deleteAgentError.value).toBeNull();

    // Close cancels
    c.closeDeleteAgent();
    expect(c.agentToDelete.value).toBeNull();

    // Prompt again and confirm
    c.promptDeleteAgent(targetAgent);
    vi.mocked(serviceAccountsApi.deleteServiceAccount).mockResolvedValueOnce(undefined);

    await c.confirmDeleteAgent();

    expect(serviceAccountsApi.deleteServiceAccount).toHaveBeenCalledWith("sa_to_delete");
    expect(c.agentToDelete.value).toBeNull();
    expect(window.confirm).not.toHaveBeenCalled();
  });

  it("surfaces delete AI agent error inside confirmation dialog", async () => {
    const c = useUsersController();
    const targetAgent: serviceAccountsApi.ServiceAccount = {
      id: "sa_fail",
      name: "failing-agent",
      role: "ai_agent",
      createdAt: "",
      updatedAt: "",
    };

    c.promptDeleteAgent(targetAgent);
    vi.mocked(serviceAccountsApi.deleteServiceAccount).mockRejectedValueOnce(
      new ApiError(400, { ERROR: "Cannot delete AI agent." })
    );

    await c.confirmDeleteAgent();

    expect(c.deleteAgentError.value).toBe("Cannot delete AI agent.");
    expect(c.agentToDelete.value).toEqual(targetAgent); // Dialog stays open
  });

  it("handles AI agent token generation flow with reauth, refresh, and flash modal", async () => {
    const c = useUsersController();
    const targetAgent: serviceAccountsApi.ServiceAccount = {
      id: "sa_agent_1",
      name: "indexer-agent",
      role: "ai_agent",
      createdAt: "2026-08-01T00:00:00Z",
      updatedAt: "2026-08-01T00:00:00Z",
    };

    // 1. Prompt get token
    c.promptGetToken(targetAgent);
    expect(c.showTokenReauth.value).toBe(true);
    expect(c.agentForToken.value).toEqual(targetAgent);
    expect(c.tokenPassword.value).toBe("");
    expect(c.tokenPasswordError.value).toBeNull();
    expect(c.tokenReauthError.value).toBeNull();

    // 2. Close cancels
    c.closeTokenReauth();
    expect(c.showTokenReauth.value).toBe(false);
    expect(c.agentForToken.value).toBeNull();

    // 3. Prompt again and test validation when password is empty
    c.promptGetToken(targetAgent);
    await c.confirmTokenReauthAndGenerate();
    expect(c.tokenPasswordError.value).toBe("Password is required.");

    // 4. Test reauth failure with invalid password
    c.tokenPassword.value = "wrong-password";
    vi.mocked(authApi.reauthenticate).mockRejectedValueOnce(
      new ApiError(403, { ERROR: "Incorrect password." })
    );
    await c.confirmTokenReauthAndGenerate();
    expect(c.tokenReauthError.value).toBe("Incorrect password.");
    expect(c.showTokenReauth.value).toBe(true);
    expect(c.createdAgentToken.value).toBeNull();

    // 5. Test successful reauth, token refresh (revoke old active token), and creation
    c.tokenPassword.value = "correct-password";
    vi.mocked(authApi.reauthenticate).mockResolvedValueOnce(undefined);
    vi.mocked(serviceAccountsApi.fetchAgentTokens).mockResolvedValueOnce([
      {
        id: "ait_old_active",
        serviceAccountId: "sa_agent_1",
        name: "token-1",
        createdAt: "2026-08-01T00:00:00Z",
        expiresAt: "2026-08-31T00:00:00Z",
        lastUsedAt: null,
        revokedAt: null,
      },
      {
        id: "ait_old_revoked",
        serviceAccountId: "sa_agent_1",
        name: "token-0",
        createdAt: "2026-07-01T00:00:00Z",
        expiresAt: "2026-07-31T00:00:00Z",
        lastUsedAt: null,
        revokedAt: "2026-07-15T00:00:00Z",
      },
    ]);
    vi.mocked(serviceAccountsApi.revokeAgentToken).mockResolvedValueOnce({
      id: "ait_old_active",
      serviceAccountId: "sa_agent_1",
      name: "token-1",
      createdAt: "2026-08-01T00:00:00Z",
      expiresAt: "2026-08-31T00:00:00Z",
      lastUsedAt: null,
      revokedAt: "2026-08-02T00:00:00Z",
    });
    const createdResult: serviceAccountsApi.CreatedAgentToken = {
      token: {
        id: "ait_new",
        serviceAccountId: "sa_agent_1",
        name: "token-new",
        createdAt: "2026-08-02T00:00:00Z",
        expiresAt: "2026-09-01T00:00:00Z",
        lastUsedAt: null,
        revokedAt: null,
      },
      plaintextToken: "dpa_abcdef1234567890",
    };
    vi.mocked(serviceAccountsApi.createAgentToken).mockResolvedValueOnce(createdResult);

    await c.confirmTokenReauthAndGenerate();

    expect(authApi.reauthenticate).toHaveBeenCalledWith("correct-password");
    expect(serviceAccountsApi.fetchAgentTokens).toHaveBeenCalledWith("sa_agent_1");
    // Only the unrevoked token should be revoked
    expect(serviceAccountsApi.revokeAgentToken).toHaveBeenCalledTimes(1);
    expect(serviceAccountsApi.revokeAgentToken).toHaveBeenCalledWith("sa_agent_1", "ait_old_active");
    expect(serviceAccountsApi.createAgentToken).toHaveBeenCalledWith(
      "sa_agent_1",
      expect.stringMatching(/^token-\d+$/)
    );
    expect(c.showTokenReauth.value).toBe(false);
    expect(c.createdAgentToken.value).toEqual(createdResult);

    // 6. Acknowledge created token
    c.acknowledgeCreatedToken();
    expect(c.createdAgentToken.value).toBeNull();
    expect(c.agentForToken.value).toBeNull();
  });
});
