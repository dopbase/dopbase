import { computed, onMounted, reactive, ref } from "vue";
import { ApiError } from "~/services/http.client";
import {
  createUser,
  deleteUser,
  fetchUsers,
  unlinkUserSso,
  updateUser,
} from "~/services/users.api";
import { fetchAuthMethods } from "~/services/auth.api";
import { fetchUserAccess } from "~/services/access.api";
import type { UserAccess } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import type { User } from "~/services/users.api";
import { useAuthStore } from "~/stores/auth.store";
import {
  createPersonalToken,
  listPersonalTokens,
  revokePersonalToken,
} from "~/services/personal-tokens.api";
import type {
  CreatedPersonalToken,
  PersonalToken,
} from "~/services/personal-tokens.api";
import type { Level } from "~/services/access.api";
import {
  isValidEmail,
  MAX_PASSWORD_LENGTH,
  MIN_PASSWORD_LENGTH,
  validatePasswordLength,
} from "~/utils/validation";
import { tokenExpiresIn, tokenExpiryUnits } from "~/utils/token-expiry";

/** AI agent tokens always expire, within 90 days. */
export const agentTokenExpiryOptions = [
  { value: "1d", label: "1 day" },
  { value: "7d", label: "7 days" },
  { value: "30d", label: "30 days" },
  { value: "90d", label: "90 days" },
  { value: "custom", label: "Custom" },
];

/** What an AI agent token can do, within the agent's own grants. */
export const agentTokenLevelOptions = [
  { value: "metadata", label: "Names only - list projects and secret keys" },
  { value: "read", label: "Read - also reveal values and run commands" },
];

// TODO(rbac): the fixed roles an admin can give out. List assignable roles
// from the server once roles are configurable.
export const roleOptions = [
  { value: "member", label: "Member - project management" },
  { value: "admin", label: "Admin - user management" },
];

