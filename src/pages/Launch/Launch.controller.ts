import { onBeforeUnmount, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";
import { ApiError } from "~/services/http.client";

/**
 * Launch: `dopbase admin passcode` prints a link to this page with the
 * account and passcode in the URL fragment, which never reaches the server
 * or its logs. The controller wipes the address bar, trades the passcode for
 * a session, and opens Projects.
 *
 * Without a usable fragment there is nothing to launch: a signed-in browser
 * goes to Projects, anyone else to sign in. A query string is ignored on
 * purpose, so passcodes are never sent in a URL.
 *
 * When the server never answered, the passcode was not used, so the page
 * keeps it in memory and offers to send it again.
 *
 * Pasting a new link into a tab already on this page changes only the
 * fragment, which does not reload the page, so the controller also listens
 * for `hashchange`.
 */
export function useLaunchController() {
  const router = useRouter();
  const auth = useAuthStore();
  const error = ref<string | null>(null);
  /** True when the server never judged the passcode, so it may still work. */
  const canRetry = ref(false);
  // Kept after the address bar is wiped, so a failed request can be sent
  // again without the link.
  let pending: { identity: string; passcode: string } | null = null;

  function takeFragment(): { identity: string; passcode: string } | null {
    const params = new URLSearchParams(window.location.hash.slice(1));
    window.history.replaceState(
      window.history.state,
      "",
      window.location.pathname,
    );
    const identity = params.get("identity")?.trim();
    const passcode = params.get("passcode")?.trim();
    return identity && passcode ? { identity, passcode } : null;
  }

  async function fallBack(): Promise<void> {
    if (!auth.isAuthenticated) {
      try {
        await auth.fetchSession({ quiet: true });
      } catch {
        await router.replace({ name: "login" });
        return;
      }
    }
    await router.replace({ name: "projects" });
  }

  function explain(cause: unknown): string {
    const status = cause instanceof ApiError ? cause.status : 0;
    canRetry.value = status !== 401 && status !== 429;
    if (status === 401)
      return "This link has expired or was already used. Run dopbase admin passcode on the server for a new one.";
    if (status === 429)
      return "Too many tries for this account. Wait a few minutes, then run dopbase admin passcode on the server again.";
    return "The server didn't answer, so the link wasn't checked. It still works until it expires, so try again in a moment.";
  }

  async function attempt(): Promise<void> {
    if (!pending) return;
    error.value = null;
    canRetry.value = false;
    try {
      await auth.completeLaunch(pending.identity, pending.passcode);
      pending = null;
      await router.replace({ name: "projects" });
    } catch (cause) {
      error.value = explain(cause);
      if (!canRetry.value) pending = null;
    }
  }

  async function launch(): Promise<void> {
    error.value = null;
    pending = takeFragment();
    if (!pending) {
      canRetry.value = false;
      await fallBack();
      return;
    }
    await attempt();
  }

  function onHashChange(): void {
    // Wiping the fragment ourselves must not start another round.
    if (window.location.hash.length > 1) void launch();
  }

  onMounted(() => {
    window.addEventListener("hashchange", onHashChange);
    void launch();
  });
  onBeforeUnmount(() => {
    window.removeEventListener("hashchange", onHashChange);
  });

  return { error, canRetry, launch, retry: attempt };
}
