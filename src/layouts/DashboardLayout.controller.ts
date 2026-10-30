import { computed, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";
import { modalsSuspended } from "~/composable/modal-suspension";
import { isPageName, visibleSections } from "~/navigation/access-map";
import type { PageName } from "~/navigation/access-map";
import { formatRole } from "~/utils/format";

/**
 * DashboardLayout controller: the sidebar sections this account can open,
 * the account menu (email, role name, which links it offers) and the
 * logout action. Logout always routes back to the login
 * screen — even when the server call fails — because the auth store has
 * already cleared the local session by the time the error surfaces.
 *
 * Below the desktop breakpoint the sidebar folds into a sheet; `navOpen`
 * tracks it and closes on every route change so a tapped link lands on
 * the new screen rather than behind the open sheet.
 */
export function useDashboardLayoutController() {
  const router = useRouter();
  const route = useRoute();
  const auth = useAuthStore();

  const email = computed(() => auth.session?.email ?? "");
  const roleName = computed(() =>
    auth.session ? auth.session.roleName || formatRole(auth.session.role) : "",
  );
  const sections = computed(() => visibleSections(auth.can));
  // Every page in the map lives at `/<name>`, so the first path segment
  // names the sidebar item to highlight.
  const activePage = computed<PageName | null>(() => {
    const segment = route.path.split("/")[1];
    return isPageName(segment) ? segment : null;
  });
  // Root has no personal tokens; the Account page hides the card for root.
  const showPersonalTokens = computed(
    () => !auth.isRoot && auth.can("tokens.personal"),
  );
  const instanceHost = window.location.host;
  const sessionExpired = computed(() => auth.sessionExpired);
  const loggingOut = ref(false);
  const navOpen = ref(false);

  // Close every other dialog while "Session expired" is up.
  watch(
    sessionExpired,
    (expired) => {
      modalsSuspended.value = expired;
    },
    { immediate: true },
  );
  onUnmounted(() => {
    modalsSuspended.value = false;
  });

  watch(
    () => route.fullPath,
    () => {
      navOpen.value = false;
    },
  );

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
    roleName,
    sections,
    activePage,
    showPersonalTokens,
    instanceHost,
    sessionExpired,
    loggingOut,
    navOpen,
    logout,
    signInAgain,
  };
}