export function useUsersController() {
  const authStore = useAuthStore();
  const { runWithReauth } = useReauthentication();
  const users = ref<User[]>([]);
  /** Accounts may skip the password only while single sign-on is on. */
  const ssoEnabled = ref(false);
  const unlinkingUser = ref<string | null>(null);
  /** Accounts without a usable password confirm through the shared dialog. */
  const tokenNeedsPassword = computed(
    () => authStore.session?.passwordConfirmation !== false,
  );
  /** AI agent accounts, listed apart from people. */
  const agents = ref<User[]>([]);
  const agentTokens = ref<Record<string, PersonalToken[] | null>>({});
  const loading = ref(false);
  const error = ref<string | null>(null);

  // User modal state
  const editing = ref<User | null>(null);
  const email = ref("");
  const password = ref("");
  const role = ref<"admin" | "member">("member");
  const showCreate = ref(false);
  const saving = ref(false);
  const formError = ref<string | null>(null);
  const fieldErrors = ref<{
    email?: string;
    password?: string;
    role?: string;
  }>({});

  // Agent modal state
  const showAgentCreate = ref(false);
  const agentName = ref("");
  const savingAgent = ref(false);
  const agentFormError = ref<string | null>(null);
  const agentFieldError = ref<string | null>(null);

  // Agent token generation state
  const agentForToken = ref<User | null>(null);
  const showTokenReauth = ref(false);
  const tokenPassword = ref("");
  const tokenPasswordError = ref<string | null>(null);
  const tokenReauthError = ref<string | null>(null);
  const tokenExpiryError = ref<string | null>(null);
  const tokenExpiryChoice = ref("30d");
  const tokenLevel = ref<Level>("metadata");
  const tokenCustomAmount = ref("");
  const tokenCustomUnit = ref("h");
  const generatingToken = ref(false);
  const createdAgentToken = ref<CreatedPersonalToken | null>(null);

  // Delete user modal state
  const userToDelete = ref<User | null>(null);
  const deletingUser = ref(false);
  const deleteUserError = ref<string | null>(null);

  // Delete agent modal state
  const agentToDelete = ref<User | null>(null);
  const deletingAgent = ref(false);
  const deleteAgentError = ref<string | null>(null);

  const canSave = computed(() => {
    const trimmedEmail = email.value.trim();
    if (!trimmedEmail || !role.value) return false;
    if (!editing.value && !password.value && !ssoEnabled.value) return false;
    return true;
  });

  const canSaveAgent = computed(() => agentName.value.trim().length > 0);

  /** Older servers lack the endpoint; then a password stays required. */
  async function loadSsoState(): Promise<void> {
    try {
      ssoEnabled.value = (await fetchAuthMethods())?.sso != null;
    } catch {
      ssoEnabled.value = false;
    }
  }

  async function load(): Promise<void> {
    void loadSsoState();
    loading.value = true;
    error.value = null;
    try {
      const accounts = await fetchUsers();
      const sa = accounts.filter((account) => account.role === "ai_agent");

      const tokenLists = await Promise.allSettled(
        sa.map((account) => listPersonalTokens(account.id)),
      );
      users.value = accounts.filter((account) => account.role !== "ai_agent");
      agents.value = sa;
      agentTokens.value = Object.fromEntries(
        sa.map((account, index) => {
          const result = tokenLists[index];
          return [
            account.id,
            result?.status === "fulfilled"
              ? result.value.filter((token) => !token.revokedAt)
              : null,
          ];
        }),
      );
    } catch {
      error.value = "Could not load accounts.";
    } finally {
      loading.value = false;
    }
  }

  function resetFormErrors(): void {
    formError.value = null;
    fieldErrors.value = {};
  }

  function openCreate(): void {
    resetFormErrors();
    editing.value = null;
    email.value = "";
    password.value = "";
    role.value = "member";
    showCreate.value = true;
  }

  function openEdit(user: User): void {
    resetFormErrors();
    editing.value = user;
    email.value = user.email;
    password.value = "";
    role.value = user.role === "member" ? "member" : "admin";
    showCreate.value = true;
  }

  function close(): void {
    resetFormErrors();
    showCreate.value = false;
  }

  function validateUserForm(): boolean {
    const errors: typeof fieldErrors.value = {};
    const trimmedEmail = email.value.trim();

    if (!trimmedEmail) {
      errors.email = "Email is required.";
    } else if (!isValidEmail(trimmedEmail)) {
      errors.email = "Please enter a valid email address.";
    }

    if (!editing.value) {
      if (!password.value) {
        if (!ssoEnabled.value) errors.password = "Password is required.";
      } else if (
        validatePasswordLength(password.value).code === "PASSWORD_TOO_SHORT"
      ) {
        errors.password = `Password must contain at least ${MIN_PASSWORD_LENGTH} characters.`;
      } else if (
        validatePasswordLength(password.value).code === "PASSWORD_TOO_LONG"
      ) {
        errors.password = `Password must contain at most ${MAX_PASSWORD_LENGTH} characters.`;
      }
    } else if (password.value) {
      if (
        validatePasswordLength(password.value).code === "PASSWORD_TOO_SHORT"
      ) {
        errors.password = `Password must contain at least ${MIN_PASSWORD_LENGTH} characters.`;
      } else if (
        validatePasswordLength(password.value).code === "PASSWORD_TOO_LONG"
      ) {
        errors.password = `Password must contain at most ${MAX_PASSWORD_LENGTH} characters.`;
      }
    }

    fieldErrors.value = errors;
    return Object.keys(errors).length === 0;
  }

  async function save(): Promise<void> {
    resetFormErrors();
    if (!validateUserForm()) return;

    saving.value = true;
    try {
      const trimmedEmail = email.value.trim();
      if (editing.value) {
        await updateUser(editing.value.id, {
          email: trimmedEmail,
          role: role.value,
          ...(password.value ? { password: password.value } : {}),
        });
      } else {
        await createUser(trimmedEmail, password.value, role.value);
      }
      showCreate.value = false;
      await load();
    } catch (err) {
      if (err instanceof ApiError) {
        let handled = false;
        if (err.hasCode("EMAIL_INVALID") || err.hasCode("EMAIL_TAKEN")) {
          fieldErrors.value.email =
            err.codes.EMAIL_INVALID ??
            err.codes.EMAIL_TAKEN ??
            "Please enter a valid email address.";
          handled = true;
        }
        if (
          err.hasCode("PASSWORD_TOO_SHORT") ||
          err.hasCode("PASSWORD_TOO_LONG")
        ) {
          fieldErrors.value.password =
            err.codes.PASSWORD_TOO_SHORT ??
            err.codes.PASSWORD_TOO_LONG ??
            `Password must contain at least ${MIN_PASSWORD_LENGTH} characters.`;
          handled = true;
        }
        if (!handled) {
          const lowerMsg = err.message.toLowerCase();
          if (lowerMsg.includes("email")) {
            fieldErrors.value.email = err.message;
          } else if (lowerMsg.includes("password")) {
            fieldErrors.value.password = err.message;
          } else {
            formError.value = err.message;
          }
        }
      } else {
        formError.value = "The user could not be saved.";
      }
    } finally {
      saving.value = false;
    }
  }

  async function unlinkSso(user: User): Promise<void> {
    error.value = null;
    unlinkingUser.value = user.id;
    try {
      await runWithReauth(async () => {
        await unlinkUserSso(user.id);
      });
      await load();
    } catch (err) {
      if (err instanceof DOMException && err.name === "AbortError") return;
      error.value =
        err instanceof ApiError
          ? err.message
          : "Single sign-on could not be unlinked.";
    } finally {
      unlinkingUser.value = null;
    }
  }

  function promptDelete(user: User): void {
    deleteUserError.value = null;
    userToDelete.value = user;
  }

  function closeDeleteUser(): void {
    if (deletingUser.value) return;
    userToDelete.value = null;
    deleteUserError.value = null;
  }

  async function confirmDeleteUser(): Promise<void> {
    if (!userToDelete.value) return;
    deletingUser.value = true;
    deleteUserError.value = null;
    try {
      await deleteUser(userToDelete.value.id);
      userToDelete.value = null;
      await load();
    } catch (err) {
      deleteUserError.value =
        err instanceof ApiError
          ? err.message
          : "The user could not be deleted.";
    } finally {
      deletingUser.value = false;
    }
  }

  function openAgentCreate(): void {
    agentName.value = "";
    agentFormError.value = null;
    agentFieldError.value = null;
    showAgentCreate.value = true;
  }

  function closeAgentCreate(): void {
    agentFormError.value = null;
    agentFieldError.value = null;
    showAgentCreate.value = false;
  }

  async function saveAgent(): Promise<void> {
    agentFormError.value = null;
    agentFieldError.value = null;
    const trimmed = agentName.value.trim();
    if (!trimmed) {
      agentFieldError.value = "Name is required.";
      return;
    }

    savingAgent.value = true;
    try {
      await createUser(trimmed, "", "ai_agent");
      showAgentCreate.value = false;
      await load();
    } catch (err) {
      agentFormError.value =
        err instanceof ApiError
          ? err.message
          : "The AI agent could not be created.";
    } finally {
      savingAgent.value = false;
    }
  }

  function promptDeleteAgent(account: User): void {
    deleteAgentError.value = null;
    agentToDelete.value = account;
  }

  function closeDeleteAgent(): void {
    if (deletingAgent.value) return;
    agentToDelete.value = null;
    deleteAgentError.value = null;
  }

  async function confirmDeleteAgent(): Promise<void> {
    if (!agentToDelete.value) return;
    deletingAgent.value = true;
    deleteAgentError.value = null;
    try {
      await deleteUser(agentToDelete.value.id);
      agentToDelete.value = null;
      await load();
    } catch (err) {
      deleteAgentError.value =
        err instanceof ApiError
          ? err.message
          : "The AI agent could not be deleted.";
    } finally {
      deletingAgent.value = false;
    }
  }

  function promptGetToken(agent: User): void {
    agentForToken.value = agent;
    tokenPassword.value = "";
    tokenPasswordError.value = null;
    tokenReauthError.value = null;
    tokenExpiryError.value = null;
    tokenExpiryChoice.value = "30d";
    tokenLevel.value = "metadata";
    tokenCustomAmount.value = "";
    tokenCustomUnit.value = "h";
    showTokenReauth.value = true;
  }

  function closeTokenReauth(): void {
    if (generatingToken.value) return;
    showTokenReauth.value = false;
    tokenPassword.value = "";
    tokenPasswordError.value = null;
    tokenReauthError.value = null;
    agentForToken.value = null;
  }

  async function confirmTokenReauthAndGenerate(): Promise<void> {
    if (!agentForToken.value) return;
    tokenPasswordError.value = null;
    tokenReauthError.value = null;
    tokenExpiryError.value = null;

    let expiresIn: string;
    try {
      expiresIn = tokenExpiresIn(
        tokenExpiryChoice.value,
        tokenCustomAmount.value,
        tokenCustomUnit.value,
      );
    } catch (error) {
      tokenExpiryError.value =
        error instanceof Error ? error.message : "Enter a valid expiry.";
      return;
    }

    if (tokenNeedsPassword.value && !tokenPassword.value) {
      tokenPasswordError.value = "Password is required.";
      return;
    }

    generatingToken.value = true;
    if (tokenNeedsPassword.value) {
      try {
        await authStore.reauthenticate(tokenPassword.value);
      } catch (err) {
        tokenReauthError.value =
          err instanceof ApiError ? err.message : "The password is incorrect.";
        generatingToken.value = false;
        return;
      }
    }

    try {
      const agentId = agentForToken.value.id;
      // Without a password, the shared dialog confirms at the provider when
      // the server asks, then this runs again.
      const created = await new Promise<CreatedPersonalToken>(
        (resolve, reject) => {
          runWithReauth(async () => {
            // Refresh token: revoke any existing active tokens for this agent
            const existingTokens = await listPersonalTokens(agentId);
            const activeTokens = existingTokens.filter((t) => !t.revokedAt);
            for (const t of activeTokens) {
              await revokePersonalToken(agentId, t.id);
            }
            agentTokens.value[agentId] = [];

            // Generate the replacement token with the selected lifetime.
            const tokenName = `token-${Date.now()}`;
            resolve(
              await createPersonalToken(agentId, {
                name: tokenName,
                level: tokenLevel.value,
                expiresIn,
              }),
            );
          }).catch(reject);
        },
      );

      createdAgentToken.value = created;
      agentTokens.value[agentId] = [created.token];
      tokenPassword.value = "";
      showTokenReauth.value = false;
    } catch (err) {
      if (err instanceof DOMException && err.name === "AbortError") return;
      tokenReauthError.value =
        err instanceof ApiError
          ? err.message
          : "Could not generate agent token.";
    } finally {
      generatingToken.value = false;
    }
  }

  // Access view state
  const accessUser = ref<User | null>(null);
  const userAccess = ref<UserAccess | null>(null);
  const accessError = ref<string | null>(null);

  async function openAccess(user: User): Promise<void> {
    accessUser.value = user;
    userAccess.value = null;
    accessError.value = null;
    try {
      userAccess.value = await fetchUserAccess(user.id);
    } catch (cause) {
      accessError.value = errorMessage(cause, "Could not load this access.");
    }
  }

  function closeAccess(): void {
    accessUser.value = null;
  }

  function acknowledgeCreatedToken(): void {
    createdAgentToken.value = null;
    agentForToken.value = null;
  }

  onMounted(load);

  const state = reactive({
    users,
    ssoEnabled,
    unlinkingUser,
    tokenNeedsPassword,
    agents,
    agentTokens,
    loading,
    error,
    editing,
    email,
    password,
    role,
    roleOptions,
    showCreate,
    saving,
    formError,
    fieldErrors,
    canSave,
    userToDelete,
    deletingUser,
    deleteUserError,
    showAgentCreate,
    agentName,
    savingAgent,
    agentFormError,
    agentFieldError,
    canSaveAgent,
    agentToDelete,
    deletingAgent,
    deleteAgentError,
    agentForToken,
    showTokenReauth,
    tokenPassword,
    tokenPasswordError,
    tokenReauthError,
    tokenExpiryError,
    tokenExpiryChoice,
    tokenCustomAmount,
    tokenCustomUnit,
    tokenExpiryOptions: agentTokenExpiryOptions,
    tokenLevel,
    tokenLevelOptions: agentTokenLevelOptions,
    tokenExpiryUnits,
    generatingToken,
    createdAgentToken,
    accessUser,
    userAccess,
    accessError,
  });

  const actions = {
    load,
    openCreate,
    openEdit,
    save,
    promptDelete,
    unlinkSso,
    closeDeleteUser,
    confirmDeleteUser,
    close,
    openAgentCreate,
    closeAgentCreate,
    saveAgent,
    promptDeleteAgent,
    closeDeleteAgent,
    confirmDeleteAgent,
    promptGetToken,
    closeTokenReauth,
    confirmTokenReauthAndGenerate,
    acknowledgeCreatedToken,
    openAccess,
    closeAccess,
  };

  return { state, actions };
}
