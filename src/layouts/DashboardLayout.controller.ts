import { computed, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";

/**
 * DashboardLayout controller: the signed-in email shown in the account
 * footer and the logout action. Logout always routes back to the login
 * screen — even when the server call fails — because the auth store has
 * already cleared the local session by the time the error surfaces.
 */
export function useDashboardLayoutController() {
  const router = useRouter();
  const route = useRoute();
  const auth = useAuthStore();

  const email = computed(() => auth.session?.email ?? "");
  const sessionExpired = computed(() => auth.sessionExpired);
  const loggingOut = ref(false);

  async function logout(): Promise<void> {
    if (loggingOut.value) return;
    loggingOut.value = true;
    try {
      await auth.logout();
    } catch {
      // The store already cleared local state (session + CSRF). The server
      // session is revoked or unreachable — either way, leave the dashboard.
    } finally {
      await router.push({ name: "login" });
      loggingOut.value = false;
    }
  }

  async function signInAgain(): Promise<void> {
    await router.replace({
      name: "login",
      query: { redirect: route.fullPath },
    });
  }

  return {
    email,
    sessionExpired,
    loggingOut,
    logout,
    signInAgain,
  };
}
