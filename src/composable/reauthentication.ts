import { ref } from "vue";
import { onReauthenticationRequired } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";

/**
 * Global reauthentication flow shared by every screen that touches
 * reveal/export.
 *
 * `runWithReauth` executes an action; when the server answers 403
 * `RECENT_AUTHENTICATION_REQUIRED`, the action is parked, the password
 * dialog opens (via the {@link onReauthenticationRequired} event), and after
 * a successful `reauthenticate` the parked action runs again exactly once.
 * The dialog component itself lives in `components/app/ReauthModal.vue`,
 * mounted once inside `DashboardLayout`.
 */
const isOpen = ref(false);
const error = ref<string | null>(null);
const submitting = ref(false);

let pendingAction: (() => Promise<void>) | null = null;

export function useReauthentication() {
  const auth = useAuthStore();

  onReauthenticationRequired(() => {
    pendingAction = null;
    error.value = null;
    isOpen.value = true;
  });

  async function submit(password: string): Promise<boolean> {
    submitting.value = true;
    error.value = null;
    try {
      await auth.reauthenticate(password);
      isOpen.value = false;
      const action = pendingAction;
      pendingAction = null;
      if (action) await action();
      return true;
    } catch (cause) {
      error.value =
        cause instanceof Error && cause.message
          ? "The password is incorrect."
          : "The password is incorrect.";
      return false;
    } finally {
      submitting.value = false;
    }
  }

  function dismiss(): void {
    if (submitting.value) return;
    isOpen.value = false;
    error.value = null;
    pendingAction = null;
  }

  /**
   * Runs `action`, parking it for a retry when a reauthentication is
   * demanded. Errors other than the 403 reauth challenge are rethrown so
   * callers keep their normal error handling.
   */
  async function runWithReauth(action: () => Promise<void>): Promise<void> {
    try {
      await action();
    } catch (cause) {
      const isReauthChallenge =
        cause instanceof Object &&
        "status" in cause &&
        (cause as { status: number }).status === 403 &&
        "hasCode" in cause &&
        (cause as { hasCode: (c: string) => boolean }).hasCode(
          "RECENT_AUTHENTICATION_REQUIRED",
        );
      if (!isReauthChallenge) throw cause;
      pendingAction = action;
    }
  }

  return { isOpen, error, submitting, submit, dismiss, runWithReauth };
}
