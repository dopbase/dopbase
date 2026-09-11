import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";
import { formatDateTime, formatRelativeTime } from "~/utils/format";
import {
  MAX_PASSWORD_LENGTH,
  MIN_PASSWORD_LENGTH,
  validatePasswordLength,
} from "~/utils/validation";

/**
 * Account screen controller: session display and password rotation.
 *
 * Changing the password revokes every human session server-side. The
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
  const role = computed(() => auth.session?.role ?? "admin");
  const lastLoginAt = computed(() => auth.session?.lastLoginAt ?? null);

  const formattedRole = computed(() => {
    switch (role.value) {
      case "root":
        return "Root Administrator";
      case "admin":
        return "Administrator";
      case "member":
        return "Member";
      case "ai_agent":
        return "AI Agent";
      default:
        return role.value;
    }
  });

  const formattedLastLogin = computed(() => {
    if (!lastLoginAt.value) return null;
    return formatRelativeTime(lastLoginAt.value);
  });

  const fullLastLogin = computed(() => {
    if (!lastLoginAt.value) return "";
    return formatDateTime(lastLoginAt.value);
  });

  const recentAuthentication = computed(
    () => auth.session?.recentAuthentication ?? false,
  );

  function validate(): boolean {
    const errors: typeof fieldErrors.value = {};
    if (currentPassword.value === "") {
      errors.currentPassword = "Enter your current password.";
    }
    const passwordResult = validatePasswordLength(newPassword.value);
    if (passwordResult.code === "PASSWORD_TOO_SHORT") {
      errors.newPassword = `Use at least ${MIN_PASSWORD_LENGTH} characters.`;
    } else if (passwordResult.code === "PASSWORD_TOO_LONG") {
      errors.newPassword = `Use at most ${MAX_PASSWORD_LENGTH} characters.`;
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
    role,
    formattedRole,
    lastLoginAt,
    formattedLastLogin,
    fullLastLogin,
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
