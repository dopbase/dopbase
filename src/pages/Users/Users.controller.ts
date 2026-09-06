import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { ApiError } from "~/services/http.client";
import {
  createUser,
  deleteUser,
  fetchUsers,
  updateUser,
} from "~/services/users.api";
import type { User } from "~/services/users.api";
import { useAuthStore } from "~/stores/auth.store";
import {
  createAgentToken,
  createServiceAccount,
  deleteServiceAccount,
  fetchAgentTokens,
  fetchServiceAccounts,
  revokeAgentToken,
} from "~/services/service-accounts.api";
import type {
  CreatedAgentToken,
  ServiceAccount,
} from "~/services/service-accounts.api";
import {
  isValidEmail,
  MAX_PASSWORD_LENGTH,
  MIN_PASSWORD_LENGTH,
  validatePasswordLength,
} from "~/utils/validation";

export const roleOptions = [
  { value: "member", label: "Member - project management" },
  { value: "admin", label: "Admin - user management" },
];

export function useUsersController() {
  const router = useRouter();
  const authStore = useAuthStore();
  const users = ref<User[]>([]);
  const serviceAccounts = ref<ServiceAccount[]>([]);
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
  const agentForToken = ref<ServiceAccount | null>(null);
  const showTokenReauth = ref(false);
  const tokenPassword = ref("");
  const tokenPasswordError = ref<string | null>(null);
  const tokenReauthError = ref<string | null>(null);
  const generatingToken = ref(false);
  const createdAgentToken = ref<CreatedAgentToken | null>(null);

  // Delete user modal state
  const userToDelete = ref<User | null>(null);
  const deletingUser = ref(false);
  const deleteUserError = ref<string | null>(null);

  // Delete agent modal state
  const agentToDelete = ref<ServiceAccount | null>(null);
  const deletingAgent = ref(false);
  const deleteAgentError = ref<string | null>(null);

  const canSave = computed(() => {
    const trimmedEmail = email.value.trim();
    if (!trimmedEmail || !role.value) return false;
    if (!editing.value && !password.value) return false;
    return true;
  });

  const canSaveAgent = computed(() => agentName.value.trim().length > 0);

  async function load(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const [u, sa] = await Promise.all([fetchUsers(), fetchServiceAccounts()]);
      users.value = u;
      serviceAccounts.value = sa;
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
        errors.password = "Password is required.";
      } else if (validatePasswordLength(password.value).code === "PASSWORD_TOO_SHORT") {
        errors.password = `Password must contain at least ${MIN_PASSWORD_LENGTH} characters.`;
      } else if (validatePasswordLength(password.value).code === "PASSWORD_TOO_LONG") {
        errors.password = `Password must contain at most ${MAX_PASSWORD_LENGTH} characters.`;
      }
    } else if (password.value) {
      if (validatePasswordLength(password.value).code === "PASSWORD_TOO_SHORT") {
        errors.password = `Password must contain at least ${MIN_PASSWORD_LENGTH} characters.`;
      } else if (validatePasswordLength(password.value).code === "PASSWORD_TOO_LONG") {
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
        if (
          err.hasCode("EMAIL_INVAILD") ||
          err.hasCode("EMAIL_INVALID") ||
          err.hasCode("EMAIL_TAKEN")
        ) {
          fieldErrors.value.email =
            err.codes.EMAIL_INVAILD ??
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
      await createServiceAccount(trimmed);
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

  function promptDeleteAgent(account: ServiceAccount): void {
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
      await deleteServiceAccount(agentToDelete.value.id);
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

  function promptGetToken(agent: ServiceAccount): void {
    agentForToken.value = agent;
    tokenPassword.value = "";
    tokenPasswordError.value = null;
    tokenReauthError.value = null;
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

    if (!tokenPassword.value) {
      tokenPasswordError.value = "Password is required.";
      return;
    }

    generatingToken.value = true;
    try {
      await authStore.reauthenticate(tokenPassword.value);
    } catch (err) {
      tokenReauthError.value =
        err instanceof ApiError ? err.message : "The password is incorrect.";
      generatingToken.value = false;
      return;
    }

    try {
      const agentId = agentForToken.value.id;
      // Refresh token: revoke any existing active tokens for this agent
      const existingTokens = await fetchAgentTokens(agentId);
      const activeTokens = existingTokens.filter((t) => !t.revokedAt);
      for (const t of activeTokens) {
        await revokeAgentToken(agentId, t.id);
      }

      // Generate a new 30-day token
      const tokenName = `token-${Date.now()}`;
      const created = await createAgentToken(agentId, tokenName);

      createdAgentToken.value = created;
      tokenPassword.value = "";
      showTokenReauth.value = false;
    } catch (err) {
      tokenReauthError.value =
        err instanceof ApiError
          ? err.message
          : "Could not generate agent token.";
    } finally {
      generatingToken.value = false;
    }
  }

  function acknowledgeCreatedToken(): void {
    createdAgentToken.value = null;
    agentForToken.value = null;
  }

  onMounted(load);

  return {
    users,
    serviceAccounts,
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
    openCreate,
    openEdit,
    save,
    remove: promptDelete,
    promptDelete,
    userToDelete,
    deletingUser,
    deleteUserError,
    closeDeleteUser,
    confirmDeleteUser,
    close,
    router,
    showAgentCreate,
    agentName,
    savingAgent,
    agentFormError,
    agentFieldError,
    canSaveAgent,
    openAgentCreate,
    closeAgentCreate,
    saveAgent,
    removeAgent: promptDeleteAgent,
    promptDeleteAgent,
    agentToDelete,
    deletingAgent,
    deleteAgentError,
    closeDeleteAgent,
    confirmDeleteAgent,
    agentForToken,
    showTokenReauth,
    tokenPassword,
    tokenPasswordError,
    tokenReauthError,
    generatingToken,
    createdAgentToken,
    promptGetToken,
    closeTokenReauth,
    confirmTokenReauthAndGenerate,
    acknowledgeCreatedToken,
  };
}
