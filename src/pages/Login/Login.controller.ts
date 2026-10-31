import { computed, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { fetchAuthMethods, ssoStartUrl } from "~/services/auth.api";
import { fetchHealth } from "~/services/health.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";
import { ssoErrorMessage } from "~/utils/sso";
import { isSafeRedirect, isValidEmail } from "~/utils/validation";

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
  /** Button label for single sign-on, when the server has it enabled. */
  const ssoName = ref<string | null>(null);
  /** False until the server says which sign-in options it offers. */
  const methodsLoaded = ref(false);
  /** Root can hide the form; `/login?password` brings it back for root. */
  const passwordFormAllowed = ref(true);
  const showPasswordForm = computed(
    () => passwordFormAllowed.value || route.query.password !== undefined,
  );
  const ssoError = computed(() => ssoErrorMessage(route.query.sso_error));
  /** The server this page belongs to, so people know where they sign in. */
  const host = window.location.host;
  /** The server's version for the footer; empty until it answers. */
  const version = ref("");
  /** Caps Lock is on while the password field has focus. */
  const capsLock = ref(false);

  /** Reads Caps Lock from a key event in the password field. */
  function trackCapsLock(event: KeyboardEvent): void {
    capsLock.value = event.getModifierState?.("CapsLock") ?? false;
  }

  function safeRedirect(): string | null {
    const redirect = route.query.redirect;
    return typeof redirect === "string" && isSafeRedirect(redirect)
      ? redirect
      : null;
  }

  /** Full-page link that starts single sign-on and returns to the same place. */
  const ssoHref = computed(() => ssoStartUrl(safeRedirect() ?? undefined));

  async function loadMethods(): Promise<void> {
    void fetchHealth()
      .then((health) => {
        version.value = health.version;
      })
      .catch(() => undefined);
    try {
      const methods = await fetchAuthMethods();
      ssoName.value = methods.sso?.name ?? null;
      passwordFormAllowed.value = methods.passwordForm !== false;
    } catch {
      // Older servers have no methods endpoint; the password form still works.
      ssoName.value = null;
    } finally {
      methodsLoaded.value = true;
    }
  }

  function validate(): boolean {
    const errors: { email?: string; password?: string } = {};
    if (email.value.trim() === "") {
      errors.email = "Enter your email address.";
    } else if (!isValidEmail(email.value)) {
      errors.email = "Enter a valid email address.";
    }
    if (password.value === "") {
      errors.password = "Enter your password.";
    }
    fieldErrors.value = errors;
    return Object.keys(errors).length === 0;
  }

  /** Branches on stable server codes. Everything else stays generic. */
  function mapApiError(error: unknown): void {
    if (error instanceof ApiError) {
      if (error.hasCode("EMAIL_INVALID")) {
        fieldErrors.value.email = "Enter a valid email address.";
        return;
      }
      // Only sent after the password checked out, so naming the reason
      // reveals nothing.
      if (error.hasCode("PASSWORD_LOGIN_DISABLED")) {
        formError.value = ssoName.value
          ? `Password sign-in is turned off. Use "Sign in with ${ssoName.value}" above.`
          : "Password sign-in is turned off. Use single sign-on.";
        return;
      }
      // The server words these the same for an unknown email and a wrong
      // password, and says how many tries are left or how long to wait.
      if (error.hasCode("RATE_LIMITED")) {
        formError.value =
          error.message ||
          "Too many tries. Wait a few minutes, then try again.";
        return;
      }
      if (error.hasCode("AUTHENTICATION_INVALID") && error.message) {
        formError.value = error.message;
        return;
      }
    }
    formError.value = "Email or password is wrong.";
  }

  async function submit(): Promise<void> {
    formError.value = null;
    if (!validate() || submitting.value) return;
    submitting.value = true;
    try {
      await auth.login(email.value.trim().toLowerCase(), password.value);
      router.push(safeRedirect() ?? { name: "projects" });
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
    ssoName,
    methodsLoaded,
    showPasswordForm,
    ssoHref,
    ssoError,
    loadMethods,
    host,
    version,
    capsLock,
    trackCapsLock,
  };
}
