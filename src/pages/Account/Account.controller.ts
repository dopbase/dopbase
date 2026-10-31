import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter, type RouteLocationRaw } from "vue-router";
import { ApiError } from "~/services/http.client";
import { fetchUserAccess } from "~/services/access.api";
import type { GroupRef } from "~/services/access.api";
import { useAuthStore } from "~/stores/auth.store";
import { useReauthentication } from "~/composable/reauthentication";
import { formatDateTime, formatRelativeTime, formatRole } from "~/utils/format";
import {
  MAX_PASSWORD_LENGTH,
  MIN_PASSWORD_LENGTH,
  validatePasswordLength,
} from "~/utils/validation";

export type AccountTab = "profile" | "tokens" | "audit-tokens";

const TAB_ROUTES: Record<AccountTab, string> = {
  profile: "account",
  tokens: "account-tokens",
  "audit-tokens": "account-audit-tokens",
};
const TAB_FOR_ROUTE = Object.fromEntries(
  Object.entries(TAB_ROUTES).map(([tab, route]) => [route, tab]),
) as Record<string, AccountTab>;

/**
 * Account screen controller: tabs, the profile, and password rotation.
 *
 * The second tab depends on the role: Personal tokens, or Audit tokens
 * for roles whose tokens read the audit log. Root and roles without
 * personal tokens get the profile only. An address for the other tab
 * moves to the one this account has.
 *
 * Changing the password revokes every human session server-side. The
 * controller clears local session state and routes back to `/login`.
 */
export function useAccountController() {
  const route = useRoute();
  const router = useRouter();
  const auth = useAuthStore();
  const { runWithReauth } = useReauthentication();
  /** An account without a password sets one instead of changing it. */
  const needsCurrentPassword = computed(
    () => auth.session?.hasPassword !== false,
  );

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

  const formattedRole = computed(
    () => auth.session?.roleName || formatRole(role.value),
  );

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

  // Tabs

  /** The tokens tab this account has, if any. */
  const tokensTab = computed<AccountTab | null>(() => {
    if (auth.isRoot || !auth.can("tokens.personal")) return null;
    return auth.session?.tokenScope === "audit" ? "audit-tokens" : "tokens";
  });
  const activeTab = computed<AccountTab>(() => {
    const asked = TAB_FOR_ROUTE[String(route.name)] ?? "profile";
    return asked === "profile" ? asked : (tokensTab.value ?? "profile");
  });
  const tabs = computed(() => {
    const all: { key: AccountTab; label: string }[] = [
      { key: "profile", label: "Profile & sign-in" },
    ];
    if (tokensTab.value === "tokens")
      all.push({ key: "tokens", label: "Personal tokens" });
    if (tokensTab.value === "audit-tokens")
      all.push({ key: "audit-tokens", label: "Audit tokens" });
    return all.map((tab) => ({
      ...tab,
      to: { name: TAB_ROUTES[tab.key] } as RouteLocationRaw,
    }));
  });
  const subtitle = computed(() => {
    if (tokensTab.value === "tokens")
      return "Your profile, sign-in and personal tokens.";
    if (tokensTab.value === "audit-tokens")
      return "Your profile, sign-in and audit tokens.";
    return "Your profile and sign-in.";
  });

  // Keep the address on the tab that shows.
  watch(
    () => [route.name, activeTab.value] as const,
    ([name, tab]) => {
      if (name && TAB_FOR_ROUTE[String(name)] && TAB_ROUTES[tab] !== name)
        void router.replace({ name: TAB_ROUTES[tab] });
    },
    { immediate: true },
  );

  // Profile

  /** Null until loaded, or when your access can't be read. */
  const groups = ref<GroupRef[] | null>(null);
  /** Group pages open for people who can see groups. */
  const canOpenGroups = computed(() => auth.can("users.view", "groups.manage"));

  async function loadGroups(): Promise<void> {
    try {
      groups.value = (await fetchUserAccess("me")).groups;
    } catch {
      groups.value = null;
    }
  }

  onMounted(loadGroups);

  // Password

  const passwordOpen = ref(false);

  function openPassword(): void {
    currentPassword.value = "";
    newPassword.value = "";
    confirmPassword.value = "";
    fieldErrors.value = {};
    formError.value = null;
    passwordOpen.value = true;
  }

  function validate(): boolean {
    const errors: typeof fieldErrors.value = {};
    if (needsCurrentPassword.value && currentPassword.value === "") {
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
      // Setting a first password needs a recent sign-in; the shared dialog
      // asks for one and this runs again.
      await runWithReauth(() =>
        auth.changePassword(
          needsCurrentPassword.value ? currentPassword.value : "",
          newPassword.value,
        ),
      );
      router.push({
        name: "login",
        query: { notice: "password-changed" },
      });
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") return;
      mapApiError(error);
    } finally {
      submitting.value = false;
    }
  }

  return {
    activeTab,
    tabs,
    subtitle,
    groups,
    canOpenGroups,
    loadGroups,
    passwordOpen,
    openPassword,
    email,
    role,
    formattedRole,
    lastLoginAt,
    formattedLastLogin,
    fullLastLogin,
    recentAuthentication,
    needsCurrentPassword,
    currentPassword,
    newPassword,
    confirmPassword,
    fieldErrors,
    formError,
    submitting,
    submit,
  };
}
