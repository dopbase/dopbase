import type { Router } from "vue-router";
import type { NavigationGuardWithThis } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";

/**
 * Installs the global navigation guards on the router.
 *
 * Order of decisions on every navigation:
 * 1. Resolve the public bootstrap status once (cheap GET, cached in store).
 * 2. `/setup` is only reachable while the server is uninitialized —
 *    including stale tabs after bootstrap closes.
 * 3. Protected routes require a session; an absent or expired cookie
 *    redirects to `/login` with the intended destination preserved.
 * 4. Signed-in admins never see `/login` again.
 */
export function installRouterGuards(router: Router): void {
  const guard: NavigationGuardWithThis<undefined> = async (to) => {
    const auth = useAuthStore();
    if (auth.bootstrapState === "unknown") {
      await auth.loadBootstrapStatus();
    }

    if (to.meta.setupOnly && auth.bootstrapState !== "setupRequired") {
      return { name: "workspace" };
    }

    if (!to.meta.public) {
      if (auth.bootstrapState === "setupRequired") {
        return { name: "setup" };
      }
      if (!auth.isAuthenticated) {
        try {
          await auth.fetchSession();
        } catch {
          return {
            name: "login",
            query: { redirect: to.fullPath },
          };
        }
      }
    }

    if (to.meta.guestOnly && auth.isAuthenticated) {
      return { name: "workspace" };
    }
    return true;
  };

  router.beforeEach(guard);
}
