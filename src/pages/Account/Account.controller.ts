import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";

export const MIN_PASSWORD_LENGTH = 12;

/**
 * Account screen controller: session display and password rotation.
 *
 * Changing the password revokes every human session server-side; the
 * controller clears local session state and routes back to `/login`.
 */
export function useAccountController() {
  const router = useRouter();
  const auth = useAuthStore();

  const currentPassword = ref("");
  const newPassword = ref("");
  const confirmPassword = ref("");

  const fieldErrors = ref<{
    currentPassword?: string;
    newPassword?: string;
    confirmPassword?: string;
  }>({});
  const formError = ref<string | null>(null);
  const submitting = ref(false);

  const email = computed(() => auth.session?.email ?? "");
  const recentAuthentication = computed(
    () => auth.session?.recentAuthentication ?? false,
  );

  function validate(): boolean {
    const errors: typeof fieldErrors.value = {};
    if (currentPassword.value === "") {
      errors.currentPassword = "Enter your current password.";
    }
    if (newPassword.value.length < MIN_PASSWORD_LENGTH) {
      errors.newPassword = `Use at least ${MIN_PASSWORD_LENGTH} characters.`;
    } else if (newPassword.value.length > 128) {
      errors.newPassword = "Use at most 128 characters.";
    }
    if (confirmPassword.value !== newPassword.value) {
      errors.confirmPassword = "Passwords do not match.";
    }
    fieldErrors.value = errors;
    return Object.keys(errors).length === 0;
  }

  function mapApiError(error: unknown): void {
    if (error instanceof ApiError) {
      if (error.status === 401) {
        fieldErrors.value.currentPassword =
          "The current password is incorrect.";
        return;
      }
      if (error.hasCode("REQUEST_INVALID")) {
        fieldErrors.value.newPassword = "Use 12–128 characters.";
        return;
      }
    }
    formError.value = "The password could not be changed. Try again.";
  }

  async function submit(): Promise<void> {
    formError.value = null;
    if (!validate() || submitting.value) return;
    submitting.value = true;
    try {
      await auth.changePassword(currentPassword.value, newPassword.value);
      router.push({
        name: "login",
        query: { notice: "password-changed" },
      });
    } catch (error) {
      mapApiError(error);
    } finally {
      submitting.value = false;
    }
  }

  return {
    email,
    recentAuthentication,
    currentPassword,
    newPassword,
    confirmPassword,
    fieldErrors,
    formError,
    submitting,
    submit,
  };
}
