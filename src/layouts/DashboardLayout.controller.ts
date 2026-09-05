import { computed } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";

/**
 * DashboardLayout controller: the signed-in email shown in the account
 * footer and the logout action. Logout always routes back to the login
 * screen — even when the server call fails — because the auth store has
 * already cleared the local session by the time the error surfaces.
 */
export function useDashboardLayoutController() {
  const router = useRouter();
  const auth = useAuthStore();

  const email = computed(() => auth.session?.email ?? "");

  async function logout(): Promise<void> {
    try {
      await auth.logout();
    } catch {
      // The store already cleared local state (session + CSRF); the server
      // session is revoked or unreachable — either way, leave the dashboard.
    } finally {
      await router.push({ name: "login" });
    }
  }

  return {
    email,
    logout,
  };
}
