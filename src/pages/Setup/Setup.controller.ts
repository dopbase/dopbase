import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
export const MIN_PASSWORD_LENGTH = 12;

/**
 * First-run setup controller: claims the uninitialized server with the
 * one-time setup token, creates the single admin, and starts the returned
 * session. Validation errors map back to their fields.
 */
export function useSetupController() {
  const router = useRouter();
  const auth = useAuthStore();

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

  return {
    setupToken,
    email,
    password,
    confirmPassword,
    passwordsMatch,
    fieldErrors,
    formError,
    submitting,
    submit,
  };
}
