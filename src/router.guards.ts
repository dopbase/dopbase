import type { Router } from "vue-router";
import type { NavigationGuardWithThis } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";
import { canOpen } from "~/navigation/access-map";

/**
 * Installs the global navigation guards on the router.
 *
 * Order of decisions on every navigation:
 * 1. Resolve the public bootstrap status once (cheap GET, cached in store).
 * 2. `/setup` is only reachable while the server is uninitialized —
 *    including stale tabs after bootstrap closes.
 * 3. Protected routes require a session. An absent or expired cookie
 *    redirects to `/login` with the intended destination preserved.
 * 4. A page the account can't open, per the capability map, renders the
 *    No access screen at the same address instead of redirecting.
 * 5. Signed-in admins never see `/login` again.
 */
export function installRouterGuards(router: Router): void {
  const guard: NavigationGuardWithThis<undefined> = async (to) => {
    const auth = useAuthStore();
    if (auth.bootstrapState === "unknown") {
      await auth.loadBootstrapStatus();
    }

    if (to.meta.setupOnly && auth.bootstrapState !== "setupRequired") {
      return { name: "projects" };
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
      // Detail routes are gated by the page they belong to.
      if (!canOpen(to.meta.page ?? to.name, auth.can)) {
        return {
          name: "no-access",
          params: { pathMatch: to.path.slice(1).split("/") },
          query: to.query,
          hash: to.hash,
        };
      }
    }

    if (to.meta.guestOnly && auth.isAuthenticated) {
      return { name: "projects" };
    }
    return true;
  };

  router.beforeEach(guard);
}
