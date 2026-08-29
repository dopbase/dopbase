import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { bootstrapAdmin, fetchBootstrapStatus } from "~/services/bootstrap.api";
import * as authApi from "~/services/auth.api";
import type { SessionKind } from "~/services/auth.api";
import { onUnauthorized, registerCsrfProvider } from "~/services/http.client";

export interface AdminSession {
  adminId: string;
  email: string;
  sessionKind: SessionKind;
  /** True when the password was confirmed within the last ten minutes. */
  recentAuthentication: boolean;
}

export type BootstrapState = "unknown" | "setupRequired" | "ready";

const CSRF_STORAGE_KEY = "dopbase.csrf";

/**
 * CSRF persistence: `sessionStorage` only. It is not secret material — the
 * server stores its hash — and surviving a reload avoids dead sessions, but
 * it must never leak into localStorage or cookies.
 */
function restoreCsrf(): string | null {
  try {
    return sessionStorage.getItem(CSRF_STORAGE_KEY);
  } catch {
    return null;
  }
}

function persistCsrf(token: string | null): void {
  try {
    if (token) sessionStorage.setItem(CSRF_STORAGE_KEY, token);
    else sessionStorage.removeItem(CSRF_STORAGE_KEY);
  } catch {
    // Storage unavailable; keep the token in memory only.
  }
}

/**
 * Single-admin session state for the whole Admin UI.
 *
 * Owns the browser session object, the CSRF token backing the
 * `X-Dopbase-CSRF` header, and the public bootstrap state that decides
 * whether new visitors land on `/setup` or `/login`. Registering the CSRF
 * provider and the 401 listener here keeps `services/` store-free.
 */
export const useAuthStore = defineStore("auth", () => {
  const session = ref<AdminSession | null>(null);
  const bootstrapState = ref<BootstrapState>("unknown");
  const csrfToken = ref<string | null>(restoreCsrf());

  registerCsrfProvider(() => csrfToken.value);
  onUnauthorized(() => {
    session.value = null;
    persistCsrf(null);
    csrfToken.value = null;
  });

  const isAuthenticated = computed(() => session.value !== null);

  function setCsrf(token: string | null): void {
    csrfToken.value = token;
    persistCsrf(token);
  }

  /**
   * Resolves the public bootstrap status once. Network failures leave the
   * state `"unknown"` so guards can retry instead of hard-redirecting.
   */
  async function loadBootstrapStatus(): Promise<BootstrapState> {
    try {
      const status = await fetchBootstrapStatus();
      bootstrapState.value =
        status.state === "setupRequired" ? "setupRequired" : "ready";
    } catch {
      bootstrapState.value = "unknown";
    }
    return bootstrapState.value;
  }

  /** Claims an uninitialized server with the one-time setup token. */
  async function bootstrap(request: {
    setupToken: string;
    email: string;
    password: string;
  }): Promise<void> {
    const response = await bootstrapAdmin(request);
    session.value = {
      adminId: response.adminId,
      email: response.email,
      sessionKind: "browser",
      recentAuthentication: true,
    };
    setCsrf(response.csrfToken);
    bootstrapState.value = "ready";
  }

  /** Signs in with email + password and stores the browser session. */
  async function login(email: string, password: string): Promise<void> {
    const response = await authApi.login({
      email,
      password,
      sessionKind: "browser",
    });
    session.value = {
      adminId: response.adminId,
      email: response.email,
      sessionKind: response.sessionKind,
      recentAuthentication: true,
    };
    if (response.csrfToken) setCsrf(response.csrfToken);
  }

  /** Re-checks the cookie session; throws ApiError(401) when absent. */
  async function fetchSession(): Promise<void> {
    const response = await authApi.fetchSession();
    session.value = {
      adminId: response.adminId,
      email: response.email,
      sessionKind: response.sessionKind,
      recentAuthentication: response.recentAuthentication,
    };
  }

  /** Revokes the session server-side and clears all local state. */
  async function logout(): Promise<void> {
    try {
      await authApi.logout();
    } finally {
      session.value = null;
      setCsrf(null);
    }
  }

  /** Confirms the password, enabling reveal/export for ten minutes. */
  async function reauthenticate(password: string): Promise<void> {
    await authApi.reauthenticate(password);
    if (session.value) {
      session.value = { ...session.value, recentAuthentication: true };
    }
  }

  /**
   * Rotates the password. The server revokes every human session, so the
   * local session and CSRF token are cleared and the admin must sign in
   * again.
   */
  async function changePassword(
    currentPassword: string,
    newPassword: string,
  ): Promise<void> {
    await authApi.changePassword(currentPassword, newPassword);
    session.value = null;
    setCsrf(null);
  }

  return {
    session,
    bootstrapState,
    csrfToken,
    isAuthenticated,
    setCsrf,
    loadBootstrapStatus,
    bootstrap,
    login,
    fetchSession,
    logout,
    reauthenticate,
    changePassword,
  };
});
