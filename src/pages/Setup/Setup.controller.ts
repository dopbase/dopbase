import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { ApiError } from "~/services/http.client";
import { bootstrapRestore } from "~/services/bootstrap.api";
import { useAuthStore } from "~/stores/auth.store";

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
export const MIN_PASSWORD_LENGTH = 12;

export type SetupMode = "setup" | "restore";

/**
 * First-run setup controller: claims the uninitialized server with the
 * one-time setup token, creates the single admin, and starts the returned
 * session; OR restores the entire instance from an encrypted .dop backup.
 */
export function useSetupController() {
  const router = useRouter();
  const auth = useAuthStore();

  const mode = ref<SetupMode>("setup");

  // Setup form state
  const setupToken = ref("");
  const email = ref("");
  const password = ref("");
  const confirmPassword = ref("");

  const fieldErrors = ref<{
    setupToken?: string;
    email?: string;
    password?: string;
    confirmPassword?: string;
  }>({});
  const formError = ref<string | null>(null);
  const submitting = ref(false);

  // Restore form state
  const selectedFile = ref<File | null>(null);
  const masterKeyFile = ref<File | null>(null);
  const masterKeyHex = ref("");
  const restoring = ref(false);
  const restoreError = ref<string | null>(null);

  const passwordsMatch = computed(
    () =>
      confirmPassword.value === "" || password.value === confirmPassword.value,
  );

  function validate(): boolean {
    const errors: typeof fieldErrors.value = {};
    if (setupToken.value.trim() === "") {
      errors.setupToken = "Paste the setup token printed by the server.";
    }
    if (email.value.trim() === "") {
      errors.email = "Enter an email address.";
    } else if (!EMAIL_PATTERN.test(email.value.trim())) {
      errors.email = "Enter a valid email address.";
    }
    if (password.value.length < MIN_PASSWORD_LENGTH) {
      errors.password = `Use at least ${MIN_PASSWORD_LENGTH} characters.`;
    } else if (password.value.length > 128) {
      errors.password = "Use at most 128 characters.";
    }
    if (confirmPassword.value !== password.value) {
      errors.confirmPassword = "Passwords do not match.";
    }
    fieldErrors.value = errors;
    return Object.keys(errors).length === 0;
  }

  function mapApiError(error: unknown): void {
    if (error instanceof ApiError) {
      if (error.hasCode("EMAIL_INVAILD")) {
        fieldErrors.value.email = "Enter a valid email address.";
        return;
      }
      if (error.hasCode("RATE_LIMITED")) {
        formError.value = "Too many attempts. Wait a moment and try again.";
        return;
      }
      if (error.firstCode === "SETUP_TOKEN_INVALID") {
        fieldErrors.value.setupToken =
          "The setup token is invalid or already used.";
        return;
      }
      if (error.status === 409) {
        formError.value =
          "This server has already been set up. Sign in instead.";
        return;
      }
    }
    formError.value = "Setup failed. Check the token and try again.";
  }

  async function submit(): Promise<void> {
    formError.value = null;
    if (!validate() || submitting.value) return;
    submitting.value = true;
    try {
      await auth.bootstrap({
        setupToken: setupToken.value.trim(),
        email: email.value.trim().toLowerCase(),
        password: password.value,
      });
      router.push({ name: "workspace" });
    } catch (error) {
      mapApiError(error);
    } finally {
      submitting.value = false;
    }
  }

  function onFileSelected(file: File | null): void {
    restoreError.value = null;
    if (!file) {
      selectedFile.value = null;
      return;
    }
    if (!file.name.endsWith(".dop")) {
      restoreError.value = "Please select a valid .dop backup file.";
      selectedFile.value = null;
      return;
    }
    selectedFile.value = file;
  }

  function onMasterKeyFileSelected(file: File | null): void {
    restoreError.value = null;
    masterKeyFile.value = file;
    if (file) {
      masterKeyHex.value = "";
    }
  }

  async function submitRestore(): Promise<void> {
    restoreError.value = null;
    if (!selectedFile.value) {
      restoreError.value = "Please select a .dop backup file to restore.";
      return;
    }
    if (!setupToken.value.trim()) {
      restoreError.value = "Enter the setup token printed by the server.";
      return;
    }
    if (restoring.value) return;

    restoring.value = true;
    try {
      const keyInput =
        masterKeyFile.value ||
        (masterKeyHex.value.trim() ? masterKeyHex.value.trim() : undefined);
      if (keyInput) {
        await bootstrapRestore(selectedFile.value, setupToken.value, keyInput);
      } else {
        await bootstrapRestore(selectedFile.value, setupToken.value);
      }
      await auth.loadBootstrapStatus();
      router.push({ name: "login", query: { notice: "backup-restored" } });
    } catch (error) {
      if (error instanceof ApiError) {
        if (error.hasCode("BACKUP_DECRYPT_FAILED")) {
          restoreError.value =
            "Failed to decrypt backup. When restoring from another server, provide its master key (master.key file or 64-character hex key).";
          return;
        }
        if (error.hasCode("INVALID_MASTER_KEY")) {
          restoreError.value =
            "The provided master key is invalid. It must be a 32-byte master.key file or 64-character hex key.";
          return;
        }
        if (error.status === 409 || error.hasCode("BOOTSTRAP_CLOSED")) {
          restoreError.value =
            "This server has already been set up. Please sign in instead.";
          return;
        }
        if (error.hasCode("RATE_LIMITED")) {
          restoreError.value =
            "Too many attempts. Wait a moment and try again.";
          return;
        }
        restoreError.value = error.message || "Failed to restore backup.";
      } else {
        restoreError.value = "An unexpected error occurred while restoring.";
      }
    } finally {
      restoring.value = false;
    }
  }

  return {
    mode,
    setupToken,
    email,
    password,
    confirmPassword,
    passwordsMatch,
    fieldErrors,
    formError,
    submitting,
    submit,
    selectedFile,
    masterKeyFile,
    masterKeyHex,
    restoring,
    restoreError,
    onFileSelected,
    onMasterKeyFileSelected,
    submitRestore,
  };
}
