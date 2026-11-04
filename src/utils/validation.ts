/** Shared input rules used by account, login, user, and secret forms. */
export const MIN_PASSWORD_LENGTH = 12;
export const MAX_PASSWORD_LENGTH = 128;
export const MAX_SECRET_KEY_LENGTH = 128;
export const MAX_SLUG_LENGTH = 63;

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const SECRET_KEY_PATTERN = /^[A-Za-z_][A-Za-z0-9_]*$/;
/** The server's slug rule (`app/src/utils/slug.rs`). */
const SLUG_PATTERN = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/;

export function isValidEmail(value: string): boolean {
  const email = value.trim();
  return email.length <= 254 && EMAIL_PATTERN.test(email);
}

export function validatePasswordLength(
  value: string,
):
  | { valid: true; code: undefined }
  | { valid: false; code: "PASSWORD_TOO_SHORT" | "PASSWORD_TOO_LONG" } {
  const length = Array.from(value).length;
  if (length < MIN_PASSWORD_LENGTH) {
    return { valid: false, code: "PASSWORD_TOO_SHORT" };
  }
  if (length > MAX_PASSWORD_LENGTH) {
    return { valid: false, code: "PASSWORD_TOO_LONG" };
  }
  return { valid: true, code: undefined };
}

export function isValidSecretKey(value: string): boolean {
  return (
    value.length > 0 &&
    value.length <= MAX_SECRET_KEY_LENGTH &&
    SECRET_KEY_PATTERN.test(value)
  );
}

/**
 * Project, environment and runner token names: lowercase letters, numbers
 * and single hyphens, starting with a letter, at most 63 characters.
 */
export function isValidSlug(value: string): boolean {
  return value.length <= MAX_SLUG_LENGTH && SLUG_PATTERN.test(value);
}

/**
 * Only same-origin, non-protocol-relative paths may be used as post-login
 * redirect targets. Blocks absolute URLs (`https://…`) and protocol-relative
 * shortcuts (`//host`). Backslashes and control characters are refused too,
 * because browsers read `/\evil.example` and `/<tab>/evil.example` as
 * `//evil.example`.
 */
export function isSafeRedirect(value: string): boolean {
  return (
    value.length <= 512 &&
    value.startsWith("/") &&
    !value.startsWith("//") &&
    !value.includes("://") &&
    !value.includes("\\") &&
    !/[\u0000-\u001f\u007f]/.test(value)
  );
}
