import type { PersonalToken } from "~/services/personal-tokens.api";
import type { Role } from "~/services/roles.api";
import type { User } from "~/services/users.api";

/** How an account signs in, for the Sign-in column. */
export type SignInMethod = "password" | "sso" | "disabled" | "none";

/** AI agents sign in with tokens only and are listed on their own page. */
export function isAgent(user: User): boolean {
  return user.role === "ai_agent";
}

export function signInMethod(user: User): SignInMethod {
  if (user.status === "disabled") return "disabled";
  if (user.ssoLinked && !user.hasPassword) return "sso";
  if (user.hasPassword) return "password";
  return user.ssoLinked ? "sso" : "none";
}

export const SIGN_IN_LABELS: Record<SignInMethod, string> = {
  password: "password",
  sso: "sso",
  disabled: "disabled",
  none: "not set up",
};

/** The role row behind an account, when the roles list has it. */
export function roleOf(user: User, roles: readonly Role[]): Role | null {
  return roles.find((role) => role.id === user.roleId) ?? null;
}

export type TokenState = "active" | "expired" | "revoked";

export function tokenState(token: PersonalToken, now = Date.now()): TokenState {
  if (token.revokedAt) return "revoked";
  if (token.expiresAt && Date.parse(token.expiresAt) <= now) return "expired";
  return "active";
}
