import type { LoginResponse, SessionResponse } from "~/services/auth.api";

/**
 * A browser-session login response as returned by `auth.api.login`.
 *
 * Tests pass only the fields they assert on; everything else keeps a stable
 * default so the login payload is not copy-pasted across suites.
 */
export function browserSession(
  overrides: Partial<LoginResponse> = {},
): LoginResponse {
  return {
    adminId: "usr_1",
    email: "a@b.c",
    sessionKind: "browser",
    token: null,
    csrfToken: "csrf_1",
    ...overrides,
  };
}

/** A browser session as returned by `auth.api.fetchSession`. */
export function browserSessionInfo(): SessionResponse {
  return {
    adminId: "usr_1",
    email: "a@b.c",
    sessionKind: "browser",
    recentAuthentication: false,
  };
}
