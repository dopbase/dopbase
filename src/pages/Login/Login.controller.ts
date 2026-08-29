import { ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

/**
 * Login screen controller: form state, client-side validation, stable
 * error-code mapping, and post-login routing back to the intended
 * destination.
 */
export function useLoginController() {
  const router = useRouter();
  const route = useRoute();
  const auth = useAuthStore();

  const email = ref("");
  const password = ref("");
  const fieldErrors = ref<{ email?: string; password?: string }>({});
  const formError = ref<string | null>(null);
  const submitting = ref(false);

  function validate(): boolean {
    const errors: { email?: string; password?: string } = {};
    if (email.value.trim() === "") {
      errors.email = "Enter your email address.";
    } else if (!EMAIL_PATTERN.test(email.value.trim())) {
      errors.email = "Enter a valid email address.";
    }
    if (password.value === "") {
      errors.password = "Enter your password.";
    }
    fieldErrors.value = errors;
    return Object.keys(errors).length === 0;
  }

  /** Branches on stable server codes; everything else stays generic. */
  function mapApiError(error: unknown): void {
    if (error instanceof ApiError) {
      if (error.hasCode("EMAIL_INVAILD")) {
        fieldErrors.value.email = "Enter a valid email address.";
        return;
      }
      if (error.hasCode("RATE_LIMITED")) {
        formError.value =
          "Too many login attempts. Wait a moment and try again.";
        return;
      }
    }
    // AUTHENTICATION_INVALID and unknown failures get one generic message —
    // the server deliberately does not reveal whether the email exists.
    formError.value = "The email or password is incorrect.";
  }

  async function submit(): Promise<void> {
    formError.value = null;
    if (!validate() || submitting.value) return;
    submitting.value = true;
    try {
      await auth.login(email.value.trim().toLowerCase(), password.value);
      const redirect = route.query.redirect;
      router.push(
        typeof redirect === "string" ? redirect : { name: "workspace" },
      );
    } catch (error) {
      mapApiError(error);
    } finally {
      submitting.value = false;
    }
  }

  return {
    email,
    password,
    fieldErrors,
    formError,
    submitting,
    submit,
  };
}
